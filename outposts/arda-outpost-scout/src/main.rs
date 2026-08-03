use std::net::SocketAddr;
use std::path::PathBuf;

use arda_outpost_scout::{
    build_runtime_router, ResearchRequest, ScoutRuntimeState, ALLOWLISTED_PUBLIC_WEB_POLICY,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "arda-outpost-scout",
    about = "Warden scout and research outpost runtime"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the Tailscale-facing scout API.
    Serve {
        #[arg(long, env = "SCOUT_BIND", default_value = "127.0.0.1:8092")]
        bind: SocketAddr,
        #[arg(long, env = "SCOUT_MEMORY_ROOT")]
        memory_root: PathBuf,
        #[arg(
            long,
            env = "SCOUT_SEARXNG_URL",
            default_value = "http://127.0.0.1:18080"
        )]
        searxng_url: String,
        #[arg(long, env = "SCOUT_SOURCE", default_value = "node-pi5-warden")]
        source: String,
    },
    /// Run configured research topics against a running scout API.
    RunTopics {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8092")]
        endpoint: String,
    },
}

#[derive(Debug, Deserialize)]
struct TopicConfig {
    topics: Vec<ResearchTopic>,
}

#[derive(Debug, Deserialize)]
struct ResearchTopic {
    id: String,
    query: String,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Serialize)]
struct TopicOutcome {
    id: String,
    status: String,
    detail: serde_json::Value,
}

fn enabled_by_default() -> bool {
    true
}

fn default_limit() -> usize {
    5
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Serve {
            bind,
            memory_root,
            searxng_url,
            source,
        } => {
            let state = ScoutRuntimeState::new(memory_root, searxng_url, source)?;
            let listener = tokio::net::TcpListener::bind(bind).await?;
            eprintln!("arda-outpost-scout listening on {bind}");
            axum::serve(listener, build_runtime_router(state)).await?;
        }
        Command::RunTopics { config, endpoint } => {
            let config: TopicConfig = serde_json::from_slice(&std::fs::read(config)?)?;
            let outcomes = run_topics(config, endpoint).await?;
            serde_json::to_writer_pretty(std::io::stdout(), &outcomes)?;
            println!();
        }
    }
    Ok(())
}

async fn run_topics(
    config: TopicConfig,
    endpoint: String,
) -> Result<Vec<TopicOutcome>, Box<dyn std::error::Error>> {
    let endpoint = reqwest::Url::parse(&endpoint)?.join("search")?;
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let mut outcomes = Vec::new();
    for topic in config
        .topics
        .into_iter()
        .filter(|topic| topic.enabled)
        .take(16)
    {
        let response = client
            .post(endpoint.clone())
            .json(&ResearchRequest {
                query: topic.query,
                limit: topic.limit,
                source_policy: ALLOWLISTED_PUBLIC_WEB_POLICY.to_string(),
                expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(15)),
            })
            .send()
            .await?;
        let status = response.status();
        let detail = response.json::<serde_json::Value>().await?;
        outcomes.push(TopicOutcome {
            id: topic.id,
            status: status.as_u16().to_string(),
            detail,
        });
        if !status.is_success() {
            return Err(format!("research topic failed with HTTP {status}").into());
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_outpost_protocol::ResearchSuggestionLedger;
    use axum::http::StatusCode;
    use axum::{routing::get, Json, Router};
    use serde_json::json;
    use tempfile::tempdir;

    async fn mock_search() -> (StatusCode, Json<serde_json::Value>) {
        (
            StatusCode::OK,
            Json(json!({
                "results": [{
                    "title": "Static topic source",
                    "url": "https://example.com/static-topic",
                    "content": "canonical static-topic evidence",
                    "engine": "fixture",
                    "score": 1.0
                }]
            })),
        )
    }

    #[tokio::test]
    async fn static_topics_produce_the_typed_durable_suggestion_contract() {
        let root = tempdir().unwrap();
        let searx_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let searx_addr = searx_listener.local_addr().unwrap();
        let searx_handle = tokio::spawn(async move {
            axum::serve(
                searx_listener,
                Router::new().route("/search", get(mock_search)),
            )
            .await
            .unwrap();
        });

        let scout_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let scout_addr = scout_listener.local_addr().unwrap();
        let state = ScoutRuntimeState::new(
            root.path(),
            format!("http://{searx_addr}"),
            "static-topic-fixture",
        )
        .unwrap();
        let scout_handle = tokio::spawn(async move {
            axum::serve(scout_listener, build_runtime_router(state))
                .await
                .unwrap();
        });

        let outcomes = run_topics(
            TopicConfig {
                topics: vec![ResearchTopic {
                    id: "static-topic-1".to_owned(),
                    query: "static topic query".to_owned(),
                    enabled: true,
                    limit: 1,
                }],
            },
            format!("http://{scout_addr}"),
        )
        .await
        .unwrap();

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, "200");
        let suggestions = ResearchSuggestionLedger::open(
            root.path().join("data/warden/research_suggestions.jsonl"),
        )
        .unwrap()
        .suggestions()
        .unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].schema_version, "arda.warden.research.v1");
        assert_eq!(suggestions[0].authority, "advisory_only");
        assert_eq!(suggestions[0].query, "static topic query");
        assert_eq!(suggestions[0].max_results, 1);

        scout_handle.abort();
        searx_handle.abort();
    }
}
