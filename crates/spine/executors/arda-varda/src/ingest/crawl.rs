use super::athena_crawl_limit;
use arda_core::error::{ArdaError, Result};
use arda_core::{try_run_bounded, try_run_bounded_async};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlMarkdownResult {
    pub url: String,
    pub filter: String,
    pub query: Option<String>,
    pub markdown: String,
    pub success: bool,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCaptureReceipt {
    pub source_id: String,
    pub url: String,
    pub captured_at_utc: String,
    pub submitted_by: String,
    pub task_context: String,
    pub filter: String,
    pub query: Option<String>,
    pub markdown_bytes: usize,
    pub artifact_path: String,
    pub crawl_service_url: String,
    pub success: bool,
}

pub async fn crawl4ai_fetch_markdown(
    service_url: &str,
    url: &str,
    filter: &str,
    query: Option<&str>,
) -> Result<CrawlMarkdownResult> {
    let Some(result) = try_run_bounded_async("athena_crawl", athena_crawl_limit(), || async move {
        let filter = match filter.trim().to_ascii_lowercase().as_str() {
            "fit" | "raw" | "bm25" | "llm" => filter.trim().to_ascii_lowercase(),
            _ => {
                return Err(ArdaError::Agent {
                    agent: "athena".to_owned(),
                    message: format!("unsupported crawl4ai filter: {filter}"),
                });
            }
        };

        let client = reqwest::Client::new();
        let endpoint = format!("{}/md", service_url.trim_end_matches('/'));
        let response = client
            .post(&endpoint)
            .json(&serde_json::json!({
                "url": url,
                "f": filter,
                "q": query,
                "c": "0",
            }))
            .send()
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "athena".to_owned(),
                message: format!("failed to call crawl4ai markdown endpoint: {e}"),
            })?
            .error_for_status()
            .map_err(|e| ArdaError::Agent {
                agent: "athena".to_owned(),
                message: format!("crawl4ai markdown endpoint returned failure: {e}"),
            })?;
        let value: serde_json::Value =
            response.json().await.map_err(|e| ArdaError::Agent {
                agent: "athena".to_owned(),
                message: format!("invalid crawl4ai markdown response JSON: {e}"),
            })?;
        let markdown = value
            .get("markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArdaError::Agent {
                agent: "athena".to_owned(),
                message: "crawl4ai response missing markdown".to_owned(),
            })?;

        Ok(CrawlMarkdownResult {
            url: value
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or(url)
                .to_owned(),
            filter,
            query: query.map(str::to_owned),
            markdown: markdown.to_owned(),
            success: value
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            provider: "crawl4ai".to_owned(),
        })
    })
    .await
    else {
        return Err(ArdaError::Agent {
            agent: "athena".to_owned(),
            message: "crawl concurrency gate saturated".to_owned(),
        });
    };

    result
}

pub fn scrapling_fetch_markdown(
    url: &str,
    filter: &str,
    query: Option<&str>,
) -> Result<CrawlMarkdownResult> {
    let Some(result) = try_run_bounded("athena_crawl", athena_crawl_limit(), || {
        let filter = match filter.trim().to_ascii_lowercase().as_str() {
            "fit" | "raw" | "bm25" | "llm" => filter.trim().to_ascii_lowercase(),
            _ => {
                return Err(ArdaError::Agent {
                    agent: "athena".to_owned(),
                    message: format!("unsupported scrapling filter: {filter}"),
                });
            }
        };
        let runtime_mode = resolve_scrapling_runtime_mode();
        let native_available = false;
        if runtime_mode == "native_required" && !native_available {
            return Err(ArdaError::Agent {
                agent: "athena".to_owned(),
                message: "native Scrapling runtime required but no native Rust Scrapling provider is installed".to_owned(),
            });
        }
        let html = fetch_scrapling_html(url)?;
        let markdown = strip_html_to_markdownish(&html);

        Ok(CrawlMarkdownResult {
            url: url.to_owned(),
            filter,
            query: query.map(str::to_owned),
            markdown,
            success: true,
            provider: "scrapling_shim".to_owned(),
        })
    }) else {
        return Err(ArdaError::Agent {
            agent: "athena".to_owned(),
            message: "crawl concurrency gate saturated".to_owned(),
        });
    };

    result
}

fn resolve_scrapling_runtime_mode() -> String {
    match std::env::var("ARDA_SCRAPLING_RUNTIME_MODE") {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "native_required" | "shim_allowed" => raw.trim().to_ascii_lowercase(),
            _ => "shim_allowed".to_owned(),
        },
        Err(_) => "shim_allowed".to_owned(),
    }
}

fn fetch_scrapling_html(url: &str) -> Result<String> {
    if let Some(raw) = url.strip_prefix("raw:") {
        return Ok(raw.to_owned());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_owned(),
            message: format!("failed to build scrapling HTTP client: {e}"),
        })?;
    let response = client
        .get(url)
        .header("User-Agent", "Arda-Scrapling-Rust/0.1")
        .send()
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_owned(),
            message: format!("failed to fetch scrapling source: {e}"),
        })?
        .error_for_status()
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_owned(),
            message: format!("scrapling source returned failure: {e}"),
        })?;
    response.text().map_err(|e| ArdaError::Agent {
        agent: "athena".to_owned(),
        message: format!("failed to decode scrapling source HTML: {e}"),
    })
}

fn strip_html_to_markdownish(html: &str) -> String {
    fn flush_text(buffer: &mut String, parts: &mut Vec<String>) {
        let normalized = buffer.split_whitespace().collect::<Vec<_>>().join(" ");
        if !normalized.is_empty() {
            parts.push(normalized);
        }
        buffer.clear();
    }

    let mut parts = Vec::new();
    let mut text_buffer = String::new();
    let mut tag_buffer = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                if !in_tag {
                    flush_text(&mut text_buffer, &mut parts);
                    in_tag = true;
                    tag_buffer.clear();
                } else {
                    tag_buffer.push(ch);
                }
            }
            '>' => {
                if in_tag {
                    let raw_tag = tag_buffer.split_whitespace().next().unwrap_or_default();
                    let is_closing = raw_tag.starts_with('/');
                    let tag = raw_tag.trim_start_matches('/').to_ascii_lowercase();
                    if !is_closing {
                        match tag.as_str() {
                            "h1" => parts.push("\n# ".to_owned()),
                            "h2" | "h3" | "h4" => parts.push("\n## ".to_owned()),
                            "p" | "div" | "section" | "article" | "main" | "li" | "ul" | "ol"
                            | "br" => parts.push("\n".to_owned()),
                            _ => {}
                        }
                    }
                    tag_buffer.clear();
                    in_tag = false;
                } else {
                    text_buffer.push(ch);
                }
            }
            _ => {
                if in_tag {
                    tag_buffer.push(ch);
                } else {
                    text_buffer.push(ch);
                }
            }
        }
    }
    flush_text(&mut text_buffer, &mut parts);

    let mut output = String::new();
    let mut newline_run = 0usize;
    for part in parts {
        if part == "\n" {
            if newline_run < 2 {
                output.push('\n');
            }
            newline_run += 1;
            continue;
        }
        if part.starts_with("\n# ") || part.starts_with("\n## ") {
            if !output.ends_with('\n') && !output.is_empty() {
                output.push('\n');
            }
            output.push_str(part.trim_start_matches('\n'));
            newline_run = 0;
            continue;
        }
        if !output.is_empty() && !output.ends_with(['\n', ' ']) {
            output.push(' ');
        }
        output.push_str(&part);
        newline_run = 0;
    }

    let trimmed = output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n", trimmed.trim())
}

pub fn resolve_crawl_provider_order(
    configured_order: Option<&str>,
    profile: Option<&str>,
) -> Vec<String> {
    let profile_default = match profile
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "production".to_owned())
        .as_str()
    {
        "research" => "scrapling,crawl4ai",
        _ => "crawl4ai,scrapling",
    };
    let raw = configured_order
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(profile_default);
    let mut providers = Vec::new();
    for provider in raw
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    {
        if !providers.contains(&provider) {
            providers.push(provider);
        }
    }
    if providers.is_empty() {
        return profile_default.split(',').map(str::to_owned).collect();
    }
    providers
}
