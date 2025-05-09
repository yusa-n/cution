pub mod api;
pub mod models;
pub mod storage;

use anyhow::Result;
use api::HackerNewsAPI;
use models::StoryData;
use std::env;
use storage::SupabaseStorageClient;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::info;

// 複数のストーリーを並列で処理する公開関数
pub async fn run_hacker_news_crawler() -> Result<()> {
    let _ = dotenv::dotenv();

    info!("Hacker News Fetcher starting up");

    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key =
        env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY must be set");
    let supabase_bucket =
        env::var("SUPABASE_BUCKET_NAME").expect("SUPABASE_BUCKET_NAME must be set");

    let hn_api = HackerNewsAPI::new();
    let storage_client = SupabaseStorageClient::new(
        &format!("{}/storage/v1", supabase_url.trim_end_matches('/')),
        &supabase_key,
        &supabase_bucket,
    );

    let story_ids = hn_api.get_top_stories(30).await?;
    info!("Fetched {} top story IDs", story_ids.len());

    let mut all_stories_markdown: Vec<String> = Vec::new();
    let mut processed_count = 0;

    // JoinSetを使用して複数のストーリーを並列処理
    let mut tasks = JoinSet::new();

    // 各ストーリーの処理を並列タスクとして登録
    for story_id in story_ids {
        let hn_api = hn_api.clone();
        let gemini_api_key = gemini_api_key.clone();
        tasks.spawn(async move {
            match hn_api.get_story(story_id).await {
                Ok(item) => {
                    if item.score < 20 {
                        return None;
                    }

                    let summary = match &item.text {
                        Some(html) if (100..10_000).contains(&html.len()) => {
                            info!("Summarizing story: {}", item.title);
                            let clean_text = hn_api.clean_html(html);
                            match hn_api
                                .summarize(&gemini_api_key, &item.title, &clean_text)
                                .await
                            {
                                Ok(summary) => Some(summary),
                                Err(e) => {
                                    eprintln!("Error summarizing story {}: {}", item.title, e);
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
                    eprintln!("Error fetching story {}: {}", story_id, e);
                    None
                }
            }
        });
    }

    // 並列タスクの結果を収集
    while let Some(result) = tasks.join_next().await {
        if let Ok(Some(markdown)) = result {
            all_stories_markdown.push(markdown);
            processed_count += 1;
        }
    }

    if processed_count > 0 {
        let today_str = OffsetDateTime::now_utc().date().to_string(); // YYYY-MM-DD
        let file_content = all_stories_markdown.join("\n\n---\n\n");
        let file_path = format!("{}/hacker-news.md", today_str);

        storage_client
            .upload_file(&file_path, file_content, "text/markdown")
            .await?;
        info!(
            "Successfully processed and uploaded {} stories to {}",
            processed_count, file_path
        );
    } else {
        info!("No stories processed today.");
    }

    Ok(())
}
