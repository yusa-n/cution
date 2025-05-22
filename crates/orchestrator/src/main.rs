use anyhow::Result;
use tokio::task::JoinSet;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use dotenv;

// Using github, hacker_news and xai_search crates
use github;
use hacker_news;
use xai_search;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    let _ = dotenv::dotenv();

    // Configure tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // JoinSet for running both crawlers in parallel
    let mut crawler_tasks = JoinSet::new();

    // Register GitHub crawler as an async task
    crawler_tasks.spawn(async {
        match github::run_github_crawler().await {
            Ok(_) => {
                info!("GitHub completed successfully");
                Ok::<_, anyhow::Error>(())
            }
            Err(e) => {
                eprintln!("GitHub failed: {}", e);
                Err(e)
            }
        }
    });

    // Register Hacker News crawler as an async task
    crawler_tasks.spawn(async {
        match hacker_news::run_hacker_news_crawler().await {
            Ok(_) => {
                info!("Hacker News completed successfully");
                Ok::<_, anyhow::Error>(())
            }
            Err(e) => {
                eprintln!("Hacker News failed: {}", e);
                Err(e)
            }
        }
    });

    // Register xAI search crawler as an async task
    crawler_tasks.spawn(async {
        match xai_search::run_xai_search().await {
            Ok(_) => {
                info!("xAI search completed successfully");
                Ok::<_, anyhow::Error>(())
            }
            Err(e) => {
                eprintln!("xAI search failed: {}", e);
                Err(e)
            }
        }
    });

    // Wait for all crawler tasks to complete
    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = crawler_tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) | Err(_) => error_count += 1,
        }
    }

    info!(
        "All crawlers finished. Successful: {}, Failed: {}",
        success_count, error_count
    );

    if error_count > 0 {
        anyhow::bail!("Some crawlers failed to complete successfully");
    }

    Ok(())
} 
