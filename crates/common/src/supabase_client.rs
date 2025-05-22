use anyhow::Result;
use reqwest::Client;
use tracing::info;

#[derive(Clone)]
pub struct SupabaseStorageClient {
    client: Client,
    base_url: String,
    api_key: String,
    bucket_name: String,
}

impl SupabaseStorageClient {
    pub fn new(base_url: &str, api_key: &str, bucket_name: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            bucket_name: bucket_name.to_string(),
        }
    }

    pub async fn upload_file(&self, path: &str, content: String, content_type: &str) -> Result<()> {
        let url = format!(
            "{}/object/{}/{}",
            self.base_url,
            self.bucket_name,
            path.trim_start_matches('/')
        );

        info!("Uploading to Supabase Storage: {} ({} bytes)", url, content.len());

        let response = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", content_type)
            .header("x-upsert", "true")
            .body(content)
            .send()
            .await?;

        if response.status().is_success() {
            info!("Successfully uploaded {} to Supabase Storage.", path);
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upload to Supabase Storage ({}): {}", url, error_text);
        }
    }
}
