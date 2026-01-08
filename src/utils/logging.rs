use anyhow::Context;
use fern::Dispatch;
use log::{Level, LevelFilter};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;

pub struct Logger;

impl Logger {
    pub fn init(
        log_dir: PathBuf,
        level: LevelFilter,
        max_size_mb: u64,
        max_files: usize,
    ) -> Result<(), anyhow::Error> {
        std::fs::create_dir_all(&log_dir)?;

        let log_file = log_dir.join(format!(
            "virus-scanner_{}.log",
            Local::now().format("%Y%m%d")
        ));

        let dispatcher = Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{}][{}][{}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(level)
            .chain(
                fern::log_file(&log_file)
                    .context(format!("无法创建日志文件: {:?}", log_file))?,
            )
            .chain(std::io::stdout());

        dispatcher.apply()?;

        log::info!("日志系统已初始化，输出目录: {:?}", log_file);

        Ok(())
    }

    pub fn get_level_filter(level: &str) -> LevelFilter {
        match level.to_uppercase().as_str() {
            "DEBUG" => LevelFilter::Debug,
            "INFO" => LevelFilter::Info,
            "WARN" => LevelFilter::Warn,
            "ERROR" => LevelFilter::Error,
            "TRACE" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }
}

pub struct AuditLogger {
    log_path: PathBuf,
    enabled: bool,
}

impl AuditLogger {
    pub fn new(log_path: PathBuf, enabled: bool) -> Self {
        if enabled {
            std::fs::create_dir_all(&log_path).ok();
        }
        Self { log_path, enabled }
    }

    pub fn log(&self, action: &str, user: &str, details: &str) {
        if !self.enabled {
            return;
        }

        let timestamp = chrono::Local::now().to_rfc3339();
        let log_entry = format!(
            "[{}] ACTION={} USER={} DETAILS={}\n",
            timestamp, action, user, details
        );

        let log_file = self.log_path.join("audit.log");

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_entry.as_bytes());
        }
    }
}
