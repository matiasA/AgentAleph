use super::{ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;

const DEFAULT_RESULTS: usize = 8;
const MAX_RESULTS: usize = 20;

pub struct WebSearch;

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for a query and return a list of results (title, url, snippet). \
         Use this to find documentation, GitHub repos, Stack Overflow answers, or any web content. \
         Follow up with web_fetch on a result URL to read the full page."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("query", ParamType::Str),
            ParamSpec::opt("num_results", ParamType::Int),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("web_search requires 'query'".into()))?;

        let num = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| (n as usize).min(MAX_RESULTS))
            .unwrap_or(DEFAULT_RESULTS);

        let results = if !ctx.brave_api_key.is_empty() {
            search_brave(query, num, &ctx.brave_api_key).await?
        } else {
            search_ddg(query, num).await?
        };

        if results.is_empty() {
            return Ok(format!("No results found for: {query}"));
        }

        let mut out = format!("Search results for: {query}\n\n");
        for (i, r) in results.iter().enumerate() {
            out.push_str(&format!("{}. {}\n   {}\n   {}\n\n", i + 1, r.title, r.url, r.snippet));
        }
        Ok(out)
    }
}

struct SearchResult {
    title:   String,
    url:     String,
    snippet: String,
}

// ── Brave Search API ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BraveResponse {
    web: Option<BraveWeb>,
}

#[derive(Deserialize)]
struct BraveWeb {
    results: Vec<BraveResult>,
}

#[derive(Deserialize)]
struct BraveResult {
    title:       Option<String>,
    url:         Option<String>,
    description: Option<String>,
}

async fn search_brave(query: &str, num: usize, api_key: &str) -> AppResult<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Other(format!("http client error: {e}")))?;

    let resp = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .query(&[("q", query), ("count", &num.to_string())])
        .send()
        .await
        .map_err(|e| AppError::Other(format!("Brave Search request failed: {e}")))?;

    if resp.status() == 401 || resp.status() == 403 {
        return Err(AppError::Other(
            "Brave Search API key is invalid or expired. \
             Update it in Settings or remove it to fall back to DuckDuckGo."
                .into(),
        ));
    }
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "Brave Search returned HTTP {}",
            resp.status()
        )));
    }

    let body: BraveResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Other(format!("Brave Search JSON parse error: {e}")))?;

    let results = body
        .web
        .map(|w| w.results)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| {
            let title = r.title?.trim().to_string();
            let url = r.url?.trim().to_string();
            if title.is_empty() || url.is_empty() {
                return None;
            }
            let snippet = r.description.unwrap_or_default().trim().to_string();
            Some(SearchResult { title, url, snippet })
        })
        .collect();

    Ok(results)
}

// ── DuckDuckGo fallback ───────────────────────────────────────────────────────

async fn search_ddg(query: &str, num: usize) -> AppResult<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| AppError::Other(format!("http client error: {e}")))?;

    let body = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .send()
        .await
        .map_err(|e| AppError::Other(format!("DDG request failed: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Other(format!("DDG response read failed: {e}")))?;

    // Detect CAPTCHA / bot-block page before trying to parse results.
    if body.contains("anomaly.js") || body.contains("challenge-form") || body.contains("anomaly-modal") {
        return Err(AppError::Other(
            "DuckDuckGo is temporarily rate-limiting this IP (bot CAPTCHA). \
             **Do not retry** — either wait a few minutes and try again, \
             or configure a Brave Search API key in Settings for reliable search."
                .into(),
        ));
    }

    parse_ddg_html(&body, num)
}

fn parse_ddg_html(html: &str, num: usize) -> AppResult<Vec<SearchResult>> {
    let document = Html::parse_document(html);

    let sel_result  = Selector::parse(".result:not(.result--ad)").unwrap();
    let sel_title   = Selector::parse(".result__title a").unwrap();
    let sel_snippet = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for el in document.select(&sel_result).take(num) {
        let title = el
            .select(&sel_title)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let raw_href = el
            .select(&sel_title)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();
        let url = extract_uddg(raw_href).unwrap_or_else(|| raw_href.to_string());

        let snippet = el
            .select(&sel_snippet)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if title.is_empty() || url.is_empty() {
            continue;
        }

        results.push(SearchResult { title, url, snippet });
    }

    Ok(results)
}

fn extract_uddg(href: &str) -> Option<String> {
    let normalized = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };

    let parsed = url::Url::parse(&normalized).ok()?;
    let uddg = parsed
        .query_pairs()
        .find(|(k, _)| k == "uddg")
        .map(|(_, v)| v.into_owned())?;

    urlencoding::decode(&uddg).ok().map(|s| s.into_owned())
}
