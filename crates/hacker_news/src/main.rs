mod api;
mod models;
mod storage;

use anyhow::Result;
use api::HackerNewsAPI;
use models::StoryData;
use std::env;
use storage::SupabaseStorageClient;
use time::OffsetDateTime;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

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

    for story_id in story_ids {
        let item = hn_api.get_story(story_id).await?;
        if item.score < 20 {
            continue;
        }

        let summary = match &item.text {
            Some(html) if (100..10_000).contains(&html.len()) => {
                info!("Summarizing story: {}", item.title);
                let clean_text = hn_api.clean_html(html);
                Some(
                    hn_api
                        .summarize(&gemini_api_key, &item.title, &clean_text)
                        .await?,
                )
            }
            _ => None,
        };

        let story_data = StoryData::from_hn_item(item, summary);
        all_stories_markdown.push(story_data.to_markdown_string());
        processed_count += 1;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
