use crate::scanner::SignatureDatabase;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub scan_mode: ScanMode,
    pub custom_paths: Vec<PathBuf>,
    pub exclude_paths: Vec<PathBuf>,
    pub exclude_extensions: Vec<String>,
    pub max_file_size: u64,
    pub thread_count: usize,
    pub quick_scan_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanMode {
    Quick,
    Full,
    Custom,
}

#[derive(Debug)]
pub struct ScanResult {
    pub file_path: PathBuf,
    pub threat_type: ThreatType,
    pub risk_level: RiskLevel,
    pub signature_id: String,
    pub file_info: FileInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThreatType {
    Virus,
    Trojan,
    Worm,
    Ransomware,
    Rootkit,
    Adware,
    Spyware,
    HackTool,
    PUA,
    Unknown,
}

impl From<&str> for ThreatType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "virus" => ThreatType::Virus,
            "trojan" => ThreatType::Trojan,
            "worm" => ThreatType::Worm,
            "ransomware" => ThreatType::Ransomware,
            "rootkit" => ThreatType::Rootkit,
            "adware" => ThreatType::Adware,
            "spyware" => ThreatType::Spyware,
            "hacktool" => ThreatType::HackTool,
            "pua" => ThreatType::PUA,
            _ => ThreatType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl From<&str> for RiskLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => RiskLevel::Critical,
            "high" => RiskLevel::High,
            "medium" => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub size: u64,
    pub permissions: String,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub accessed: Option<u64>,
}

pub struct ScanStats {
    pub start_time: Instant,
    pub files_scanned: AtomicUsize,
    pub threats_found: AtomicUsize,
    pub bytes_scanned: AtomicUsize,
    pub errors: AtomicUsize,
}

impl ScanStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            files_scanned: AtomicUsize::new(0),
            threats_found: AtomicUsize::new(0),
            bytes_scanned: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
        }
    }

    pub fn get_files_scanned(&self) -> usize {
        self.files_scanned.load(Ordering::Relaxed)
    }

    pub fn get_threats_found(&self) -> usize {
        self.threats_found.load(Ordering::Relaxed)
    }

    pub fn get_bytes_scanned(&self) -> usize {
        self.bytes_scanned.load(Ordering::Relaxed)
    }

    pub fn get_speed_mb_per_s(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs() == 0 {
            return 0.0;
        }
        let bytes = self.bytes_scanned.load(Ordering::Relaxed) as f64;
        bytes / elapsed.as_secs_f64() / (1024.0 * 1024.0)
    }
}

pub struct ScannerEngine {
    signature_db: Arc<SignatureDatabase>,
    options: ScanOptions,
    stats: Arc<ScanStats>,
    progress_callback: Option<Arc<dyn Fn(f64) + Send + Sync>>,
}

impl ScannerEngine {
    pub fn new(signature_db: Arc<SignatureDatabase>, options: ScanOptions) -> Self {
        Self {
            signature_db,
            options,
            stats: Arc::new(ScanStats::new()),
            progress_callback: None,
        }
    }

    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(f64) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
    }

    pub async fn start_scan(&self) -> Result<Vec<ScanResult>, anyhow::Error> {
        log::info!("开始扫描，模式: {:?}", self.options.scan_mode);

        let paths = self.get_scan_paths()?;
        let stats = Arc::clone(&self.stats);
        let signature_db = Arc::clone(&self.signature_db);
        let options = self.options.clone();
        let max_file_size = options.max_file_size;

        let mut results = Vec::new();

        for root_path in &paths {
            let iter = walkdir::WalkDir::new(root_path)
                .follow_links(false)
                .same_file_system(true)
                .into_iter();

            for entry in iter {
                match entry {
                    Ok(entry) => {
                        let path = entry.path().to_path_buf();
                        if !self.should_exclude(&path) && entry.file_type().is_file() {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                if metadata.len() <= max_file_size {
                                    stats.files_scanned.fetch_add(1, Ordering::Relaxed);
                                    stats.bytes_scanned.fetch_add(metadata.len() as usize, Ordering::Relaxed);

                                    if let Some(threat) = signature_db.scan_file_sync(&path) {
                                        stats.threats_found.fetch_add(1, Ordering::Relaxed);
                                        results.push(ScanResult {
                                            file_path: path.clone(),
                                            threat_type: threat.threat_type.as_str().into(),
                                            risk_level: threat.risk_level.as_str().into(),
                                            signature_id: threat.id,
                                            file_info: FileInfo {
                                                size: metadata.len(),
                                                permissions: String::new(),
                                                created: None,
                                                modified: None,
                                                accessed: None,
                                            },
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("访问路径错误: {}", e);
                        stats.errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(results)
    }

    fn get_scan_paths(&self) -> Result<Vec<PathBuf>, anyhow::Error> {
        match self.options.scan_mode {
            ScanMode::Quick => Ok(self.options.quick_scan_paths.clone()),
            ScanMode::Full => {
                let mut paths = Vec::new();
                for entry in std::fs::read_dir("/")? {
                    let path = entry?.path();
                    if self.should_exclude(&path) {
                        continue;
                    }
                    paths.push(path);
                }
                Ok(paths)
            }
            ScanMode::Custom => Ok(self.options.custom_paths.clone()),
        }
    }

    fn should_exclude(&self, path: &PathBuf) -> bool {
        self.options.exclude_paths.iter().any(|p| path.starts_with(p))
            || path.extension().and_then(|e| e.to_str()).map(|e| {
                self.options.exclude_extensions.contains(&e.to_string())
            }).unwrap_or(false)
    }

    fn get_permissions(path: &PathBuf) -> String {
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut perms = String::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                perms.push(if mode & 0o100 != 0 { 'x' } else { '-' });
                perms.push(if mode & 0o200 != 0 { 'w' } else { '-' });
                perms.push(if mode & 0o400 != 0 { 'r' } else { '-' });
            }
            perms
        } else {
            String::from("???")
        }
    }

    fn get_created_time(path: &PathBuf) -> Option<u64> {
        std::fs::metadata(path).ok()?.created().ok()?.elapsed().ok().map(|d| d.as_secs())
    }

    fn get_modified_time(path: &PathBuf) -> Option<u64> {
        std::fs::metadata(path).ok()?.modified().ok()?.elapsed().ok().map(|d| d.as_secs())
    }

    fn get_accessed_time(path: &PathBuf) -> Option<u64> {
        std::fs::metadata(path).ok()?.accessed().ok()?.elapsed().ok().map(|d| d.as_secs())
    }

    pub fn get_stats(&self) -> &Arc<ScanStats> {
        &self.stats
    }
}

struct ThreatInfo {
    threat_type: ThreatType,
    risk_level: RiskLevel,
    signature_id: String,
}
