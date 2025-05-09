use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Call the function defined in lib.rs
    github::run_github_crawler().await
}
