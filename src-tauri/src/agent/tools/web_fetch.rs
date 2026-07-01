use super::{ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

/// Minimum characters of extracted text to consider the page non-empty.
/// Pages below this threshold are likely JS-rendered shells with no real content.
const MIN_CONTENT_CHARS: usize = 200;

pub struct WebFetch;

#[async_trait]
impl Tool for WebFetch {
    fn name(&self) -> &'static str {
        "web_fetch"
    }

    fn description(&self) -> &'static str {
        "Fetch the text content of a public URL. HTML is stripped to readable plain text. \
         GitHub blob URLs are automatically converted to raw content. \
         Use this to read documentation, articles, GitHub files, or any web page."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![ParamSpec::req("url", ParamType::Str)]
    }

    async fn execute(&self, args: &Value, _ctx: &ToolCtx) -> AppResult<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("web_fetch requires 'url'".into()))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AppError::Other(
                "url must start with http:// or https://".into(),
            ));
        }

        // Rewrite GitHub blob URLs to raw content before fetching.
        let (fetch_url, rewritten) = rewrite_github_url(url);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; AgentAleph/0.2; +https://agentaleph.com)")
            .build()
            .map_err(|e| AppError::Other(format!("http client error: {e}")))?;

        let resp = client
            .get(&fetch_url)
            .send()
            .await
            .map_err(|e| AppError::Other(format!("fetch failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Other(format!(
                "HTTP {} for {}",
                resp.status(),
                fetch_url
            )));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Other(format!("read body failed: {e}")))?;

        let is_html = content_type.contains("text/html")
            || body.trim_start().starts_with("<!DOCTYPE")
            || body.trim_start().starts_with("<!doctype")
            || body.trim_start().starts_with("<html");

        let text = if is_html { strip_html(&body) } else { body };

        if text.chars().count() < MIN_CONTENT_CHARS {
            let hint = js_rendered_hint(url);
            return Ok(format!(
                "[Page returned almost no readable content ({} chars). \
                 This page is likely JavaScript-rendered and requires a real browser to load.\n\
                 Original URL: {url}{hint}]",
                text.chars().count(),
            ));
        }

        if rewritten {
            Ok(format!("[Fetched raw content from: {fetch_url}]\n\n{text}"))
        } else {
            Ok(text)
        }
    }
}

/// Rewrites GitHub blob URLs to raw.githubusercontent.com so we get plain text content
/// instead of GitHub's JS-rendered HTML shell.
///
/// Examples:
///   https://github.com/owner/repo/blob/main/path/to/file.md
///   → https://raw.githubusercontent.com/owner/repo/main/path/to/file.md
///
/// Returns (url_to_fetch, was_rewritten).
fn rewrite_github_url(url: &str) -> (String, bool) {
    // Match: https://github.com/{owner}/{repo}/blob/{ref}/{path}
    let prefix = "https://github.com/";
    if !url.starts_with(prefix) {
        return (url.to_string(), false);
    }
    let rest = &url[prefix.len()..];
    // rest = "owner/repo/blob/ref/path..."
    let parts: Vec<&str> = rest.splitn(4, '/').collect();
    if parts.len() == 4 && parts[2] == "blob" {
        // parts[3] = "ref/path..." — keep as-is for raw URL
        let raw = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            parts[0], parts[1], parts[3]
        );
        return (raw, true);
    }
    (url.to_string(), false)
}

/// Returns a hint string pointing the model toward an alternative when a page is JS-rendered.
fn js_rendered_hint(url: &str) -> String {
    // GitHub PR/issues/commits pages — suggest using gh CLI via bash instead.
    if url.contains("github.com") && !url.contains("/blob/") && !url.contains("/raw/") {
        return "\n Suggestion: for GitHub pages (PRs, issues, repos) use the \
                `bash` tool with `gh` CLI commands instead of web_fetch."
            .to_string();
    }
    // Generic SPA hint.
    "\n Suggestion: try searching for a cached/mirrored version of this page, \
     or look for a raw/API endpoint that returns plain text or JSON."
        .to_string()
}

/// Strips HTML tags and returns readable plain text.
/// Removes <head>, <script>, and <style> blocks entirely, then strips remaining tags.
fn strip_html(html: &str) -> String {
    let html = remove_block(html, "head");
    let html = remove_block(&html, "script");
    let html = remove_block(&html, "style");
    let html = remove_block(&html, "nav");
    let html = remove_block(&html, "footer");

    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut prev_space = true; // start true to trim leading whitespace

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !prev_space {
                    out.push('\n');
                    prev_space = true;
                }
            }
            _ if in_tag => {}
            '\n' | '\r' => {
                if !prev_space {
                    out.push('\n');
                    prev_space = true;
                }
            }
            c if c.is_whitespace() => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c => {
                out.push(c);
                prev_space = false;
            }
        }
    }

    let decoded = decode_entities(out.trim());

    // Collapse runs of 3+ blank lines into 2.
    let mut result = String::with_capacity(decoded.len());
    let mut blank_run = 0u32;
    for line in decoded.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(line);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

/// Removes all content between `<tag ...>` and `</tag>` (case-insensitive).
fn remove_block(html: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let lower = html.to_lowercase();
    let mut result = String::with_capacity(html.len());
    let mut pos = 0;

    loop {
        let start = match lower[pos..].find(&open) {
            Some(i) => pos + i,
            None => {
                result.push_str(&html[pos..]);
                break;
            }
        };
        result.push_str(&html[pos..start]);
        let search_from = start + open.len();
        let end = match lower[search_from..].find(&close) {
            Some(i) => search_from + i + close.len(),
            None => html.len(),
        };
        pos = end;
        if pos >= html.len() {
            break;
        }
    }

    result
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
}
