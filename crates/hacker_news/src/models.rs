use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct HNItem {
    pub id: u64,
    pub title: String,
    pub score: i64,
    pub url: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoryData {
    pub story_id: u64,
    pub title: String,
    pub score: i64,
    pub url: Option<String>,
    pub text: Option<String>,
    pub summary: Option<String>,
}

impl StoryData {
    pub fn from_hn_item(item: HNItem, summary: Option<String>) -> Self {
        Self {
            story_id: item.id,
            title: item.title,
            score: item.score,
            url: item.url,
            text: item.text,
            summary,
        }
    }

    pub fn to_markdown_string(&self) -> String {
        let url_or_summary_or_text = self
            .url
            .as_ref()
            .map(|u| format!("[View Link]({})", u))
            .or_else(|| self.summary.clone())
            .or_else(|| {
                self.text.as_ref().map(|t| {
                    scraper::Html::parse_fragment(t)
                        .root_element()
                        .text()
                        .collect::<String>()
                })
            })
            .unwrap_or_else(|| String::from("No content available."));

        format!(
            "# {}\n\n**Score**: {}\n\n{}",
            self.title, self.score, url_or_summary_or_text
        )
    }
}
