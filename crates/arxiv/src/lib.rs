use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};

#[derive(Clone)]
pub struct ArxivClient {
    client: Client,
}

impl ArxivClient {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn fetch_html(&self, arxiv_id: &str) -> Result<String> {
        let url = format!("https://arxiv.org/html/{}", arxiv_id);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.text().await?)
    }

    pub async fn fetch_paper_body(&self, arxiv_id: &str) -> Result<String> {
        let html = self.fetch_html(arxiv_id).await?;
        Ok(extract_body_text(&html))
    }
}

pub fn extract_body_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let body_selector = Selector::parse("body").unwrap();
    let body = document.select(&body_selector).next();

    let full_text = body
        .map(|b| b.text().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();

    let lines: Vec<_> = full_text.lines().collect();
    let mut start_index = 0;

    for (i, line) in lines.iter().enumerate() {
        let clean = line.trim();
        if clean.len() < 40 {
            continue;
        }
        if is_valid_body_line(clean, 100) {
            start_index = i;
            break;
        }
    }

    lines[start_index..]
        .iter()
        .filter_map(|line| {
            let clean = line.trim();
            if clean.len() >= 40 {
                Some(clean.replace('Â', " "))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_valid_body_line(line: &str, min_length: usize) -> bool {
    if line.contains('@') {
        return false;
    }

    let lower = line.to_lowercase();
    for kw in [
        "university",
        "lab",
        "department",
        "institute",
        "corresponding author",
    ] {
        if lower.contains(kw) {
            return false;
        }
    }

    if line.len() < min_length {
        return false;
    }

    line.contains('.')
}
