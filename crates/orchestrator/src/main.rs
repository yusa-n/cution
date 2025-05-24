use anyhow::Result;
use common::{Config, CrawlerManager, Crawler};
use dotenv;
use std::env;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    let _ = dotenv::dotenv();

    // Configure tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let config = Config::from_env()?;

    // Create crawler manager
    let mut manager = CrawlerManager::new();

    // Add GitHub crawler if LANGUAGES is set
    if !config.languages.is_empty() {
        if let Ok(github_crawler) = github::GithubTrendingFetcher::new(&config) {
            manager = manager.add_crawler(Box::new(github_crawler));
        } else {
            info!("Skipping GitHub crawler: LANGUAGES not properly set");
        }
    } else {
        info!("Skipping GitHub crawler: LANGUAGES not set");
    }

    // Add Hacker News crawler if GEMINI_API_KEY is set
    if config.gemini_api_key.is_some() {
        if let Ok(hn_crawler) = hacker_news::HackerNewsCrawler::new(&config) {
            manager = manager.add_crawler(Box::new(hn_crawler));
        } else {
            info!("Failed to create Hacker News crawler");
        }
    } else {
        info!("Skipping Hacker News crawler: GEMINI_API_KEY not set");
    }

    // Add xAI search crawler if XAI_API_KEY is set
    if config.xai_api_key.is_some() {
        info!("xAI search crawler would be added here (implementation pending)");
    } else {
        info!("Skipping xAI search crawler: XAI_API_KEY not set");
    }

    // Add Custom Site crawler if CUSTOM_SITE_URL is set
    if config.custom_site_url.is_some() {
        info!("Custom Site crawler would be added here (implementation pending)");
    } else {
        info!("Skipping Custom Site crawler: CUSTOM_SITE_URL not set");
    }

    // Add OpenRouter crawler - always enabled
    if let Ok(openrouter_crawler) = openrouter::OpenRouterCrawler::new(&config) {
        manager = manager.add_crawler(Box::new(openrouter_crawler));
    } else {
        info!("Failed to create OpenRouter crawler");
    }

    // Add MCP Rankings crawler - always enabled
    if let Ok(mcp_crawler) = mcp_rankings::McpRankingsCrawler::new(&config) {
        manager = manager.add_crawler(Box::new(mcp_crawler));
    } else {
        info!("Failed to create MCP Rankings crawler");
    }

    // Run all crawlers
    manager.run_all().await.map_err(|e| anyhow::anyhow!(e))?;

    info!("All crawlers completed successfully");
    Ok(())
}
