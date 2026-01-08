use virus_scanner::cli::Command;
use anyhow::Result;
use std::process;

#[tokio::main]
async fn main() -> Result<()> {
    let command = Command::build();

    match Command::execute(&command).await {
        Ok(_) => {
            log::info!("程序执行完成");
            Ok(())
        }
        Err(e) => {
            log::error!("执行错误: {}", e);
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}
