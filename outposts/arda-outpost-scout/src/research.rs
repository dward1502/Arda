//! Bounded internet discovery through a SearXNG-compatible endpoint.

use std::time::Duration;

use arda_outpost_protocol::{
    AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation,
};
use serde::{Deserialize, Serialize};

const MAX_RESULTS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchRequest {
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchResult {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchReport {
    pub query: String,
    pub provider: String,
    pub limit: usize,
    pub results: Vec<ResearchResult>,
}

impl ResearchReport {
    pub fn into_observation(self, source: &str) -> OutpostObservation {
        let query = self.query.clone();
        OutpostObservation::new(
            source,
            ObservationScope::Custom("internet_research".to_string()),
            ObservationClassification::RawMeasurement,
            AuthorityClass::Advisory,
            serde_json::to_value(self).expect("research report is serializable"),
        )
        .with_confidence(0.7)
        .with_provenance(format!("searxng://{query}"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResearchError {
    #[error("research query cannot be empty")]
    EmptyQuery,
    #[error("invalid SearXNG endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("SearXNG request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct SearxngClient {
    endpoint: reqwest::Url,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    #[serde(default)]
    results: Vec<ResearchResult>,
}

impl SearxngClient {
    pub fn new(endpoint: impl AsRef<str>) -> Result<Self, ResearchError> {
        let endpoint = reqwest::Url::parse(endpoint.as_ref())
            .map_err(|error| ResearchError::InvalidEndpoint(error.to_string()))?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()?;
        Ok(Self { endpoint, client })
    }

    pub async fn search(&self, request: &ResearchRequest) -> Result<ResearchReport, ResearchError> {
        let query = request.query.trim();
        if query.is_empty() {
            return Err(ResearchError::EmptyQuery);
        }
        let limit = request.limit.clamp(1, MAX_RESULTS);
        let endpoint = self
            .endpoint
            .join("search")
            .map_err(|error| ResearchError::InvalidEndpoint(error.to_string()))?;
        let response: SearxngResponse = self
            .client
            .get(endpoint)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("language", "en"),
                ("safesearch", "1"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(ResearchReport {
            query: query.to_string(),
            provider: "searxng".to_string(),
            limit,
            results: response.results.into_iter().take(limit).collect(),
        })
    }
}
