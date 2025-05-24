use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrawlerError {
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),
    
    #[error("HTML parsing failed: {0}")]
    HtmlParse(String),
    
    #[error("Storage upload failed: {0}")]
    StorageUpload(String),
    
    #[error("Configuration error: {0}")]
    Config(#[from] anyhow::Error),
    
    #[error("Environment variable error: {0}")]
    EnvVar(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Parsing error: {0}")]
    Parse(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type CrawlerResult<T> = Result<T, CrawlerError>;