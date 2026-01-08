use crate::config::ScannerConfig;
use crate::scanner::{ScannerEngine, ScanOptions, ScanMode, SignatureDatabase};
use crate::report::{ReportGenerator, ReportFormat};
use crate::update::DatabaseUpdater;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_config_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    let config = ScannerConfig::default();
    config.save(&config_path).unwrap();

    let loaded_config = ScannerConfig::load(&config_path).unwrap();

    assert_eq!(loaded_config.scan_modes.quick_scan_paths.len(), 5);
    assert_eq!(loaded_config.performance.thread_pool_size, num_cpus::get());
    assert!(loaded_config.logging.enabled);
}

#[tokio::test]
async fn test_scan_options_creation() {
    let options = ScanOptions {
        scan_mode: ScanMode::Quick,
        custom_paths: vec![PathBuf::from("/tmp")],
        exclude_paths: vec![PathBuf::from("/proc")],
        exclude_extensions: vec!["log".to_string()],
        max_file_size: 1024 * 1024,
        thread_count: 4,
        quick_scan_paths: vec![PathBuf::from("/tmp")],
    };

    assert_eq!(options.scan_mode, ScanMode::Quick);
    assert_eq!(options.thread_count, 4);
    assert_eq!(options.max_file_size, 1024 * 1024);
}

#[tokio::test]
async fn test_report_generation() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReportGenerator::new(temp_dir.path().to_path_buf());

    let report = generator.generate(
        &[],
        "quick",
        &[PathBuf::from("/test")],
        std::time::Instant::now(),
        50.0,
        "1.0.0".to_string(),
    ).unwrap();

    assert!(!report.id.is_empty());
    assert_eq!(report.scan_type, "quick");
    assert_eq!(report.summary.total_files_scanned, 0);
    assert!(report.summary.scan_duration >= 0);
}

#[tokio::test]
async fn test_report_save_json() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReportGenerator::new(temp_dir.path().to_path_buf());

    let report = generator.generate(
        &[],
        "full",
        &[PathBuf::from("/")],
        std::time::Instant::now(),
        100.0,
        "2.0.0".to_string(),
    ).unwrap();

    let save_path = generator.save(&report, ReportFormat::Json).unwrap();
    assert!(save_path.exists());

    let content = std::fs::read_to_string(&save_path).unwrap();
    assert!(content.contains("\"scan_type\""));
    assert!(content.contains("\"full\""));
}

#[tokio::test]
async fn test_report_save_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReportGenerator::new(temp_dir.path().to_path_buf());

    let report = generator.generate(
        &[],
        "custom",
        &[PathBuf::from("/home")],
        std::time::Instant::now(),
        75.0,
        "3.0.0".to_string(),
    ).unwrap();

    let save_path = generator.save(&report, ReportFormat::Yaml).unwrap();
    assert!(save_path.exists());

    let content = std::fs::read_to_string(&save_path).unwrap();
    assert!(content.contains("scan_type:"));
    assert!(content.contains("custom"));
}

#[tokio::test]
async fn test_signature_database_memory_usage() {
    let db = SignatureDatabase::new();
    assert_eq!(db.get_memory_usage(), 0);
}

#[tokio::test]
async fn test_signature_database_version() {
    let db = SignatureDatabase::new();
    db.set_version("1.0.0".to_string());
    assert_eq!(db.get_version(), "1.0.0");
}

#[tokio::test]
async fn test_signature_database_last_update() {
    let db = SignatureDatabase::new();
    assert!(db.get_last_update().is_none());
}

#[tokio::test]
async fn test_database_updater_status() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = temp_dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();

    let updater = DatabaseUpdater::new(
        "https://database.clamav.net".to_string(),
        temp_dir.path().join("database"),
        backup_dir,
    );

    let status = updater.get_status();
    assert!(!status.in_progress);
    assert!(status.current_version.is_empty() || status.current_version == "0.0.0");
}

#[tokio::test]
async fn test_database_updater_history() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = temp_dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).unwrap();

    let updater = DatabaseUpdater::new(
        "https://database.clamav.net".to_string(),
        temp_dir.path().join("database"),
        backup_dir,
    );

    let history = updater.get_update_history();
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_scanner_engine_creation() {
    let signature_db = Arc::new(SignatureDatabase::new());
    let options = ScanOptions {
        scan_mode: ScanMode::Full,
        custom_paths: vec![],
        exclude_paths: vec![PathBuf::from("/proc")],
        exclude_extensions: vec![],
        max_file_size: 10 * 1024 * 1024,
        thread_count: 4,
        quick_scan_paths: vec![],
    };

    let engine = ScannerEngine::new(signature_db, options);
    let stats = engine.get_stats();

    assert_eq!(stats.get_files_scanned(), 0);
    assert_eq!(stats.get_threats_found(), 0);
}

#[tokio::test]
async fn test_scan_stats_operations() {
    let stats = crate::scanner::engine::ScanStats::new();

    assert_eq!(stats.get_files_scanned(), 0);
    assert_eq!(stats.get_threats_found(), 0);
    assert_eq!(stats.get_bytes_scanned(), 0);
    assert!(stats.get_speed_mb_per_s() >= 0.0);
}

#[tokio::test]
async fn test_config_default_values() {
    let config = ScannerConfig::default();

    assert!(config.scan_modes.quick_scan_paths.contains(&"/bin".to_string()));
    assert!(config.scan_modes.quick_scan_paths.contains(&"/etc".to_string()));
    assert_eq!(config.performance.cpu_usage_limit, 70.0);
    assert_eq!(config.performance.memory_limit_mb, 200);
    assert_eq!(config.logging.level, "INFO");
    assert!(config.update.enabled);
}

#[tokio::test]
async fn test_config_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");

    let mut config = ScannerConfig::default();
    config.performance.thread_pool_size = 4;
    config.logging.level = "DEBUG".to_string();

    config.save(&config_path).unwrap();

    let loaded_config = ScannerConfig::load(&config_path).unwrap();
    assert_eq!(loaded_config.performance.thread_pool_size, 4);
    assert_eq!(loaded_config.logging.level, "DEBUG");
}

#[tokio::test]
async fn test_empty_file_scan() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    std::fs::write(&test_file, "This is a test file").unwrap();

    let signature_db = Arc::new(SignatureDatabase::new());
    let options = ScanOptions {
        scan_mode: ScanMode::Custom,
        custom_paths: vec![temp_dir.path().to_path_buf()],
        exclude_paths: vec![],
        exclude_extensions: vec![],
        max_file_size: 1024 * 1024,
        thread_count: 1,
        quick_scan_paths: vec![],
    };

    let engine = ScannerEngine::new(signature_db, options);
    let results = engine.start_scan().await.unwrap();

    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_exclude_directory() {
    let temp_dir = TempDir::new().unwrap();

    let excluded_dir = temp_dir.path().join("excluded");
    std::fs::create_dir_all(&excluded_dir).unwrap();

    let test_file = excluded_dir.join("malware.exe");
    std::fs::write(&test_file, "fake malware").unwrap();

    let signature_db = Arc::new(SignatureDatabase::new());
    let options = ScanOptions {
        scan_mode: ScanMode::Custom,
        custom_paths: vec![temp_dir.path().to_path_buf()],
        exclude_paths: vec![excluded_dir],
        exclude_extensions: vec![],
        max_file_size: 1024 * 1024,
        thread_count: 1,
        quick_scan_paths: vec![],
    };

    let engine = ScannerEngine::new(signature_db, options);
    let results = engine.start_scan().await.unwrap();

    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_exclude_file_extensions() {
    let temp_dir = TempDir::new().unwrap();

    let test_file = temp_dir.path().join("test.log");
    std::fs::write(&test_file, "log content").unwrap();

    let signature_db = Arc::new(SignatureDatabase::new());
    let options = ScanOptions {
        scan_mode: ScanMode::Custom,
        custom_paths: vec![temp_dir.path().to_path_buf()],
        exclude_paths: vec![],
        exclude_extensions: vec!["log".to_string()],
        max_file_size: 1024 * 1024,
        thread_count: 1,
    };

    let engine = ScannerEngine::new(signature_db, options);
    let results = engine.start_scan().await.unwrap();

    assert_eq!(results.len(), 0);
}
