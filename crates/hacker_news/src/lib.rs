pub mod api;
pub mod models;

pub use self::HackerNewsCrawler;
use api::HackerNewsAPI;
use models::StoryData;
use common::{Config, Crawler, CrawlerResult, SupabaseStorageClient};
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::info;
use async_trait::async_trait;

pub struct HackerNewsCrawler {
    api: HackerNewsAPI,
    storage_client: SupabaseStorageClient,
    gemini_api_key: String,
    config: common::HackerNewsConfig,
}

impl HackerNewsCrawler {
    pub fn new(config: &Config) -> CrawlerResult<Self> {
        let gemini_api_key = config.require_gemini_api_key()?.clone();
        let storage_client = SupabaseStorageClient::new(
            &config.supabase.storage_url,
            &config.supabase.key,
            &config.supabase.bucket,
        );

        Ok(Self {
            api: HackerNewsAPI::new(),
            storage_client,
            gemini_api_key,
            config: config.hacker_news.clone(),
        })
    }

    async fn process_stories(&self) -> CrawlerResult<()> {
        let story_ids = self.api.get_top_stories(self.config.max_stories).await
            .map_err(|e| common::CrawlerError::Api(e.to_string()))?;
        info!("Fetched {} top story IDs", story_ids.len());

        let mut all_stories_markdown: Vec<String> = Vec::new();
        let mut processed_count = 0;

        let mut tasks = JoinSet::new();

        for story_id in story_ids {
            let api = self.api.clone();
            let gemini_api_key = self.gemini_api_key.clone();
            let min_score_threshold = self.config.min_score_threshold;
            let min_html_length = self.config.min_html_length;
            let max_html_length = self.config.max_html_length;
            tasks.spawn(async move {
                match api.get_story(story_id).await {
                    Ok(item) => {
                        if item.score < min_score_threshold {
                            return None;
                        }

                        let summary = match &item.text {
                            Some(html) if (min_html_length..max_html_length).contains(&html.len()) => {
                                info!("Summarizing story: {}", item.title);
                                let clean_text = api.clean_html(html);
                                match api
                                    .summarize(&gemini_api_key, &item.title, &clean_text)
                                    .await
                                {
                                    Ok(summary) => Some(summary),
                                    Err(e) => {
                                        tracing::warn!("Error summarizing story {}: {}", item.title, e);
                                        None
                                    }
                                }
                            }
                            _ => None,
                        };

                        let story_data = StoryData::from_hn_item(item, summary);
                        Some(story_data.to_markdown_string())
                    }
                    Err(e) => {
                        tracing::warn!("Error fetching story {}: {}", story_id, e);
                        None
                    }
                }
            });
        }

        while let Some(result) = tasks.join_next().await {
            if let Ok(Some(markdown)) = result {
                all_stories_markdown.push(markdown);
                processed_count += 1;
            }
        }

        if processed_count > 0 {
            let today_str = OffsetDateTime::now_utc().date().to_string();
            let file_content = all_stories_markdown.join("\n\n---\n\n");
            let file_path = format!("{}/hacker-news.md", today_str);

            self.storage_client
                .upload_file(&file_path, file_content, "text/markdown")
                .await
                .map_err(|e| common::CrawlerError::StorageUpload(e.to_string()))?;
            info!(
                "Successfully processed and uploaded {} stories to {}",
                processed_count, file_path
            );
        } else {
            info!("No stories processed today.");
        }

        Ok(())
    }
}

#[async_trait]
impl Crawler for HackerNewsCrawler {
    async fn run(&self) -> CrawlerResult<()> {
        info!("Hacker News Fetcher starting up");
        self.process_stories().await
    }

    fn name(&self) -> &'static str {
        "Hacker News"
    }
}

// Backward compatibility function
pub async fn run_hacker_news_crawler() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();
    let config = Config::from_env()?;
    let crawler = HackerNewsCrawler::new(&config)?;
    crawler.run().await.map_err(|e| anyhow::anyhow!(e))
}
