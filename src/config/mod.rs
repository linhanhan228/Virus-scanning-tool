use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use num_cpus;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub scan_modes: ScanModesConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub update: UpdateConfig,
    pub monitor: MonitorConfig,
    pub report: ReportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanModesConfig {
    pub quick_scan_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub exclude_extensions: Vec<String>,
    pub max_file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub thread_pool_size: usize,
    pub cpu_usage_limit: f64,
    pub memory_limit_mb: u64,
    pub scan_buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub run_as_user: Option<String>,
    pub database_encryption: bool,
    pub audit_log_enabled: bool,
    pub quarantine_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub log_dir: PathBuf,
    pub max_size_mb: u64,
    pub max_files: usize,
    pub remote_logging: Option<RemoteLoggingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteLoggingConfig {
    pub endpoint: String,
    pub use_tls: bool,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub auto_download: bool,
    pub schedule: UpdateSchedule,
    pub mirror_url: String,
    pub verify_signatures: bool,
    pub database_path: PathBuf,
    pub backup_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSchedule {
    pub frequency: String,
    pub time: String,
    pub day_of_week: Option<u8>,
    pub check_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub enabled: bool,
    pub watch_paths: Vec<String>,
    pub events: Vec<String>,
    pub actions: MonitorActions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorActions {
    pub on_create: String,
    pub on_modify: String,
    pub on_delete: String,
    pub auto_quarantine: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub enabled: bool,
    pub format: String,
    pub output_dir: PathBuf,
    pub include_details: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_modes: ScanModesConfig {
                quick_scan_paths: vec![
                    "/bin".to_string(),
                    "/usr/bin".to_string(),
                    "/etc".to_string(),
                ],
                exclude_paths: vec![
                    "/proc".to_string(),
                    "/sys".to_string(),
                    "/dev".to_string(),
                    "/run".to_string(),
                    "/var/log".to_string(),
                ],
                exclude_extensions: vec![
                    "log".to_string(),
                    "txt".to_string(),
                    "tmp".to_string(),
                    "cache".to_string(),
                    "pid".to_string(),
                ],
                max_file_size: 50 * 1024 * 1024,
            },
            performance: PerformanceConfig {
                thread_pool_size: 1,
                cpu_usage_limit: 50.0,
                memory_limit_mb: 64,
                scan_buffer_size: 4096,
            },
            security: SecurityConfig {
                run_as_user: None,
                database_encryption: false,
                audit_log_enabled: false,
                quarantine_dir: PathBuf::from("/var/lib/virus-scanner/quarantine"),
            },
            logging: LoggingConfig {
                level: "WARN".to_string(),
                log_dir: PathBuf::from("/var/log/virus-scanner"),
                max_size_mb: 10,
                max_files: 3,
                remote_logging: None,
            },
            update: UpdateConfig {
                enabled: true,
                auto_download: true,
                schedule: UpdateSchedule {
                    frequency: "weekly".to_string(),
                    time: "03:00".to_string(),
                    day_of_week: Some(0),
                    check_interval_hours: 24,
                },
                mirror_url: "https://database.clamav.net".to_string(),
                verify_signatures: false,
                database_path: PathBuf::from("/var/lib/virus-scanner/database"),
                backup_path: PathBuf::from("/var/lib/virus-scanner/backup"),
            },
            monitor: MonitorConfig {
                enabled: false,
                watch_paths: vec!["/tmp".to_string()],
                events: vec!["create".to_string()],
                actions: MonitorActions {
                    on_create: "log".to_string(),
                    on_modify: "log".to_string(),
                    on_delete: "log".to_string(),
                    auto_quarantine: false,
                },
            },
            report: ReportConfig {
                enabled: true,
                format: "text".to_string(),
                output_dir: PathBuf::from("/var/lib/virus-scanner/reports"),
                include_details: false,
            },
        }
    }
}

impl ScannerConfig {
    pub fn load(path: &PathBuf) -> Result<Self, anyhow::Error> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn create_default_config_file() -> Result<PathBuf, anyhow::Error> {
        let config_path = dirs::config_dir()
            .unwrap_or(PathBuf::from("/etc"))
            .join("virus-scanner");

        std::fs::create_dir_all(&config_path)?;
        let config_file = config_path.join("config.yaml");
        let config = Self::default();
        config.save(&config_file)?;
        Ok(config_file)
    }
}
