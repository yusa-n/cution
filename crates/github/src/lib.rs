use anyhow::Result;
use std::env;
use time::OffsetDateTime;
use tracing::{info, warn};

const GITHUB_TRENDING_URL_FORMAT: &str = "https://github.com/trending/{language}?since=daily";
const MARKDOWN_FORMAT: &str =
    "\n# {title}\n\n**Stars**: {stars}\n\n[View Repository]({link})\n\n{description}\n";

#[derive(Debug)]
struct Repository {
    name: String,
    description: Option<String>,
    link: String,
    stars: String, // Keep as String for direct insertion into markdown
}

pub struct GithubTrendingFetcher {
    http_client: reqwest::Client,
    supabase_client: SupabaseStorageClient,
    languages: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("HTML parsing failed: {0}")]
    HtmlParse(String),
    #[error("Supabase upload failed: {0}")]
    SupabaseUpload(String),
    #[error("Environment variable LANGUAGES not set or empty")]
    LanguagesEnvVarMissing,
}

impl GithubTrendingFetcher {
    pub async fn new(
        supabase_url: &str,
        supabase_key: &str,
        supabase_bucket: &str,
    ) -> Result<Self, FetchError> {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()?;
        let supabase_client =
            SupabaseStorageClient::new(supabase_url, supabase_key, supabase_bucket);
        let languages = Self::load_languages()?;
        Ok(Self {
            http_client,
            supabase_client,
            languages,
        })
    }

    fn load_languages() -> Result<Vec<String>, FetchError> {
        match env::var("LANGUAGES") {
            Ok(langs_str) if !langs_str.trim().is_empty() => {
                let languages = langs_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>();
                if languages.is_empty() {
                    warn!("'LANGUAGES' environment variable is set but resulted in an empty list after parsing.");
                    Err(FetchError::LanguagesEnvVarMissing)
                } else {
                    info!("Loaded languages from ENV: {:?}", languages);
                    Ok(languages)
                }
            }
            _ => {
                warn!("'LANGUAGES' environment variable not set or is empty. No languages to process.");
                Err(FetchError::LanguagesEnvVarMissing) // Or return Ok(Vec::new()) if it's acceptable to run with no languages
            }
        }
    }

    async fn fetch_trending_for_language(
        &self,
        language: &str,
    ) -> Result<Vec<Repository>, FetchError> {
        let url = if language.is_empty() {
            GITHUB_TRENDING_URL_FORMAT.replace("/{language}", "")
        } else {
            GITHUB_TRENDING_URL_FORMAT.replace("{language}", language)
        };
        info!("Fetching trending repositories from: {}", url);

        let response_text = self.http_client.get(&url).send().await?.text().await?;
        let document = scraper::Html::parse_document(&response_text);

        let article_selector = scraper::Selector::parse("article.Box-row").map_err(|e| {
            FetchError::HtmlParse(format!("Failed to parse article selector: {}", e))
        })?;
        let name_selector = scraper::Selector::parse("h2.h3 a")
            .map_err(|e| FetchError::HtmlParse(format!("Failed to parse name selector: {}", e)))?;
        let desc_selector = scraper::Selector::parse("p.col-9").map_err(|e| {
            FetchError::HtmlParse(format!("Failed to parse description selector: {}", e))
        })?;
        let stars_selector = scraper::Selector::parse("a[href*='/stargazers']")
            .map_err(|e| FetchError::HtmlParse(format!("Failed to parse stars selector: {}", e)))?;

        let mut repositories = Vec::new();

        for article in document.select(&article_selector) {
            let name_element = article.select(&name_selector).next();
            let repo_name_and_owner = name_element
                .and_then(|el| el.attr("href"))
                .map(|href| href.trim_start_matches('/').to_string());

            if repo_name_and_owner.is_none() {
                warn!("Could not extract repository name and owner from an article. Skipping.");
                continue;
            }
            let full_name = repo_name_and_owner.unwrap();

            let description = article
                .select(&desc_selector)
                .next()
                .map(|p| p.text().collect::<String>().trim().to_string());

            let stars = article
                .select(&stars_selector)
                .next()
                .map(|s| s.text().collect::<String>().trim().replace(',', ""))
                .unwrap_or_else(|| "0".to_string());

            repositories.push(Repository {
                name: full_name.clone(),
                link: format!("https://github.com/{}", full_name),
                description,
                stars,
            });
        }
        info!(
            "Found {} repositories for language '{}'",
            repositories.len(),
            if language.is_empty() {
                "overall"
            } else {
                language
            }
        );
        Ok(repositories)
    }

    fn stylize_repository_info(&self, repository: &Repository) -> String {
        MARKDOWN_FORMAT
            .replace("{title}", &repository.name)
            .replace("{stars}", &repository.stars)
            .replace("{link}", &repository.link)
            .replace(
                "{description}",
                repository
                    .description
                    .as_deref()
                    .unwrap_or("No description provided."),
            )
    }

    pub async fn run(&self) -> Result<()> {
        let mut all_markdowns: Vec<String> = Vec::new();
        let mut processed_languages = 0;

        // 各言語のクローリングを並列化
        let mut tasks = Vec::new();
        for language in &self.languages {
            let language_clone = language.clone();
            tasks.push(tokio::spawn({
                let self_clone = self.clone();
                async move {
                    match self_clone
                        .fetch_trending_for_language(&language_clone)
                        .await
                    {
                        Ok(repos) => {
                            let mut markdown_results = Vec::new();
                            for repo in repos {
                                markdown_results.push(self_clone.stylize_repository_info(&repo));
                            }
                            if !markdown_results.is_empty() {
                                Some((language_clone, markdown_results))
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to fetch trending for language '{}': {}",
                                language_clone, e
                            );
                            None
                        }
                    }
                }
            }));
        }

        // 全てのタスクの結果を集約
        for task in tasks {
            if let Ok(Some((language, markdowns))) = task.await {
                all_markdowns.extend(markdowns);
                processed_languages += 1;
                info!("Processed language: {}", language);
            }
        }

        if processed_languages > 0 && !all_markdowns.is_empty() {
            let today_str = OffsetDateTime::now_utc().date().to_string(); // YYYY-MM-DD
            let file_content = all_markdowns.join("\n---\n");
            let file_path = format!("{}/github-trending.md", today_str);

            info!(
                "Uploading {} trending repositories to Supabase Storage at {}",
                all_markdowns.len(),
                file_path
            );
            self.supabase_client
                .upload_file(&file_path, file_content, "text/markdown")
                .await
                .map_err(FetchError::SupabaseUpload)?;
            info!(
                "Successfully uploaded trending repositories to {}",
                file_path
            );
        } else {
            info!("No trending repositories processed or found.");
        }
        Ok(())
    }
}

// Clone トレイトを実装して、各言語処理の並列タスク内でクローンして使用できるようにする
impl Clone for GithubTrendingFetcher {
    fn clone(&self) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .expect("Failed to build HTTP client clone");

        Self {
            http_client,
            supabase_client: self.supabase_client.clone(),
            languages: self.languages.clone(),
        }
    }
}

// Simplified Supabase client (similar to hacker_news crate)
#[derive(Clone)]
struct SupabaseStorageClient {
    base_url: String,
    api_key: String,
    bucket_name: String,
    http_client: reqwest::Client,
}

impl SupabaseStorageClient {
    fn new(base_url: &str, api_key: &str, bucket_name: &str) -> Self {
        SupabaseStorageClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            bucket_name: bucket_name.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    async fn upload_file(
        &self,
        path: &str,
        content: String,
        content_type: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/object/{}/{}",
            self.base_url,
            self.bucket_name,
            path.trim_start_matches('/')
        );

        let response = self
            .http_client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", content_type)
            .header("x-upsert", "true")
            .body(content)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => Ok(()),
            Ok(res) => Err(format!(
                "Failed to upload file: {} - {}",
                res.status(),
                res.text().await.unwrap_or_else(|_| "<no body>".to_string())
            )),
            Err(e) => Err(format!("HTTP request failed during upload: {}", e)),
        }
    }
}

// メイン関数をライブラリの関数として公開
pub async fn run_github_crawler() -> Result<()> {
    let _ = dotenv::dotenv();

    info!("GitHub Trending Fetcher starting up");

    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key =
        env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY must be set");
    let supabase_bucket =
        env::var("SUPABASE_BUCKET_NAME").expect("SUPABASE_BUCKET_NAME must be set");

    let fetcher = GithubTrendingFetcher::new(
        &format!("{}/storage/v1", supabase_url.trim_end_matches('/')),
        &supabase_key,
        &supabase_bucket,
    )
    .await?;

    if let Err(e) = fetcher.run().await {
        eprintln!("Error running fetcher: {}", e);
        return Err(e.into());
    }

    info!("GitHub Trending Fetcher finished successfully.");
    Ok(())
}
