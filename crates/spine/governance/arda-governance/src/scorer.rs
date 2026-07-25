//! Async-first governance scoring contracts and conservative degraded receipts.

use std::{future::Future, pin::Pin, time::Duration};

#[cfg(feature = "llm-scorer")]
use std::{collections::HashMap, sync::Mutex, time::Instant};

use arda_core::Task;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct GovernanceScoreRequest {
    pub task: Task,
    pub lens_id: String,
}

impl GovernanceScoreRequest {
    pub fn new(task: Task, lens_id: impl Into<String>) -> Self {
        Self {
            task,
            lens_id: lens_id.into(),
        }
    }

    pub fn task_hash(&self) -> String {
        let payload = serde_json::to_vec(&self.task).unwrap_or_default();
        sha256_hex(&payload)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceScorerState {
    Complete,
    Degraded,
    Timeout,
    Unavailable,
    Error,
    StaleCache,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceScoreCacheStatus {
    #[default]
    NotApplicable,
    Miss,
    Hit,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceScoreReceipt {
    pub schema_version: String,
    pub lens_id: String,
    pub score: f64,
    pub state: GovernanceScorerState,
    pub scorer_id: String,
    pub provider: String,
    pub model: String,
    pub task_hash: String,
    pub provenance: String,
    pub reproducibility_limits: Vec<String>,
    #[serde(default)]
    pub cache_status: GovernanceScoreCacheStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

impl GovernanceScoreReceipt {
    pub fn complete_local(
        request: &GovernanceScoreRequest,
        scorer_id: impl Into<String>,
        score: f64,
    ) -> Self {
        Self {
            schema_version: "arda.governance.scorer_receipt.v1".to_string(),
            lens_id: request.lens_id.clone(),
            score,
            state: GovernanceScorerState::Complete,
            scorer_id: scorer_id.into(),
            provider: "local".to_string(),
            model: "structured_evidence_v2".to_string(),
            task_hash: request.task_hash(),
            provenance: "arda-governance deterministic structured evidence scorer".to_string(),
            reproducibility_limits: vec![
                "reproducible only for identical task serialization and crate policy version"
                    .to_string(),
            ],
            cache_status: GovernanceScoreCacheStatus::NotApplicable,
            diagnostic: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceScorerErrorKind {
    Unavailable,
    InvalidResponse,
    Backend,
}

#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct GovernanceScorerError {
    pub kind: GovernanceScorerErrorKind,
    pub message: String,
}

impl GovernanceScorerError {
    pub fn new(kind: GovernanceScorerErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

pub type GovernanceScoreFuture<'a> = Pin<
    Box<dyn Future<Output = Result<GovernanceScoreReceipt, GovernanceScorerError>> + Send + 'a>,
>;

pub trait GovernanceScorer: Send + Sync {
    fn scorer_id(&self) -> &str;

    fn provider_identity(&self) -> &str {
        "local"
    }

    fn model_identity(&self) -> &str {
        "structured_evidence_v2"
    }

    fn provenance(&self) -> &str {
        "arda-governance scorer"
    }

    fn reproducibility_limits(&self) -> Vec<String> {
        vec!["scorer implementation and policy version must remain identical".to_string()]
    }

    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalGovernanceScorer;

impl GovernanceScorer for LocalGovernanceScorer {
    fn scorer_id(&self) -> &str {
        "deterministic-local-v1"
    }

    fn provenance(&self) -> &str {
        "arda-governance::triad structured evidence lens functions"
    }

    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async move {
            let score =
                crate::triad::score_governance_lens_for_scorer(&request.task, &request.lens_id)
                    .ok_or_else(|| {
                        GovernanceScorerError::new(
                            GovernanceScorerErrorKind::InvalidResponse,
                            format!("unknown governance lens: {}", request.lens_id),
                        )
                    })?;
            Ok(GovernanceScoreReceipt::complete_local(
                &request,
                self.scorer_id(),
                score,
            ))
        })
    }
}

pub async fn score_governance_with_timeout(
    scorer: &dyn GovernanceScorer,
    request: GovernanceScoreRequest,
    timeout: Duration,
) -> GovernanceScoreReceipt {
    let degraded_request = request.clone();
    match tokio::time::timeout(timeout, scorer.score(request)).await {
        Ok(Ok(mut receipt)) if valid_score(receipt.score) => {
            normalize_receipt(scorer, &degraded_request, &mut receipt);
            receipt
        }
        Ok(Ok(receipt)) => degraded_receipt(
            scorer,
            &degraded_request,
            GovernanceScorerState::Error,
            format!("scorer returned invalid score: {}", receipt.score),
        ),
        Ok(Err(error)) => degraded_receipt(
            scorer,
            &degraded_request,
            match error.kind {
                GovernanceScorerErrorKind::Unavailable => GovernanceScorerState::Unavailable,
                GovernanceScorerErrorKind::InvalidResponse | GovernanceScorerErrorKind::Backend => {
                    GovernanceScorerState::Error
                }
            },
            error.to_string(),
        ),
        Err(_) => degraded_receipt(
            scorer,
            &degraded_request,
            GovernanceScorerState::Timeout,
            format!("scorer exceeded timeout of {}ms", timeout.as_millis()),
        ),
    }
}

fn degraded_receipt(
    scorer: &dyn GovernanceScorer,
    request: &GovernanceScoreRequest,
    state: GovernanceScorerState,
    diagnostic: String,
) -> GovernanceScoreReceipt {
    let mut reproducibility_limits = scorer.reproducibility_limits();
    if reproducibility_limits.is_empty() {
        reproducibility_limits.push("scorer declared no reproducibility guarantee".to_string());
    }
    GovernanceScoreReceipt {
        schema_version: "arda.governance.scorer_receipt.v1".to_string(),
        lens_id: request.lens_id.clone(),
        score: 0.0,
        state,
        scorer_id: scorer.scorer_id().to_string(),
        provider: scorer.provider_identity().to_string(),
        model: scorer.model_identity().to_string(),
        task_hash: request.task_hash(),
        provenance: scorer.provenance().to_string(),
        reproducibility_limits,
        cache_status: GovernanceScoreCacheStatus::NotApplicable,
        diagnostic: Some(diagnostic),
    }
}

fn valid_score(score: f64) -> bool {
    score.is_finite() && (0.0..=1.0).contains(&score)
}

fn normalize_receipt(
    scorer: &dyn GovernanceScorer,
    request: &GovernanceScoreRequest,
    receipt: &mut GovernanceScoreReceipt,
) {
    receipt.lens_id.clone_from(&request.lens_id);
    receipt.task_hash = request.task_hash();
    if receipt.scorer_id.trim().is_empty() {
        receipt.scorer_id = scorer.scorer_id().to_string();
    }
    if receipt.provider.trim().is_empty() {
        receipt.provider = scorer.provider_identity().to_string();
    }
    if receipt.model.trim().is_empty() {
        receipt.model = scorer.model_identity().to_string();
    }
    if receipt.provenance.trim().is_empty() {
        receipt.provenance = scorer.provenance().to_string();
    }
    if receipt.reproducibility_limits.is_empty() {
        receipt.reproducibility_limits = scorer.reproducibility_limits();
    }
    if receipt.reproducibility_limits.is_empty() {
        receipt
            .reproducibility_limits
            .push("scorer declared no reproducibility guarantee".to_string());
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(feature = "llm-scorer")]
#[derive(Debug, Clone)]
pub struct LlmGovernanceScorerConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub cache_ttl: Duration,
    pub reproducibility_limits: Vec<String>,
}

#[cfg(feature = "llm-scorer")]
#[derive(Debug, Clone, PartialEq)]
pub struct LlmScoreResponse {
    pub score: f64,
    pub provenance: String,
}

#[cfg(feature = "llm-scorer")]
pub type LlmScoreBackendFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LlmScoreResponse, GovernanceScorerError>> + Send + 'a>>;

#[cfg(feature = "llm-scorer")]
pub trait LlmScoreBackend: Send + Sync {
    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> LlmScoreBackendFuture<'a>;
}

#[cfg(feature = "llm-scorer")]
struct LlmCacheEntry {
    receipt: GovernanceScoreReceipt,
    stored_at: Instant,
}

#[cfg(feature = "llm-scorer")]
pub struct LlmGovernanceScorer<B> {
    config: LlmGovernanceScorerConfig,
    backend: B,
    cache: Mutex<HashMap<String, LlmCacheEntry>>,
}

#[cfg(feature = "llm-scorer")]
impl<B> LlmGovernanceScorer<B>
where
    B: LlmScoreBackend,
{
    pub fn new(config: LlmGovernanceScorerConfig, backend: B) -> Self {
        let mut config = config;
        if config.provider.trim().is_empty() {
            config.provider = "unconfigured-provider".to_string();
        }
        if config.model.trim().is_empty() {
            config.model = "unconfigured-model".to_string();
        }
        if config.reproducibility_limits.is_empty() {
            config.reproducibility_limits.push(
                "provider-backed scores are not guaranteed to be bit reproducible".to_string(),
            );
        }
        Self {
            config,
            backend,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn cache_key(&self, request: &GovernanceScoreRequest) -> String {
        format!(
            "{}:{}:{}:{}",
            request.task_hash(),
            request.lens_id,
            self.config.provider,
            self.config.model
        )
    }
}

#[cfg(feature = "llm-scorer")]
impl<B> GovernanceScorer for LlmGovernanceScorer<B>
where
    B: LlmScoreBackend,
{
    fn scorer_id(&self) -> &str {
        "optional-llm-v1"
    }

    fn provider_identity(&self) -> &str {
        &self.config.provider
    }

    fn model_identity(&self) -> &str {
        &self.config.model
    }

    fn provenance(&self) -> &str {
        "optional LLM governance scorer backend"
    }

    fn reproducibility_limits(&self) -> Vec<String> {
        self.config.reproducibility_limits.clone()
    }

    fn score<'a>(&'a self, request: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async move {
            if !self.config.enabled {
                return Err(GovernanceScorerError::new(
                    GovernanceScorerErrorKind::Unavailable,
                    "LLM governance scorer is disabled by configuration",
                ));
            }

            let key = self.cache_key(&request);
            let cached = {
                let mut cache = self.cache.lock().unwrap();
                match cache.get(&key) {
                    Some(entry) if entry.stored_at.elapsed() <= self.config.cache_ttl => {
                        let mut receipt = entry.receipt.clone();
                        receipt.cache_status = GovernanceScoreCacheStatus::Hit;
                        Some(receipt)
                    }
                    Some(_) => {
                        cache.remove(&key);
                        let mut receipt = degraded_receipt(
                            self,
                            &request,
                            GovernanceScorerState::StaleCache,
                            "cached LLM score exceeded configured TTL".to_string(),
                        );
                        receipt.cache_status = GovernanceScoreCacheStatus::Stale;
                        Some(receipt)
                    }
                    None => None,
                }
            };
            if let Some(receipt) = cached {
                return Ok(receipt);
            }

            let response = self.backend.score(request.clone()).await?;
            if !valid_score(response.score) {
                return Err(GovernanceScorerError::new(
                    GovernanceScorerErrorKind::InvalidResponse,
                    format!("LLM backend returned invalid score: {}", response.score),
                ));
            }
            let receipt = GovernanceScoreReceipt {
                schema_version: "arda.governance.scorer_receipt.v1".to_string(),
                lens_id: request.lens_id.clone(),
                score: response.score,
                state: GovernanceScorerState::Complete,
                scorer_id: self.scorer_id().to_string(),
                provider: self.config.provider.clone(),
                model: self.config.model.clone(),
                task_hash: request.task_hash(),
                provenance: response.provenance,
                reproducibility_limits: self.config.reproducibility_limits.clone(),
                cache_status: GovernanceScoreCacheStatus::Miss,
                diagnostic: None,
            };
            self.cache.lock().unwrap().insert(
                key,
                LlmCacheEntry {
                    receipt: receipt.clone(),
                    stored_at: Instant::now(),
                },
            );
            Ok(receipt)
        })
    }
}
