use async_trait::async_trait;
use crate::error::CrawlerResult;

#[async_trait]
pub trait Crawler: Send + Sync {
    async fn run(&self) -> CrawlerResult<()>;
    fn name(&self) -> &'static str;
}

#[async_trait]
pub trait DataSource: Send + Sync {
    type Item;
    
    async fn fetch_data(&self) -> CrawlerResult<Vec<Self::Item>>;
    fn format_output(&self, items: &[Self::Item]) -> String;
}

pub struct CrawlerManager {
    crawlers: Vec<Box<dyn Crawler>>,
}

impl CrawlerManager {
    pub fn new() -> Self {
        Self {
            crawlers: Vec::new(),
        }
    }

    pub fn add_crawler(mut self, crawler: Box<dyn Crawler>) -> Self {
        self.crawlers.push(crawler);
        self
    }

    pub async fn run_all(&self) -> CrawlerResult<()> {
        use tokio::task::JoinSet;
        use tracing::{info, warn};

        let mut tasks = JoinSet::new();
        
        for crawler in &self.crawlers {
            let name = crawler.name();
            tasks.spawn(async move {
                match crawler.run().await {
                    Ok(_) => {
                        info!("{} completed successfully", name);
                        Ok(())
                    }
                    Err(e) => {
                        warn!("{} failed: {}", name, e);
                        Err(e)
                    }
                }
            });
        }

        let mut success_count = 0;
        let mut error_count = 0;

        while let Some(result) = tasks.join_next().await {
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
            return Err(crate::error::CrawlerError::Api(
                format!("Some crawlers failed: {} failed, {} succeeded", error_count, success_count)
            ));
        }

        Ok(())
    }
}