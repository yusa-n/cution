use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRanking {
    pub rank: usize,
    pub name: String,
    pub score: f64,
    #[serde(with = "time::serde::iso8601")]
    pub fetched_at: OffsetDateTime,
}

impl ModelRanking {
    pub fn new(rank: usize, name: String, score: f64) -> Self {
        Self {
            rank,
            name,
            score,
            fetched_at: OffsetDateTime::now_utc(),
        }
    }
}