use common::{Config, Crawler, CrawlerResult, SupabaseStorageClient};
use time::OffsetDateTime;
use tracing::{info, warn};
use async_trait::async_trait;

pub use self::GithubTrendingFetcher;

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

impl GithubTrendingFetcher {
    pub fn new(config: &Config) -> CrawlerResult<Self> {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .map_err(|e| common::CrawlerError::HttpRequest(e))?;
        
        let supabase_client = SupabaseStorageClient::new(
            &config.supabase.storage_url,
            &config.supabase.key,
            &config.supabase.bucket,
        );
        
        let languages = config.require_languages()?.clone();
        
        Ok(Self {
            http_client,
            supabase_client,
            languages,
        })
    }

    async fn fetch_trending_for_language(
        &self,
        language: &str,
    ) -> CrawlerResult<Vec<Repository>> {
        let url = if language.is_empty() {
            GITHUB_TRENDING_URL_FORMAT.replace("/{language}", "")
        } else {
            GITHUB_TRENDING_URL_FORMAT.replace("{language}", language)
        };
        info!("Fetching trending repositories from: {}", url);

        let response_text = self.http_client.get(&url).send().await
            .map_err(|e| common::CrawlerError::HttpRequest(e))?
            .text().await
            .map_err(|e| common::CrawlerError::HttpRequest(e))?;
        let document = scraper::Html::parse_document(&response_text);

        let article_selector = scraper::Selector::parse("article.Box-row").map_err(|e| {
            common::CrawlerError::HtmlParse(format!("Failed to parse article selector: {}", e))
        })?;
        let name_selector = scraper::Selector::parse("h2.h3 a")
            .map_err(|e| common::CrawlerError::HtmlParse(format!("Failed to parse name selector: {}", e)))?;
        let desc_selector = scraper::Selector::parse("p.col-9").map_err(|e| {
            common::CrawlerError::HtmlParse(format!("Failed to parse description selector: {}", e))
        })?;
        let stars_selector = scraper::Selector::parse("a[href*='/stargazers']")
            .map_err(|e| common::CrawlerError::HtmlParse(format!("Failed to parse stars selector: {}", e)))?;

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

    async fn process(&self) -> CrawlerResult<()> {
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
            self
                .supabase_client
                .upload_file(&file_path, file_content, "text/markdown")
                .await
                .map_err(|e| common::CrawlerError::StorageUpload(e.to_string()))?;
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

#[async_trait]
impl Crawler for GithubTrendingFetcher {
    async fn run(&self) -> CrawlerResult<()> {
        info!("GitHub Trending Fetcher starting up");
        self.process().await
    }

    fn name(&self) -> &'static str {
        "GitHub Trending"
    }
}


// Backward compatibility function
pub async fn run_github_crawler() -> anyhow::Result<()> {
    use dotenv;
    let _ = dotenv::dotenv();
    let config = Config::from_env()?;
    let crawler = GithubTrendingFetcher::new(&config)?;
    crawler.run().await.map_err(|e| anyhow::anyhow!(e))
}
