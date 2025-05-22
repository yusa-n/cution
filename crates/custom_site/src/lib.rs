use anyhow::Result;
use reqwest::Client;
use scraper::Html;
use std::env;
use time::OffsetDateTime;
use tracing::{info, warn};

#[derive(Clone)]
struct SiteFetcher {
    client: Client,
}

impl SiteFetcher {
    fn new() -> Self {
        Self { client: Client::new() }
    }

    async fn fetch(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await?;
        Ok(resp.text().await?)
    }

    fn clean_html(&self, html: &str) -> String {
        Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join("")
    }

    async fn summarize(&self, content: &str) -> Result<String> {
        // Placeholder summary logic
        Ok(content.chars().take(200).collect())
    }
}

struct SupabaseStorageClient {
    client: Client,
    base_url: String,
    api_key: String,
    bucket_name: String,
}

impl SupabaseStorageClient {
    fn new(base_url: &str, api_key: &str, bucket_name: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            bucket_name: bucket_name.to_string(),
        }
    }

    async fn upload_file(&self, path: &str, content: String, content_type: &str) -> Result<()> {
        let url = format!("{}/object/{}/{}", self.base_url, self.bucket_name, path.trim_start_matches('/'));
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", content_type)
            .header("x-upsert", "true")
            .body(content)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let err = resp.text().await?;
            anyhow::bail!("Failed to upload: {}", err)
        }
    }
}

pub async fn run_custom_site_crawler() -> Result<()> {
    let _ = dotenv::dotenv();

    info!("Custom site crawler starting up");

    let url = match env::var("CUSTOM_SITE_URL") {
        Ok(v) => v,
        Err(_) => {
            warn!("CUSTOM_SITE_URL not set; skipping custom site crawler");
            return Ok(());
        }
    };
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY must be set");
    let supabase_bucket = env::var("SUPABASE_BUCKET_NAME").expect("SUPABASE_BUCKET_NAME must be set");

    let fetcher = SiteFetcher::new();
    let storage = SupabaseStorageClient::new(&format!("{}/storage/v1", supabase_url.trim_end_matches('/')), &supabase_key, &supabase_bucket);

    let html = fetcher.fetch(&url).await?;
    let clean_text = fetcher.clean_html(&html);
    let summary = fetcher.summarize(&clean_text).await?;

    let markdown = format!("# Fetched Content\n\nURL: {}\n\n{}", url, summary);
    let today_str = OffsetDateTime::now_utc().date().to_string();
    let file_path = format!("{}/custom-site.md", today_str);
    storage.upload_file(&file_path, markdown, "text/markdown").await?;

    info!("Custom site crawler finished: {}", file_path);
    Ok(())
}

