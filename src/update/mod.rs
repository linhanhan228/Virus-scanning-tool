use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub signatures_added: u32,
    pub signatures_removed: u32,
    pub total_signatures: u32,
    pub download_size: u64,
}

#[derive(Debug, Clone)]
pub struct UpdateStatus {
    pub in_progress: bool,
    pub last_update: Option<Instant>,
    pub next_update: Option<Instant>,
    pub current_version: String,
    pub latest_version: String,
    pub error: Option<String>,
}

pub struct DatabaseUpdater {
    mirror_url: String,
    local_database_path: PathBuf,
    backup_path: PathBuf,
    status: Arc<Mutex<UpdateStatus>>,
    update_history: Arc<Mutex<Vec<UpdateInfo>>>,
    last_check: Arc<Mutex<Option<Instant>>>,
    event_tx: Option<mpsc::Sender<UpdateEvent>>,
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    Started,
    Progress(u64, u64),
    Completed(UpdateInfo),
    Failed(String),
    VersionAvailable(String),
}

impl DatabaseUpdater {
    pub fn new(
        mirror_url: String,
        local_database_path: PathBuf,
        backup_path: PathBuf,
    ) -> Self {
        Self {
            mirror_url,
            local_database_path,
            backup_path,
            status: Arc::new(Mutex::new(UpdateStatus {
                in_progress: false,
                last_update: None,
                next_update: None,
                current_version: String::from("0.0.0"),
                latest_version: String::from("0.0.0"),
                error: None,
            })),
            update_history: Arc::new(Mutex::new(Vec::new())),
            last_check: Arc::new(Mutex::new(None)),
            event_tx: None,
        }
    }

    pub fn set_event_tx(&mut self, tx: mpsc::Sender<UpdateEvent>) {
        self.event_tx = Some(tx);
    }

    pub async fn check_for_updates(&self) -> Result<Option<String>, anyhow::Error> {
        log::info!("正在检查病毒库更新...");

        *self.last_check.lock().unwrap() = Some(Instant::now());

        let client = reqwest::Client::new();

        let version_url = format!("{}/version.txt", self.mirror_url);
        let response = client
            .get(&version_url)
            .send()
            .await
            .context("无法连接到病毒库服务器")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("服务器返回错误: {}", response.status()));
        }

        let version = response
            .text()
            .await
            .context("无法读取版本信息")?;

        let version = version.trim().to_string();

        let mut status = self.status.lock().unwrap();
        let old_version = status.latest_version.clone();
        status.latest_version = version.clone();

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(UpdateEvent::VersionAvailable(version.clone())).await;
        }

        log::info!("当前版本: {}, 最新版本: {}", old_version, version);

        if old_version != version {
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    pub async fn perform_update(&self) -> Result<UpdateInfo, anyhow::Error> {
        {
            let mut status = self.status.lock().unwrap();

            if status.in_progress {
                return Err(anyhow::anyhow!("更新已在进行中"));
            }

            status.in_progress = true;
            status.error = None;
        }

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(UpdateEvent::Started).await;
        }

        log::info!("开始下载病毒库更新...");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        let main_url = format!("{}/main.cvd", self.mirror_url);
        let daily_url = format!("{}/daily.cvd", self.mirror_url);
        let bytecode_url = format!("{}/bytecode.cvd", self.mirror_url);

        let temp_dir = tempfile::tempdir_in(&self.local_database_path)
            .context("无法创建临时目录")?;

        let mut signatures_added = 0u32;
        let mut signatures_removed = 0u32;
        let mut total_signatures = 0u32;
        let mut download_size = 0u64;

        let database_files = vec![
            ("main.cvd", &main_url),
            ("daily.cvd", &daily_url),
            ("bytecode.cvd", &bytecode_url),
        ];

        for (name, url) in &database_files {
            log::info!("正在下载 {}...", name);

            let response = client
                .get(*url)
                .send()
                .await
                .with_context(|| format!("无法下载 {}", name))?;

            if !response.status().is_success() {
                log::warn!("无法下载 {}，服务器返回: {}", name, response.status());
                continue;
            }

            let size = response
                .content_length()
                .unwrap_or(0);
            download_size += size;

            let file_path = temp_dir.path().join(name);
            let mut file = File::create(&file_path)
                .await
                .with_context(|| format!("无法创建文件: {:?}", file_path))?;

            let bytes = response.bytes().await.context("下载失败")?;
            file.write_all(&bytes)
                .await
                .context("写入文件失败")?;
            let downloaded = bytes.len() as u64;

            file.flush().await.context("刷新文件失败")?;

            log::info!("{} 下载完成 ({:.2} MB)", name, downloaded as f64 / 1024.0 / 1024.0);
        }

        let new_version = self.get_latest_version().await?;

        let update_info = UpdateInfo {
            version: new_version.clone(),
            timestamp: Utc::now(),
            signatures_added,
            signatures_removed,
            total_signatures,
            download_size,
        };

        self.backup_current_database()?;
        self.install_new_database(temp_dir.path())?;

        {
            let mut status = self.status.lock().unwrap();
            status.in_progress = false;
            status.last_update = Some(Instant::now());
            status.current_version = new_version.clone();
            status.error = None;
        }

        self.update_history.lock().unwrap().push(update_info.clone());

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(UpdateEvent::Completed(update_info.clone())).await;
        }

        log::info!("病毒库更新完成，版本: {}", new_version);

        Ok(update_info)
    }

    fn backup_current_database(&self) -> Result<(), anyhow::Error> {
        log::info!("正在备份当前病毒库...");

        if let Err(e) = std::fs::create_dir_all(&self.backup_path) {
            log::warn!("无法创建备份目录: {}", e);
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_file = self.backup_path.join(format!("backup_{}.tar.gz", timestamp));

        let mut cmd = std::process::Command::new("tar");
        cmd.arg("-czf")
            .arg(&backup_file)
            .arg("-C")
            .arg(self.local_database_path.parent().unwrap_or(Path::new(".")))
            .arg(self.local_database_path.file_name().unwrap_or(std::ffi::OsStr::new("cvd")));

        let output = cmd.output().context("备份失败")?;

        if !output.status.success() {
            log::warn!("备份失败: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            log::info!("备份已创建: {:?}", backup_file);
        }

        Ok(())
    }

    fn install_new_database(&self, temp_dir: &Path) -> Result<(), anyhow::Error> {
        log::info!("正在安装新病毒库...");

        for file in &["main.cvd", "daily.cvd", "bytecode.cvd"] {
            let src = temp_dir.join(file);
            let dst = self.local_database_path.join(file);

            if src.exists() {
                std::fs::copy(&src, &dst)
                    .with_context(|| format!("无法安装 {}", file))?;
                log::info!("已安装: {:?}", dst);
            }
        }

        Ok(())
    }

    async fn get_latest_version(&self) -> Result<String, anyhow::Error> {
        let status = self.status.lock().unwrap();
        Ok(status.latest_version.clone())
    }

    pub fn get_status(&self) -> UpdateStatus {
        let status = self.status.lock().unwrap().clone();
        status
    }

    pub fn get_update_history(&self) -> Vec<UpdateInfo> {
        self.update_history.lock().unwrap().clone()
    }

    pub async fn rollback(&self, version: &str) -> Result<(), anyhow::Error> {
        log::info!("正在回滚到版本: {}", version);

        let backup_file = self
            .backup_path
            .join(format!("backup_{}.tar.gz", version));

        if !backup_file.exists() {
            return Err(anyhow::anyhow!("备份文件不存在: {:?}", backup_file));
        }

        let mut cmd = std::process::Command::new("tar");
        cmd.arg("-xzf")
            .arg(&backup_file)
            .arg("-C")
            .arg(self.local_database_path.parent().unwrap_or(Path::new(".")));

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("回滚失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        log::info!("已成功回滚到版本: {}", version);

        Ok(())
    }
}

pub struct UpdateScheduler {
    updater: Arc<DatabaseUpdater>,
    schedule: UpdateSchedule,
    running: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct UpdateSchedule {
    pub enabled: bool,
    pub frequency: String,
    pub time: String,
    pub day_of_week: Option<u8>,
}

impl UpdateScheduler {
    pub fn new(updater: Arc<DatabaseUpdater>, schedule: UpdateSchedule) -> Self {
        Self {
            updater,
            schedule,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) {
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        log::info!("更新调度器已启动");

        let running = Arc::clone(&self.running);
        let updater = self.updater.clone();
        let schedule = self.schedule.clone();

        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let should_update = {
                    let now = chrono::Local::now();
                    if schedule.frequency == "daily" {
                        let update_time: Vec<&str> = schedule.time.split(':').collect();
                        if update_time.len() >= 2 {
                            let hour: u32 = update_time[0].parse().unwrap_or(3);
                            let minute: u32 = update_time[1].parse().unwrap_or(0);
                            let now_hour: u32 = now.format("%H").to_string().parse().unwrap_or(0);
                            let now_minute: u32 = now.format("%M").to_string().parse().unwrap_or(0);
                            now_hour == hour && now_minute == minute
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_update {
                    if let Err(e) = updater.perform_update().await {
                        log::error!("自动更新失败: {}", e);
                    }
                }

                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        log::info!("更新调度器已停止");
    }

    async fn should_update(&self) -> bool {
        if !self.schedule.enabled {
            return false;
        }

        let now = chrono::Local::now();

        if self.schedule.frequency == "daily" {
            let update_time: Vec<&str> = self.schedule.time.split(':').collect();
            if update_time.len() >= 2 {
                let hour: u32 = update_time[0].parse().unwrap_or(3);
                let minute: u32 = update_time[1].parse().unwrap_or(0);

                let now_hour: u32 = now.format("%H").to_string().parse().unwrap_or(0);
                let now_minute: u32 = now.format("%M").to_string().parse().unwrap_or(0);
                if now_hour == hour && now_minute == minute {
                    return true;
                }
            }
        }

        false
    }
}
