pub mod models;

pub use McpRankingsCrawler;
use models::McpServer;
use common::{Config, Crawler, CrawlerResult, SupabaseStorageClient};
use time::OffsetDateTime;
use tracing::info;
use async_trait::async_trait;
use scraper::{Html, Selector};

pub struct McpRankingsCrawler {
    storage_client: SupabaseStorageClient,
    client: reqwest::Client,
}

impl McpRankingsCrawler {
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

    async fn fetch_rankings(&self) -> CrawlerResult<Vec<McpServer>> {
        let url = "https://mcp.so";
        
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| common::CrawlerError::Api(format!("Failed to fetch MCP rankings: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| common::CrawlerError::Api(format!("Failed to read response: {}", e)))?;

        self.parse_rankings(&html)
    }

    fn parse_rankings(&self, html: &str) -> CrawlerResult<Vec<McpServer>> {
        let document = Html::parse_document(html);
        let mut servers = Vec::new();

        // This is a placeholder implementation - the actual selectors would need to be 
        // determined by examining the actual MCP.so page structure
        let row_selector = Selector::parse("tr, .server-row, .mcp-row, .ranking-item")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid selector: {}", e)))?;
        
        let name_selector = Selector::parse(".server-name, .name, h3, h4, .title")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid name selector: {}", e)))?;
        
        let description_selector = Selector::parse(".description, .desc, p")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid description selector: {}", e)))?;

        let stars_selector = Selector::parse(".stars, .star-count, .github-stars")
            .map_err(|e| common::CrawlerError::Parse(format!("Invalid stars selector: {}", e)))?;

        for (index, row) in document.select(&row_selector).enumerate() {
            if let Some(name_elem) = row.select(&name_selector).next() {
                let name = name_elem.text().collect::<String>().trim().to_string();
                if !name.is_empty() {
                    let description = row.select(&description_selector)
                        .next()
                        .map(|elem| elem.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();

                    let stars = row.select(&stars_selector)
                        .next()
                        .and_then(|elem| {
                            elem.text().collect::<String>()
                                .chars()
                                .filter(|c| c.is_ascii_digit())
                                .collect::<String>()
                                .parse::<u32>()
                                .ok()
                        })
                        .unwrap_or(0);

                    servers.push(McpServer {
                        rank: index + 1,
                        name,
                        description,
                        stars,
                        fetched_at: OffsetDateTime::now_utc(),
                    });
                }
            }
        }

        info!("Parsed {} MCP servers from MCP.so", servers.len());
        Ok(servers)
    }

    async fn process_rankings(&self) -> CrawlerResult<()> {
        let servers = self.fetch_rankings().await?;
        
        if servers.is_empty() {
            info!("No MCP servers found");
            return Ok(());
        }

        let today_str = OffsetDateTime::now_utc().date().to_string();
        let file_content = self.format_servers_markdown(&servers);
        let file_path = format!("{}/mcp-rankings.md", today_str);

        self.storage_client
            .upload_file(&file_path, file_content, "text/markdown")
            .await
            .map_err(|e| common::CrawlerError::StorageUpload(e.to_string()))?;

        info!("Successfully uploaded {} MCP servers to {}", servers.len(), file_path);
        Ok(())
    }

    fn format_servers_markdown(&self, servers: &[McpServer]) -> String {
        let mut content = String::new();
        content.push_str("# MCP Server Rankings\n\n");
        content.push_str(&format!("*Fetched on {}*\n\n", OffsetDateTime::now_utc().date()));
        
        content.push_str("| Rank | Server Name | Description | Stars |\n");
        content.push_str("|------|-------------|-------------|-------|\n");
        
        for server in servers {
            content.push_str(&format!("| {} | {} | {} | {} |\n", 
                server.rank, server.name, server.description, server.stars));
        }
        
        content
    }
}

#[async_trait]
impl Crawler for McpRankingsCrawler {
    async fn run(&self) -> CrawlerResult<()> {
        info!("MCP Rankings Crawler starting up");
        self.process_rankings().await
    }

    fn name(&self) -> &'static str {
        "MCP Rankings"
    }
}