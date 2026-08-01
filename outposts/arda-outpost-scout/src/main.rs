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
                    serde_json::to_writer_pretty(std::io::stdout(), &outcomes)?;
                    return Err(format!("research topic failed with HTTP {status}").into());
                }
            }
            serde_json::to_writer_pretty(std::io::stdout(), &outcomes)?;
            println!();
        }
    }
    Ok(())
}
