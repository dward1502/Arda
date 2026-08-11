//! Bounded internet discovery through a SearXNG-compatible endpoint.

use std::time::Duration;

use arda_outpost_protocol::{
    AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};

const MAX_RESULTS: usize = 10;
const MAX_QUERY_BYTES: usize = 512;
const MAX_REQUEST_TTL_HOURS: i64 = 24;
pub const ALLOWLISTED_PUBLIC_WEB_POLICY: &str = "allowlisted_public_web";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchRequest {
    pub query: String,
    pub limit: usize,
    #[serde(default)]
    pub source_policy: String,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

impl ResearchRequest {
    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), ResearchError> {
        if self.query.trim().is_empty() {
            return Err(ResearchError::EmptyQuery);
        }
        if self.query.len() > MAX_QUERY_BYTES {
            return Err(ResearchError::QueryTooLong(MAX_QUERY_BYTES));
        }
        if self.source_policy != ALLOWLISTED_PUBLIC_WEB_POLICY {
            return Err(ResearchError::UnsupportedSourcePolicy(
                self.source_policy.clone(),
            ));
        }
        let expires_at = self.expires_at.ok_or(ResearchError::MissingExpiry)?;
        if expires_at <= now {
            return Err(ResearchError::ExpiredRequest);
        }
        if expires_at > now + ChronoDuration::hours(MAX_REQUEST_TTL_HOURS) {
            return Err(ResearchError::ExpiryTooDistant(MAX_REQUEST_TTL_HOURS));
        }
        Ok(())
    }
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
    pub source_policy: String,
    pub expires_at: DateTime<Utc>,
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
    #[error("research query exceeds {0} bytes")]
    QueryTooLong(usize),
    #[error("unsupported source policy: {0}")]
    UnsupportedSourcePolicy(String),
    #[error("research request expiry is required")]
    MissingExpiry,
    #[error("research request has expired")]
    ExpiredRequest,
    #[error("research request expiry exceeds the {0}-hour bound")]
    ExpiryTooDistant(i64),
    #[error("research result is missing a valid HTTP(S) source URL: {0}")]
    InvalidSourceUrl(String),
    #[error("invalid SearXNG endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("crawl4ai returned no markdown for URL: {0}")]
    CrawlMissingMarkdown(String),
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
        request.validate_at(Utc::now())?;
        let query = request.query.trim();
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

        let results = response.results.into_iter().take(limit).collect::<Vec<_>>();
        for result in &results {
            let url = reqwest::Url::parse(&result.url)
                .map_err(|_| ResearchError::InvalidSourceUrl(result.url.clone()))?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                return Err(ResearchError::InvalidSourceUrl(result.url.clone()));
            }
        }

        Ok(ResearchReport {
            query: query.to_string(),
            provider: "searxng".to_string(),
            limit,
            source_policy: request.source_policy.clone(),
            expires_at: request.expires_at.expect("validated request expiry"),
            results,
        })
    }

    /// Fetch the canonical markdown content of a URL through Crawl4AI so the
    /// content hash recorded in the research observation matches what Varda's
    /// external-lane import handler will re-fetch and verify.
    pub async fn crawl_canonical_content(
        &self,
        url: &str,
        filter: &str,
    ) -> Result<String, ResearchError> {
        let endpoint = format!("{}/md", self.endpoint.as_str().trim_end_matches('/'));
        let markdown: serde_json::Value = self
            .client
            .post(&endpoint)
            .json(&serde_json::json!({
                "url": url,
                "f": filter,
                "c": "0"
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        markdown
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| ResearchError::CrawlMissingMarkdown(url.to_string()))
    }
}
