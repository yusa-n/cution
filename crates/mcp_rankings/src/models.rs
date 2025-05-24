use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub rank: usize,
    pub name: String,
    pub description: String,
    pub stars: u32,
    #[serde(with = "time::serde::iso8601")]
    pub fetched_at: OffsetDateTime,
}

impl McpServer {
    pub fn new(rank: usize, name: String, description: String, stars: u32) -> Self {
        Self {
            rank,
            name,
            description,
            stars,
            fetched_at: OffsetDateTime::now_utc(),
        }
    }
}