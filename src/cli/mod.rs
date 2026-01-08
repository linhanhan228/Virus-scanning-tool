use crate::config::ScannerConfig;
use crate::scanner::{ScannerEngine, ScanOptions, ScanMode, SignatureDatabase};
use crate::update::{DatabaseUpdater, UpdateScheduler};
use crate::report::{ReportGenerator, ReportFormat};
use crate::monitor::FileMonitor;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "virus-scanner")]
#[command(author = "Security Team")]
#[command(version = "1.0.0")]
#[command(about = "企业级病毒查杀工具", long_about = None)]
pub struct Command {
    #[command(subcommand)]
    pub subcommand: SubCommands,
    #[arg(short, long, global = true, help = "指定配置文件路径")]
    pub config: Option<PathBuf>,
    #[arg(short, long, global = true, help = "显示详细输出")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum SubCommands {
    #[command(name = "scan", about = "执行病毒扫描")]
    Scan(ScanArgs),
    #[command(name = "update", about = "更新病毒库")]
    Update(UpdateArgs),
    #[command(name = "monitor", about = "文件监控")]
    Monitor(MonitorArgs),
    #[command(name = "report", about = "生成扫描报告")]
    Report(ReportArgs),
    #[command(name = "status", about = "查看系统状态")]
    Status(StatusArgs),
}

#[derive(Args)]
pub struct ScanArgs {
    #[arg(long, short = 't', help = "扫描类型: quick(快速), full(全盘), custom(自定义)")]
    pub scan_type: Option<String>,
    #[arg(long, short = 'p', help = "指定扫描路径")]
    pub paths: Vec<PathBuf>,
    #[arg(long, short = 'e', help = "排除路径")]
    pub exclude: Vec<PathBuf>,
    #[arg(long, help = "线程数")]
    pub threads: Option<usize>,
    #[arg(long, help = "生成扫描报告")]
    pub report: bool,
    #[arg(long, short = 'f', help = "报告格式: json, yaml, html, text")]
    pub format: Option<String>,
}

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(long, short = 'f', help = "强制更新")]
    pub force: bool,
    #[arg(long, help = "启用定时更新")]
    pub schedule: bool,
    #[arg(long, help = "仅检查更新")]
    pub check_only: bool,
}

#[derive(Args)]
pub struct MonitorArgs {
    #[arg(long, short = 's', help = "启动监控")]
    pub start: bool,
    #[arg(long, short = 'p', help = "停止监控")]
    pub stop: bool,
    #[arg(long, help = "监控路径")]
    pub watch: Vec<PathBuf>,
}

#[derive(Args)]
pub struct ReportArgs {
    #[arg(long, short = 'i', help = "输入报告文件")]
    pub input: PathBuf,
    #[arg(long, short = 'f', help = "报告格式: json, yaml, html, text")]
    pub format: String,
    #[arg(long, short = 'o', help = "输出报告文件")]
    pub output: PathBuf,
}

#[derive(Args)]
pub struct StatusArgs {
    #[arg(long, short = 'd', help = "显示病毒库信息")]
    pub database: bool,
    #[arg(long, short = 's', help = "显示系统信息")]
    pub system: bool,
}

impl Command {
    pub fn build() -> Self {
        Command::parse()
    }

    pub async fn execute(matches: &Command) -> Result<()> {
        let config_path = matches.config.clone()
            .unwrap_or_else(|| PathBuf::from("/etc/virus-scanner/config.yaml"));

        let config = ScannerConfig::load(&config_path)
            .with_context(|| format!("无法加载配置文件: {:?}", config_path))?;

        let signature_db = Arc::new(SignatureDatabase::new());

        match &matches.subcommand {
            SubCommands::Scan(args) => Self::handle_scan(args, &config, &signature_db).await,
            SubCommands::Update(args) => Self::handle_update(args, &config).await,
            SubCommands::Monitor(args) => Self::handle_monitor(args, &config).await,
            SubCommands::Report(args) => Self::handle_report(args, &config).await,
            SubCommands::Status(args) => Self::handle_status(args, &config, &signature_db).await,
        }
    }

    async fn handle_scan(
        args: &ScanArgs,
        config: &ScannerConfig,
        signature_db: &Arc<SignatureDatabase>,
    ) -> Result<()> {
        println!("开始病毒扫描...");

        let scan_mode = match args.scan_type.as_ref().map(|s| s.as_str()) {
            Some("quick") | Some("fast") => ScanMode::Quick,
            Some("full") => ScanMode::Full,
            Some("custom") | None => ScanMode::Custom,
            _ => return Err(anyhow::anyhow!("无效的扫描类型")),
        };

        let paths = if args.paths.is_empty() {
            match scan_mode {
                ScanMode::Quick => config.scan_modes.quick_scan_paths.iter()
                    .map(|p| PathBuf::from(p))
                    .collect(),
                ScanMode::Full => vec![PathBuf::from("/")],
                ScanMode::Custom => vec![PathBuf::from(".")],
            }
        } else {
            args.paths.clone()
        };

        let scan_options = ScanOptions {
            scan_mode,
            custom_paths: paths.clone(),
            exclude_paths: if args.exclude.is_empty() {
                config.scan_modes.exclude_paths.iter()
                    .map(|p| PathBuf::from(p))
                    .collect()
            } else {
                args.exclude.clone()
            },
            exclude_extensions: config.scan_modes.exclude_extensions.clone(),
            max_file_size: config.scan_modes.max_file_size,
            thread_count: args.threads.unwrap_or(config.performance.thread_pool_size),
            quick_scan_paths: config.scan_modes.quick_scan_paths.iter()
                .map(|p| PathBuf::from(p))
                .collect(),
        };

        let engine = ScannerEngine::new(Arc::clone(signature_db), scan_options);
        let start_time = Instant::now();

        let results = engine.start_scan().await?;

        let duration = start_time.elapsed();
        let stats = engine.get_stats();

        println!("\n扫描完成!");
        println!("扫描文件数: {}", stats.get_files_scanned());
        println!("发现威胁数: {}", stats.get_threats_found());
        println!("扫描耗时: {:.2}秒", duration.as_secs_f64());
        println!("扫描速度: {:.2} MB/s", stats.get_speed_mb_per_s());

        if args.report {
            let report_generator = ReportGenerator::new(config.report.output_dir.clone());
            let report = report_generator.generate(
                &results,
                &format!("{:?}", scan_mode),
                &paths,
                start_time,
                0.0,
                signature_db.get_version(),
            )?;

            let format = match args.format.as_ref().map(|s| s.as_str()) {
                Some("json") => ReportFormat::Json,
                Some("yaml") => ReportFormat::Yaml,
                Some("html") => ReportFormat::Html,
                Some("text") | None => ReportFormat::Text,
                _ => ReportFormat::Text,
            };

            let report_path = report_generator.save(&report, format)?;
            println!("报告已保存: {:?}", report_path);
        }

        Ok(())
    }

    async fn handle_update(args: &UpdateArgs, config: &ScannerConfig) -> Result<()> {
        let database_path = PathBuf::from("/var/lib/virus-scanner/database");
        let backup_path = PathBuf::from("/var/lib/virus-scanner/backups");

        std::fs::create_dir_all(&database_path)?;
        std::fs::create_dir_all(&backup_path)?;

        let updater = Arc::new(DatabaseUpdater::new(
            config.update.mirror_url.clone(),
            database_path.clone(),
            backup_path,
        ));

        println!("病毒库更新工具");
        println!("镜像服务器: {}", config.update.mirror_url);
        println!("本地数据库路径: {:?}", database_path);
        println!();

        if args.check_only {
            println!("正在检查病毒库更新...");
            if let Some(version) = updater.check_for_updates().await? {
                println!("发现新版本: {}", version);
                println!("请运行 'virus-scanner update --force' 进行更新");
            } else {
                println!("当前已是最新版本");
            }
            return Ok(());
        }

        if args.force || args.schedule {
            println!("开始更新病毒库...");
            println!("正在下载 ClamAV 病毒库文件:");
            println!("  - main.cvd (主病毒库)");
            println!("  - daily.cvd (每日更新)");
            println!("  - bytecode.cvd (字节码库)");
            println!();

            match updater.perform_update().await {
                Ok(update_info) => {
                    println!("病毒库更新完成!");
                    println!();
                    println!("更新详情:");
                    println!("  版本: {}", update_info.version);
                    println!("  更新时间: {}", update_info.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("  下载大小: {:.2} MB", update_info.download_size as f64 / 1024.0 / 1024.0);
                    println!("  新增签名: {}", update_info.signatures_added);
                    println!("  删除签名: {}", update_info.signatures_removed);
                    println!("  总签名数: {}", update_info.total_signatures);
                    println!();
                    println!("病毒库文件已更新到: {:?}", database_path);
                }
                Err(e) => {
                    println!("病毒库更新失败: {}", e);
                    println!();
                    println!("可能的原因:");
                    println!("  1. 网络连接问题");
                    println!("  2. 镜像服务器不可用");
                    println!("  3. 磁盘空间不足");
                    println!("  4. 权限不足");
                    println!();
                    println!("建议:");
                    println!("  - 检查网络连接");
                    println!("  - 尝试使用其他镜像服务器");
                    println!("  - 检查磁盘空间");
                    println!("  - 确保有足够的权限");
                    return Err(e);
                }
            }
        }

        if args.schedule {
            let schedule = crate::update::UpdateSchedule {
                enabled: true,
                frequency: config.update.schedule.frequency.clone(),
                time: config.update.schedule.time.clone(),
                day_of_week: config.update.schedule.day_of_week,
            };
            let scheduler = UpdateScheduler::new(Arc::clone(&updater), schedule);
            scheduler.start().await;
            println!("定时更新已启用");
            println!("更新频率: {}", config.update.schedule.frequency);
            println!("更新时间: {}", config.update.schedule.time);
        }

        Ok(())
    }

    async fn handle_monitor(args: &MonitorArgs, config: &ScannerConfig) -> Result<()> {
        let mut monitor = FileMonitor::new();

        if args.start {
            monitor.add_default_watches()?;
            monitor.start()?;
            println!("文件监控已启动");
            println!("监控路径: {:?}", config.monitor.watch_paths);

            tokio::signal::ctrl_c().await?;
            monitor.stop();
            println!("监控已停止");
        } else if args.stop {
            monitor.stop();
            println!("文件监控已停止");
        } else {
            println!("用法: virus-scanner monitor --start|--stop");
        }

        Ok(())
    }

    async fn handle_report(args: &ReportArgs, config: &ScannerConfig) -> Result<()> {
        let report_generator = ReportGenerator::new(config.report.output_dir.clone());

        match std::fs::read_to_string(&args.input) {
            Ok(content) => {
                let report: crate::report::ScanReport = match args.format.as_str() {
                    "json" => serde_json::from_str(&content)?,
                    "yaml" => serde_yaml::from_str(&content)?,
                    _ => return Err(anyhow::anyhow!("不支持的格式")),
                };

                let output_path = if args.output.as_os_str().is_empty() {
                    report_generator.save(&report, ReportFormat::Text)?
                } else {
                    args.output.clone()
                };

                println!("报告已保存: {:?}", output_path);
            }
            Err(e) => return Err(anyhow::anyhow!("无法读取报告文件: {}", e)),
        }

        Ok(())
    }

    async fn handle_status(
        args: &StatusArgs,
        config: &ScannerConfig,
        signature_db: &Arc<SignatureDatabase>,
    ) -> Result<()> {
        println!("病毒查杀工具状态");
        println!("================");

        if args.database || args.system {
            println!("\n病毒库信息:");
            println!("  签名数量: {}", signature_db.get_signature_count());
            println!("  内存占用: {:.2} MB", signature_db.get_memory_usage() as f64 / 1024.0 / 1024.0);
            println!("  最后更新: {:?}", signature_db.get_last_update());
            println!("  病毒库版本: {}", signature_db.get_version());
        }

        if args.system {
            println!("\n系统信息:");
            println!("  线程数: {}", config.performance.thread_pool_size);
            println!("  CPU限制: {}%", config.performance.cpu_usage_limit);
            println!("  内存限制: {} MB", config.performance.memory_limit_mb);
        }

        Ok(())
    }
}
