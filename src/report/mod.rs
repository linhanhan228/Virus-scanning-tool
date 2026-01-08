use crate::scanner::{ScanResult, ThreatType, RiskLevel};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub id: String,
    pub timestamp: DateTime<Local>,
    pub scan_type: String,
    pub scan_paths: Vec<PathBuf>,
    pub summary: ReportSummary,
    pub threats: Vec<ThreatReport>,
    pub recommendations: Vec<String>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_files_scanned: u64,
    pub total_threats: u64,
    pub threats_by_type: HashMap<String, u64>,
    pub threats_by_risk: HashMap<String, u64>,
    pub scan_duration: u64,
    pub scan_speed_mb_s: f64,
    pub memory_peak_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatReport {
    pub id: String,
    pub file_path: PathBuf,
    pub threat_type: String,
    pub risk_level: String,
    pub signature_id: String,
    pub detection_name: String,
    pub file_info: FileReportInfo,
    pub action_taken: Option<String>,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReportInfo {
    pub size: u64,
    pub permissions: String,
    pub created: Option<u64>,
    pub modified: Option<u64>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub scanner_version: String,
    pub database_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: String,
    pub category: String,
    pub description: String,
    pub action: String,
    pub affected_items: Vec<PathBuf>,
}

pub struct ReportGenerator {
    output_dir: PathBuf,
    include_system_info: bool,
    include_file_hashes: bool,
}

impl ReportGenerator {
    pub fn new(output_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&output_dir).ok();
        Self {
            output_dir,
            include_system_info: true,
            include_file_hashes: false,
        }
    }

    pub fn generate(
        &self,
        results: &[ScanResult],
        scan_type: &str,
        scan_paths: &[PathBuf],
        duration: Instant,
        memory_peak: f64,
        database_version: String,
    ) -> Result<ScanReport, anyhow::Error> {
        let threats_by_type = Self::count_threats_by_type(results);
        let threats_by_risk = Self::count_threats_by_risk(results);
        let scan_speed = Self::calculate_scan_speed(results, duration);

        let recommendations = self.generate_recommendations(results);

        let threat_reports: Vec<ThreatReport> = results
            .iter()
            .enumerate()
            .map(|(i, result)| ThreatReport {
                id: format!("THR{:08}", i + 1),
                file_path: result.file_path.clone(),
                threat_type: format!("{:?}", result.threat_type),
                risk_level: format!("{:?}", result.risk_level),
                signature_id: result.signature_id.clone(),
                detection_name: self.get_detection_name(&result.signature_id),
                file_info: FileReportInfo {
                    size: result.file_info.size,
                    permissions: result.file_info.permissions.clone(),
                    created: result.file_info.created,
                    modified: result.file_info.modified,
                    md5: None,
                    sha256: None,
                },
                action_taken: None,
                timestamp: Local::now(),
            })
            .collect();

        let system_info = if self.include_system_info {
            self.get_system_info(database_version)
        } else {
            SystemInfo {
                os_name: String::new(),
                os_version: String::new(),
                kernel_version: String::new(),
                architecture: String::new(),
                scanner_version: String::new(),
                database_version,
            }
        };

        let report = ScanReport {
            id: self.generate_report_id(),
            timestamp: Local::now(),
            scan_type: scan_type.to_string(),
            scan_paths: scan_paths.to_vec(),
            summary: ReportSummary {
                total_files_scanned: results.len() as u64,
                total_threats: results.len() as u64,
                threats_by_type,
                threats_by_risk,
                scan_duration: duration.elapsed().as_secs(),
                scan_speed_mb_s: scan_speed,
                memory_peak_mb: memory_peak,
            },
            threats: threat_reports,
            recommendations,
            system_info,
        };

        Ok(report)
    }

    pub fn save(&self, report: &ScanReport, format: ReportFormat) -> Result<PathBuf, anyhow::Error> {
        let filename = format!("report_{}.{}", report.timestamp.format("%Y%m%d_%H%M%S"), format.extension());
        let filepath = self.output_dir.join(&filename);

        match format {
            ReportFormat::Json => {
                let json = serde_json::to_string_pretty(report)?;
                std::fs::write(&filepath, json)?;
            }
            ReportFormat::Yaml => {
                let yaml = serde_yaml::to_string(report)?;
                std::fs::write(&filepath, yaml)?;
            }
            ReportFormat::Html => {
                let html = self.render_html(report);
                std::fs::write(&filepath, html)?;
            }
            ReportFormat::Text => {
                let text = self.render_text(report);
                std::fs::write(&filepath, text)?;
            }
        }

        log::info!("报告已保存: {:?}", filepath);
        Ok(filepath)
    }

    fn render_html(&self, report: &ScanReport) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>病毒扫描报告 - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #2c3e50; color: white; padding: 20px; }}
        .summary {{ background: #ecf0f1; padding: 15px; margin: 10px 0; }}
        .threat {{ border: 1px solid #e74c3c; padding: 10px; margin: 5px 0; }}
        .high {{ background: #ffebee; }}
        .critical {{ background: #ffcdd2; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>病毒扫描报告</h1>
        <p>扫描时间: {}</p>
        <p>扫描类型: {}</p>
    </div>
    <div class="summary">
        <h2>扫描摘要</h2>
        <p>扫描文件数: {}</p>
        <p>发现威胁: {}</p>
        <p>扫描时长: {}秒</p>
    </div>
</body>
</html>"#,
            report.id,
            report.timestamp,
            report.scan_type,
            report.summary.total_files_scanned,
            report.summary.total_threats,
            report.summary.scan_duration
        )
    }

    fn render_text(&self, report: &ScanReport) -> String {
        let mut text = format!(
            r#"病毒扫描报告
===============
扫描ID: {}
扫描时间: {}
扫描类型: {}

扫描摘要
--------
扫描文件数: {}
发现威胁: {}
扫描时长: {}秒
扫描速度: {:.2} MB/s

威胁列表
--------
"#,
            report.id,
            report.timestamp,
            report.scan_type,
            report.summary.total_files_scanned,
            report.summary.total_threats,
            report.summary.scan_duration,
            report.summary.scan_speed_mb_s
        );

        for threat in &report.threats {
            text.push_str(&format!(
                "- 文件: {:?}\n  类型: {}\n  风险等级: {}\n  签名ID: {}\n\n",
                threat.file_path,
                threat.threat_type,
                threat.risk_level,
                threat.signature_id
            ));
        }

        text.push_str("\n处理建议\n--------\n");
        for rec in &report.recommendations {
            text.push_str(&format!("- {}\n", rec));
        }

        text
    }

    fn count_threats_by_type(results: &[ScanResult]) -> HashMap<String, u64> {
        let mut counts = HashMap::new();
        for result in results {
            let threat_type = format!("{:?}", result.threat_type);
            *counts.entry(threat_type).or_insert(0) += 1;
        }
        counts
    }

    fn count_threats_by_risk(results: &[ScanResult]) -> HashMap<String, u64> {
        let mut counts = HashMap::new();
        for result in results {
            let risk_level = format!("{:?}", result.risk_level);
            *counts.entry(risk_level).or_insert(0) += 1;
        }
        counts
    }

    fn calculate_scan_speed(results: &[ScanResult], duration: Instant) -> f64 {
        let total_bytes: u64 = results.iter().map(|r| r.file_info.size).sum();
        let duration_secs = duration.elapsed().as_secs_f64();
        if duration_secs > 0.0 {
            total_bytes as f64 / duration_secs / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }

    fn generate_report_id(&self) -> String {
        format!("RPT{:08}", rand::random::<u32>())
    }

    fn get_detection_name(&self, signature_id: &str) -> String {
        format!("Malware.{}", signature_id)
    }

    fn get_system_info(&self, database_version: String) -> SystemInfo {
        let uname = nix::sys::utsname::uname().unwrap();
        SystemInfo {
            os_name: "Linux".to_string(),
            os_version: uname.release().to_string_lossy().into_owned(),
            kernel_version: uname.version().to_string_lossy().into_owned(),
            architecture: uname.machine().to_string_lossy().into_owned(),
            scanner_version: env!("CARGO_PKG_VERSION").to_string(),
            database_version,
        }
    }

    fn generate_recommendations(&self, results: &[ScanResult]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let critical_count = results.iter().filter(|r| r.risk_level == RiskLevel::Critical).count();
        if critical_count > 0 {
            recommendations.push(format!(
                "发现 {} 个高危威胁，请立即隔离并清除受影响文件",
                critical_count
            ));
        }

        let virus_count = results.iter().filter(|r| r.threat_type == ThreatType::Virus).count();
        if virus_count > 0 {
            recommendations.push(format!(
                "发现 {} 个病毒，请使用最新病毒库进行全盘扫描",
                virus_count
            ));
        }

        recommendations.push("建议定期更新病毒库以确保检测能力".to_string());
        recommendations.push("建议启用实时文件监控功能".to_string());

        recommendations
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReportFormat {
    Json,
    Yaml,
    Html,
    Text,
}

impl ReportFormat {
    pub fn extension(&self) -> &str {
        match self {
            ReportFormat::Json => "json",
            ReportFormat::Yaml => "yaml",
            ReportFormat::Html => "html",
            ReportFormat::Text => "txt",
        }
    }
}
