pub mod models;

pub use OpenRouterCrawler;
use models::ModelRanking;
use common::{Config, Crawler, CrawlerResult, SupabaseStorageClient};
use time::OffsetDateTime;
use tracing::info;
use async_trait::async_trait;
use scraper::{Html, Selector};

pub struct OpenRouterCrawler {
    storage_client: SupabaseStorageClient,
    client: reqwest::Client,
}

impl OpenRouterCrawler {
    pub fn new(config: &Config) -> CrawlerResult<Self> {
        let storage_client = SupabaseStorageClient::new(
            &config.supabase.storage_url,
            &config.supabase.key,
            &config.supabase.bucket,
        );

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| common::CrawlerError::Api(e.to_string()))?;

        Ok(Self {
            storage_client,
            client,
        })
    }

    async fn fetch_rankings(&self) -> CrawlerResult<Vec<ModelRanking>> {
        let url = "https://openrouter.ai/rankings";
        
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| common::CrawlerError::Api(format!("Failed to fetch OpenRouter rankings: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| common::CrawlerError::Api(format!("Failed to read response: {}", e)))?;

        self.parse_rankings(&html)
    }

    fn parse_rankings(&self, html: &str) -> CrawlerResult<Vec<ModelRanking>> {
        let document = Html::parse_document(html);
        let mut rankings = Vec::new();

        // This is a placeholder implementation - the actual selectors would need to be 
        // determined by examining the actual OpenRouter rankings page structure
        let row_selector = Selector::parse("tr, .ranking-row, .model-row")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid selector: {}", e)))?;
        
        let name_selector = Selector::parse(".model-name, .name, h3, h4")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid name selector: {}", e)))?;
        
        let score_selector = Selector::parse(".score, .rating, .points")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid score selector: {}", e)))?;

        for (index, row) in document.select(&row_selector).enumerate() {
            if let Some(name_elem) = row.select(&name_selector).next() {
                let name = name_elem.text().collect::<String>().trim().to_string();
                if !name.is_empty() {
                    let score = row.select(&score_selector)
                        .next()
                        .and_then(|elem| elem.text().collect::<String>().trim().parse::<f64>().ok())
                        .unwrap_or(0.0);

                    rankings.push(ModelRanking {
                        rank: index + 1,
                        name,
                        score,
                        fetched_at: OffsetDateTime::now_utc(),
                    });
                }
            }
        }

        info!("Parsed {} model rankings from OpenRouter", rankings.len());
        Ok(rankings)
    }

    async fn process_rankings(&self) -> CrawlerResult<()> {
        let rankings = self.fetch_rankings().await?;
        
        if rankings.is_empty() {
            info!("No OpenRouter rankings found");
            return Ok(());
        }

        let today_str = OffsetDateTime::now_utc().date().to_string();
        let file_content = self.format_rankings_markdown(&rankings);
        let file_path = format!("{}/openrouter-rankings.md", today_str);

        self.storage_client
            .upload_file(&file_path, file_content, "text/markdown")
            .await
            .map_err(|e| common::CrawlerError::StorageUpload(e.to_string()))?;

        info!("Successfully uploaded {} OpenRouter rankings to {}", rankings.len(), file_path);
        Ok(())
    }

    fn format_rankings_markdown(&self, rankings: &[ModelRanking]) -> String {
        let mut content = String::new();
        content.push_str("# OpenRouter Model Rankings\n\n");
        content.push_str(&format!("*Fetched on {}*\n\n", OffsetDateTime::now_utc().date()));
        
        content.push_str("| Rank | Model Name | Score |\n");
        content.push_str("|------|------------|-------|\n");
        
        for ranking in rankings {
            content.push_str(&format!("| {} | {} | {:.2} |\n", 
                ranking.rank, ranking.name, ranking.score));
        }
        
        content
    }
}

#[async_trait]
impl Crawler for OpenRouterCrawler {
    async fn run(&self) -> CrawlerResult<()> {
        info!("OpenRouter Crawler starting up");
        self.process_rankings().await
    }

    fn name(&self) -> &'static str {
        "OpenRouter"
    }
}