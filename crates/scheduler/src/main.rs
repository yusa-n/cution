use anyhow::Result;
use dotenv;
use scheduler::DailyScheduler;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use std::env;

async fn run_daily_crawlers() -> Result<()> {
    info!("Starting daily crawlers execution");
    
    // This will run the orchestrator which includes all crawlers
    // including the new OpenRouter and MCP rankings crawlers
    let result = std::process::Command::new("cargo")
        .args(&["run", "--bin", "orchestrator"])
        .current_dir(env::current_dir()?.parent().unwrap_or(&env::current_dir()?))
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                info!("Daily crawlers completed successfully");
                info!("Output: {}", String::from_utf8_lossy(&output.stdout));
            } else {
                anyhow::bail!("Daily crawlers failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to execute daily crawlers: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    let _ = dotenv::dotenv();

    // Configure tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting daily scheduler for OpenRouter and MCP rankings");

    let mut scheduler = DailyScheduler::new().await?;

    // Schedule daily execution at 9:00 AM UTC
    // You can modify this time by changing the hour parameter
    scheduler.add_daily_job(9, 0, || async {
        run_daily_crawlers().await
    }).await?;

    info!("Scheduler configured to run daily at 09:00 UTC");
    info!("Press Ctrl+C to stop the scheduler");

    // Handle graceful shutdown
    let scheduler_clone = scheduler;
    tokio::select! {
        _ = scheduler_clone.run_forever() => {
            info!("Scheduler stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received interrupt signal, shutting down...");
            scheduler_clone.shutdown().await?;
        }
    }

    Ok(())
}