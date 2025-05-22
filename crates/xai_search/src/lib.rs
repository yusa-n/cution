use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use time::OffsetDateTime;
use tracing::{info, warn};
use common::SupabaseStorageClient;

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub struct XaiClient {
    http_client: Client,
    api_key: String,
    supabase_client: SupabaseStorageClient,
}

impl XaiClient {
    pub fn new(api_key: &str, supabase_url: &str, supabase_key: &str, supabase_bucket: &str) -> Self {
        let http_client = Client::new();
        let supabase_client = SupabaseStorageClient::new(supabase_url, supabase_key, supabase_bucket);
        Self {
            http_client,
            api_key: api_key.to_string(),
            supabase_client,
        }
    }

    async fn fetch_news_digest(&self) -> Result<String> {
        let url = "https://api.x.ai/v1/chat/completions";
        let body = serde_json::json!({
            "messages": [{"role": "user", "content": "Provide me a digest of world news in the last 24 hours."}],
            "search_parameters": {"mode": "auto"},
            "model": "grok-3-latest"
        });

        let res = self
            .http_client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("Request failed: {} - {}", status, text);
        }

        let resp: ChatCompletionResponse = res.json().await?;
        let content = resp
            .choices
            .get(0)
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        Ok(content)
    }

    pub async fn run(&self) -> Result<()> {
        info!("Fetching news digest from xAI");
        let digest = self.fetch_news_digest().await?;

        if digest.is_empty() {
            warn!("Received empty digest from xAI");
            return Ok(());
        }

        let today = OffsetDateTime::now_utc().date().to_string();
        let file_path = format!("{}/xai-news.md", today);
        self
            .supabase_client
            .upload_file(&file_path, digest, "text/markdown")
            .await?;
        info!("Uploaded xAI news digest to {}", file_path);
        Ok(())
    }
}


pub async fn run_xai_search() -> Result<()> {
    let _ = dotenv::dotenv();
    let api_key = match env::var("XAI_API_KEY") {
        Ok(v) => v,
        Err(_) => {
            warn!("XAI_API_KEY not set; skipping xAI search");
            return Ok(());
        }
    };
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY must be set");
    let supabase_bucket = env::var("SUPABASE_BUCKET_NAME").expect("SUPABASE_BUCKET_NAME must be set");

    let client = XaiClient::new(
        &api_key,
        &format!("{}/storage/v1", supabase_url.trim_end_matches('/')),
        &supabase_key,
        &supabase_bucket,
    );

    client.run().await
}
