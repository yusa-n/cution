use std::env;
use anyhow::{Result, Context};

#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub url: String,
    pub storage_url: String,
    pub key: String,
    pub bucket: String,
}

#[derive(Debug, Clone)]
pub struct HackerNewsConfig {
    pub min_html_length: usize,
    pub max_html_length: usize,
    pub min_score_threshold: i32,
    pub max_stories: usize,
}

impl Default for HackerNewsConfig {
    fn default() -> Self {
        Self {
            min_html_length: 100,
            max_html_length: 10_000,
            min_score_threshold: 20,
            max_stories: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub supabase: SupabaseConfig,
    pub gemini_api_key: Option<String>,
    pub xai_api_key: Option<String>,
    pub custom_site_url: Option<String>,
    pub languages: Vec<String>,
    pub hacker_news: HackerNewsConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let supabase_url = env::var("SUPABASE_URL")
            .context("SUPABASE_URL must be set")?;
        let supabase_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .context("SUPABASE_SERVICE_ROLE_KEY must be set")?;
        let supabase_bucket = env::var("SUPABASE_BUCKET_NAME")
            .context("SUPABASE_BUCKET_NAME must be set")?;

        let storage_url = format!("{}/storage/v1", supabase_url.trim_end_matches('/'));

        let languages = env::var("LANGUAGES")
            .ok()
            .map(|langs_str| {
                langs_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let hacker_news = HackerNewsConfig {
            min_html_length: env::var("HN_MIN_HTML_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            max_html_length: env::var("HN_MAX_HTML_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
            min_score_threshold: env::var("HN_MIN_SCORE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            max_stories: env::var("HN_MAX_STORIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        };

        Ok(Config {
            supabase: SupabaseConfig {
                url: supabase_url,
                storage_url,
                key: supabase_key,
                bucket: supabase_bucket,
            },
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            xai_api_key: env::var("XAI_API_KEY").ok(),
            custom_site_url: env::var("CUSTOM_SITE_URL").ok(),
            languages,
            hacker_news,
        })
    }

    pub fn require_gemini_api_key(&self) -> Result<&String> {
        self.gemini_api_key
            .as_ref()
            .context("GEMINI_API_KEY must be set")
    }

    pub fn require_xai_api_key(&self) -> Result<&String> {
        self.xai_api_key
            .as_ref()
            .context("XAI_API_KEY must be set")
    }

    pub fn require_custom_site_url(&self) -> Result<&String> {
        self.custom_site_url
            .as_ref()
            .context("CUSTOM_SITE_URL must be set")
    }

    pub fn require_languages(&self) -> Result<&Vec<String>> {
        if self.languages.is_empty() {
            anyhow::bail!("LANGUAGES environment variable must be set and non-empty");
        }
        Ok(&self.languages)
    }
}