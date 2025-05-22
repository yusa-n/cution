use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use time::OffsetDateTime;
use tracing::{info, warn};

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

#[derive(Clone)]
struct SupabaseStorageClient {
    base_url: String,
    api_key: String,
    bucket_name: String,
    http_client: Client,
}

impl SupabaseStorageClient {
    fn new(base_url: &str, api_key: &str, bucket_name: &str) -> Self {
        SupabaseStorageClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            bucket_name: bucket_name.to_string(),
            http_client: Client::new(),
        }
    }

    async fn upload_file(&self, path: &str, content: String, content_type: &str) -> Result<()> {
        let url = format!(
            "{}/object/{}/{}",
            self.base_url,
            self.bucket_name,
            path.trim_start_matches('/')
        );

        let res = self
            .http_client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", content_type)
            .header("x-upsert", "true")
            .body(content)
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("Upload failed: {} - {}", status, text);
        }
    }
}

pub async fn run_xai_search() -> Result<()> {
    let _ = dotenv::dotenv();
    let api_key = env::var("XAI_API_KEY").expect("XAI_API_KEY must be set");
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
