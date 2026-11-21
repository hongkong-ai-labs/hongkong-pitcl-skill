//! Latest notices from the PITCL website.
//!
//! The site is a small PHP application with no JSON API, so the list and
//! detail pages are parsed as HTML. Its TLS certificate does not match the
//! hostname, hence `danger_accept_invalid_certs`; only public, read-only
//! pages are fetched.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;

const BASE: &str = "https://www.pitcl.com.hk";
const USER_AGENT: &str = "mawan/0.1 (Ma Wan bus CLI; +https://github.com/)";

#[derive(Debug, Clone, Serialize)]
pub struct NewsItem {
    pub id: u32,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewsDetail {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub content: String,
}

pub struct NewsClient {
    http: reqwest::Client,
}

impl NewsClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(20))
            .build()
            .context("building PITCL news client")?;
        Ok(Self { http })
    }

    pub async fn list(&self, lang: &str, limit: usize) -> Result<Vec<NewsItem>> {
        let html = self
            .get(&format!("/{lang}/news.php"))
            .await
            .context("fetching PITCL notices")?;
        let mut items = Vec::new();
        for block in html.split("<div class=\"news\">").skip(1) {
            let Some(title) = between(block, "<div class=\"news_title\">", "</div>") else {
                continue;
            };
            let Some(id) = between(block, "news_detail.php?id=", "\"") else {
                continue;
            };
            let Ok(id) = id.trim().parse::<u32>() else {
                continue;
            };
            items.push(NewsItem {
                id,
                title: clean_text(title),
                url: detail_url(lang, id),
            });
            if limit > 0 && items.len() == limit {
                break;
            }
        }
        if items.is_empty() {
            bail!("no notices found on the PITCL news page");
        }
        Ok(items)
    }

    pub async fn detail(&self, lang: &str, id: u32) -> Result<NewsDetail> {
        let path = format!("/{lang}/news_detail.php?id={id}");
        let html = self
            .get(&path)
            .await
            .with_context(|| format!("fetching PITCL notice {id}"))?;
        let title = between(&html, "<div class=\"news_detail_title\">", "</div>")
            .map(clean_text)
            .filter(|title| !title.is_empty())
            .ok_or_else(|| anyhow::anyhow!("PITCL notice {id} has no title"))?;
        let raw = between(
            &html,
            "<div class=\"news_detail_content\">",
            "<div class=\"news_btn\">",
        )
        .unwrap_or("");
        Ok(NewsDetail {
            id,
            title,
            url: detail_url(lang, id),
            content: clean_html(raw),
        })
    }

    async fn get(&self, path: &str) -> Result<String> {
        let url = format!("{BASE}{path}");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("reading {url}"))?;
        if !status.is_success() {
            bail!(
                "GET {url} returned HTTP {status}: {}",
                body.chars().take(160).collect::<String>()
            );
        }
        Ok(body)
    }
}

fn detail_url(lang: &str, id: u32) -> String {
    format!("{BASE}/{lang}/news_detail.php?id={id}")
}

fn between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let begin = haystack.find(start)? + start.len();
    let rest = &haystack[begin..];
    let finish = rest.find(end)?;
    Some(&rest[..finish])
}

fn clean_text(value: &str) -> String {
    decode_entities(&strip_tags(value))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_html(value: &str) -> String {
    let mut text = value.to_string();
    for (from, to) in [
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("</p>", "\n\n"),
        ("</div>", "\n"),
        ("</li>", "\n"),
        ("</tr>", "\n"),
        ("</td>", "  "),
        ("</th>", "  "),
    ] {
        text = text.replace(from, to);
    }
    let text = decode_entities(&strip_tags(&text));
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if lines.last().is_some_and(|last| !last.is_empty()) {
                lines.push(String::new());
            }
        } else {
            lines.push(line);
        }
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn strip_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut inside = false;
    for ch in value.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        let tail = &rest[index..];
        let Some(end) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let entity = &tail[..=end];
        let decoded = match entity {
            "&amp;" => Some('&'),
            "&lt;" => Some('<'),
            "&gt;" => Some('>'),
            "&quot;" => Some('"'),
            "&apos;" | "&#39;" => Some('\''),
            "&nbsp;" => Some(' '),
            _ => entity
                .strip_prefix("&#")
                .and_then(|value| value.strip_suffix(';'))
                .and_then(|value| value.parse::<u32>().ok())
                .and_then(char::from_u32),
        };
        match decoded {
            Some(ch) => out.push(ch),
            None => out.push_str(entity),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_news_list_block() {
        let html = r#"
            <div class="news">
              <div class="news_title">Ferry &amp; bus changes</div>
              <div class="news_btn"><a href="news_detail.php?id=61">Details +</a></div>
            </div>
        "#;
        let title = between(html, "<div class=\"news_title\">", "</div>").unwrap();
        assert_eq!(clean_text(title), "Ferry & bus changes");
        let id = between(html, "news_detail.php?id=", "\"").unwrap();
        assert_eq!(id, "61");
    }

    #[test]
    fn cleans_detail_html_into_readable_text() {
        let html = "<p>First&nbsp;line</p><p>Second &amp; last</p>";
        assert_eq!(clean_html(html), "First line\n\nSecond & last");
    }
}
