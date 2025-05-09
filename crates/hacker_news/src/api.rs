use crate::models::HNItem;
use anyhow::Result;
use reqwest::Client;
use scraper::Html;

#[derive(Clone)]
pub struct HackerNewsAPI {
    client: Client,
    base_url: String,
}

impl HackerNewsAPI {
    pub fn new() -> Self {
        let client = Client::new();
        let base_url = "https://hacker-news.firebaseio.com/v0".to_string();
        Self { client, base_url }
    }

    pub async fn get_top_stories(&self, limit: usize) -> Result<Vec<u64>> {
        let url = format!("{}/topstories.json", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let ids: Vec<u64> = resp.json().await?;
        Ok(ids.into_iter().take(limit).collect())
    }

    pub async fn get_story(&self, story_id: u64) -> Result<HNItem> {
        let url = format!("{}/item/{}.json", self.base_url, story_id);
        let resp = self.client.get(&url).send().await?;
        let item: HNItem = resp.json().await?;
        Ok(item)
    }

    pub fn clean_html(&self, html: &str) -> String {
        Html::parse_fragment(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join("")
    }

    pub async fn summarize(&self, _api_key: &str, _title: &str, content: &str) -> Result<String> {
        // TODO: Get summary from LLM
        Ok(content.chars().take(200).collect::<String>())
    }
}
