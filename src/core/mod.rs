use crate::api::ApiServer;
use crate::config::ScannerConfig;
use crate::monitor::FileMonitor;
use crate::report::ReportGenerator;
use crate::scanner::{ScannerEngine, ScanOptions, ScanMode, SignatureDatabase};
use crate::update::{DatabaseUpdater, UpdateScheduler};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal;
use tokio::sync::RwLock;

pub struct VirusScanner {
    config: Arc<RwLock<ScannerConfig>>,
    signature_db: Arc<SignatureDatabase>,
    scanner_engine: Option<ScannerEngine>,
    monitor: Option<FileMonitor>,
    updater: Option<Arc<DatabaseUpdater>>,
    api_server: Option<ApiServer>,
}

impl VirusScanner {
    pub fn new(config: ScannerConfig) -> Self {
        let config = Arc::new(RwLock::new(config));
        let signature_db = Arc::new(SignatureDatabase::new());

        Self {
            config,
            signature_db,
            scanner_engine: None,
            monitor: None,
            updater: None,
            api_server: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), anyhow::Error> {
        log::info!("正在初始化病毒查杀工具...");

        let config = self.config.read().await;

        std::fs::create_dir_all(&config.security.quarantine_dir)?;
        std::fs::create_dir_all(&config.logging.log_dir)?;
        std::fs::create_dir_all(&config.report.output_dir)?;

        let database_path = PathBuf::from("/var/lib/virus-scanner/database");
        let backup_path = PathBuf::from("/var/lib/virus-scanner/backups");

        std::fs::create_dir_all(&database_path)?;
        std::fs::create_dir_all(&backup_path)?;

        if let Err(e) = self.signature_db.load_from_directory(&database_path).await {
            log::warn!("无法加载本地病毒库: {}，将使用空数据库", e);
        }

        log::info!(
            "病毒库已加载，签名数量: {}",
            self.signature_db.get_signature_count()
        );

        drop(config);

        let updater = Arc::new(DatabaseUpdater::new(
            self.config.read().await.update.mirror_url.clone(),
            database_path,
            backup_path,
        ));
        self.updater = Some(updater);

        Ok(())
    }

    pub async fn run_quick_scan(&mut self) -> Result<Vec<crate::scanner::ScanResult>, anyhow::Error> {
        let config = self.config.read().await;

        let scan_options = ScanOptions {
            scan_mode: ScanMode::Quick,
            custom_paths: config.scan_modes.quick_scan_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
            exclude_paths: config.scan_modes.exclude_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
            exclude_extensions: config.scan_modes.exclude_extensions.clone(),
            max_file_size: config.scan_modes.max_file_size,
            thread_count: config.performance.thread_pool_size,
            quick_scan_paths: config.scan_modes.quick_scan_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
        };

        drop(config);

        self.scanner_engine = Some(ScannerEngine::new(Arc::clone(&self.signature_db), scan_options));

        if let Some(engine) = &self.scanner_engine {
            engine.start_scan().await
        } else {
            Err(anyhow::anyhow!("扫描引擎未初始化"))
        }
    }

    pub async fn run_full_scan(&mut self) -> Result<Vec<crate::scanner::ScanResult>, anyhow::Error> {
        let config = self.config.read().await;

        let scan_options = ScanOptions {
            scan_mode: ScanMode::Full,
            custom_paths: vec![PathBuf::from("/")],
            exclude_paths: config.scan_modes.exclude_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
            exclude_extensions: config.scan_modes.exclude_extensions.clone(),
            max_file_size: config.scan_modes.max_file_size,
            thread_count: config.performance.thread_pool_size,
            quick_scan_paths: vec![],
        };

        drop(config);

        self.scanner_engine = Some(ScannerEngine::new(Arc::clone(&self.signature_db), scan_options));

        if let Some(engine) = &self.scanner_engine {
            engine.start_scan().await
        } else {
            Err(anyhow::anyhow!("扫描引擎未初始化"))
        }
    }

    pub async fn run_custom_scan(
        &mut self,
        paths: Vec<PathBuf>,
    ) -> Result<Vec<crate::scanner::ScanResult>, anyhow::Error> {
        let config = self.config.read().await;

        let scan_options = ScanOptions {
            scan_mode: ScanMode::Custom,
            custom_paths: paths,
            exclude_paths: config.scan_modes.exclude_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
            exclude_extensions: config.scan_modes.exclude_extensions.clone(),
            max_file_size: config.scan_modes.max_file_size,
            thread_count: config.performance.thread_pool_size,
            quick_scan_paths: vec![],
        };

        drop(config);

        self.scanner_engine = Some(ScannerEngine::new(Arc::clone(&self.signature_db), scan_options));

        if let Some(engine) = &self.scanner_engine {
            engine.start_scan().await
        } else {
            Err(anyhow::anyhow!("扫描引擎未初始化"))
        }
    }

    pub async fn update_database(&self, force: bool) -> Result<(), anyhow::Error> {
        if let Some(ref updater) = self.updater {
            if force {
                updater.perform_update().await?;
            } else {
                if let Some(version) = updater.check_for_updates().await? {
                    log::info!("发现新版本: {}，开始更新...", version);
                    updater.perform_update().await?;
                }
            }
        }
        Ok(())
    }

    pub fn start_file_monitor(&mut self) -> Result<(), anyhow::Error> {
        let mut monitor = FileMonitor::new();
        monitor.add_default_watches()?;
        monitor.start()?;
        self.monitor = Some(monitor);
        log::info!("文件监控已启动");
        Ok(())
    }

    pub fn stop_file_monitor(&mut self) {
        if let Some(ref mut monitor) = self.monitor {
            monitor.stop();
            log::info!("文件监控已停止");
        }
    }

    pub fn start_api_server(&mut self, addr: &str, api_key: &str) -> Result<(), anyhow::Error> {
        let addr: std::net::SocketAddr = addr.parse()?;
        self.api_server = Some(ApiServer::new(addr, api_key.to_string()));
        log::info!("API服务器将在后台启动...");
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        log::info!("病毒查杀工具启动完成");

        tokio::select! {
            _ = signal::ctrl_c() => {
                log::info!("收到终止信号，正在关闭...");
            }
        }

        self.shutdown().await
    }

    pub async fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        log::info!("正在关闭病毒查杀工具...");

        self.stop_file_monitor();

        log::info!("病毒查杀工具已关闭");
        Ok(())
    }

    pub fn get_signature_count(&self) -> usize {
        self.signature_db.get_signature_count()
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.signature_db.get_memory_usage()
    }

    pub fn get_status(&self) -> ScannerStatus {
        ScannerStatus {
            running: true,
            signature_count: self.signature_db.get_signature_count(),
            memory_usage_bytes: self.signature_db.get_memory_usage(),
            last_scan: None,
            database_version: self.signature_db.get_version(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannerStatus {
    pub running: bool,
    pub signature_count: usize,
    pub memory_usage_bytes: u64,
    pub last_scan: Option<Instant>,
    pub database_version: String,
}

impl Default for VirusScanner {
    fn default() -> Self {
        Self::new(ScannerConfig::default())
    }
}
