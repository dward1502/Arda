use crate::OutpostObservation;
use arda_vaire::service::RecallRecentEntry;
use arda_vaire::{InformantEvent, MnemosyneService};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScoutMemoryError {
    #[error("missing vaire service")]
    Service,
    #[error("invalid ARDA root: {0}")]
    InvalidRoot(String),
    #[error("observation serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type MemoryResult<T> = std::result::Result<T, ScoutMemoryError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryFallback {
    pub scope: String,
    pub source_crate: String,
    pub content: String,
    pub missing_root: bool,
    pub failure_reason: String,
    pub observed_at: DateTime<Utc>,
    pub suggested_event_type: String,
    pub credentials: Vec<CredentialProposal>,
    /// The append-only Mnemosyne record is the ingestion receipt. Scout
    /// observations are never promoted into semantic/procedural authority here.
    pub memory_id: Option<String>,
    pub confidence: Option<f64>,
    pub trust: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CredentialProposal {
    pub key: String,
    pub hint: String,
    pub scope: String,
    pub notes: Vec<String>,
    pub unlock_code: UnlockCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnlockCode {
    pub code_name: String,
    pub code_verifier: Option<String>,
    pub expected_prefix: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScoutRecallStatus {
    Available,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ScoutRecallQuery {
    pub hours: i64,
    pub scope: Option<String>,
    /// Exact crate or app name from the observation payload.
    pub name: Option<String>,
    pub path: Option<String>,
    pub query: Option<String>,
    pub limit: usize,
    /// Optional consumer freshness requirement. Matching records older than
    /// this remain visible but are marked stale.
    pub max_age_seconds: Option<u64>,
}

impl Default for ScoutRecallQuery {
    fn default() -> Self {
        Self {
            hours: 24,
            scope: None,
            name: None,
            path: None,
            query: None,
            limit: 20,
            max_age_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecalledScoutObservation {
    pub memory_id: String,
    pub observation: OutpostObservation,
    pub significance: f64,
    pub confidence: f64,
    pub trust: f64,
    pub age_seconds: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoutRecallReport {
    pub status: ScoutRecallStatus,
    pub records: Vec<RecalledScoutObservation>,
    pub warning: Option<String>,
}

impl MemoryFallback {
    pub fn new(
        scope: impl Into<String>,
        content: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            scope: scope.into(),
            source_crate: "arda-outpost-scout".to_string(),
            content: content.into(),
            missing_root: false,
            failure_reason: reason.into(),
            observed_at: Utc::now(),
            suggested_event_type: "scout_survey".to_string(),
            credentials: Vec::new(),
            memory_id: None,
            confidence: None,
            trust: None,
        }
    }

    pub fn with_missing_root(mut self, missing_root: bool) -> Self {
        self.missing_root = missing_root;
        self
    }

    pub fn with_credentials(mut self, credentials: Vec<CredentialProposal>) -> Self {
        self.credentials = credentials;
        self
    }

    pub fn with_suggested_event_type(mut self, suggested_event_type: impl Into<String>) -> Self {
        self.suggested_event_type = suggested_event_type.into();
        self
    }

    fn with_encoded_entry(mut self, entry: &RecallRecentEntry) -> Self {
        self.memory_id = Some(entry.memory_id.clone());
        self.confidence = Some(entry.confidence);
        self.trust = Some(entry.trust);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ObservationMemoryBridge {
    scope: String,
    primary_root: Option<PathBuf>,
    fallback_root: Option<PathBuf>,
    credentials: Vec<CredentialProposal>,
}

impl ObservationMemoryBridge {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            primary_root: nonempty_env_path("ARDA_ROOT")
                .map(|root| root.join("data").join("mnemosyne")),
            fallback_root: nonempty_env_path("SCOUT_MEMORY_FALLBACK_ROOT"),
            credentials: default_credentials(),
        }
    }

    /// Construct an environment-independent bridge when the memory root is known.
    pub fn at_root(scope: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            scope: scope.into(),
            primary_root: Some(root.into()),
            fallback_root: None,
            credentials: default_credentials(),
        }
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.primary_root = Some(root.into());
        self
    }

    pub fn with_fallback_root(mut self, fallback_root: impl Into<PathBuf>) -> Self {
        self.fallback_root = Some(fallback_root.into());
        self
    }

    pub fn with_credentials(mut self, credentials: Vec<CredentialProposal>) -> Self {
        self.credentials = credentials;
        self
    }

    pub fn encode_observation_to_memory(
        &self,
        observation: &OutpostObservation,
    ) -> MemoryResult<MemoryFallback> {
        let content = serde_json::to_string(observation)?;
        if observation.schema_version != arda_outpost_protocol::SCHEMA_VERSION {
            return Ok(MemoryFallback::new(
                self.scope.clone(),
                content,
                format!(
                    "refused schema mismatch: expected {}, got {}",
                    arda_outpost_protocol::SCHEMA_VERSION,
                    observation.schema_version
                ),
            )
            .with_suggested_event_type("outpost_observation")
            .with_credentials(self.credentials.clone()));
        }

        let service = match self.open_service() {
            Ok(service) => service,
            Err(reason) => {
                return Ok(MemoryFallback::new(self.scope.clone(), content, reason)
                    .with_missing_root(true)
                    .with_suggested_event_type("outpost_observation")
                    .with_credentials(self.credentials.clone()));
            }
        };
        match service.encode(bridge_event(&self.scope, observation)) {
            Ok(Some(encoded)) => {
                Ok(
                    MemoryFallback::new(encoded.memory_scope.clone(), content, "encoded")
                        .with_suggested_event_type("outpost_observation")
                        .with_credentials(self.credentials.clone())
                        .with_encoded_entry(&encoded),
                )
            }
            Ok(None) => Ok(MemoryFallback::new(
                self.scope.clone(),
                content,
                "encode classified observation as noise; no memory record written",
            )
            .with_suggested_event_type("outpost_observation")
            .with_credentials(self.credentials.clone())),
            Err(err) => Ok(
                MemoryFallback::new(self.scope.clone(), content, err.to_string())
                    .with_suggested_event_type("outpost_observation")
                    .with_credentials(self.credentials.clone()),
            ),
        }
    }

    /// Compatibility read for callers that need raw Mnemosyne records.
    pub fn recall_recent_observations(&self, hours: i64) -> MemoryResult<Vec<RecallRecentEntry>> {
        let service = self.open_service().map_err(ScoutMemoryError::InvalidRoot)?;
        service
            .recall_recent_scoped(hours, Some("arda-outpost-scout"), Some(&self.scope))
            .map_err(|err| ScoutMemoryError::InvalidRoot(err.to_string()))
    }

    /// Consumer-facing P3 read path. It supports scoped path/query filtering,
    /// carries confidence/trust metadata, and degrades instead of failing when
    /// memory is stale or unavailable.
    pub fn recall_observations(&self, query: &ScoutRecallQuery) -> ScoutRecallReport {
        let service = match self.open_service() {
            Ok(service) => service,
            Err(reason) => return unavailable_report(reason),
        };
        let entries = match service.recall_recent_scoped(
            query.hours.max(1),
            Some("arda-outpost-scout"),
            Some(&self.scope),
        ) {
            Ok(entries) => entries,
            Err(err) => return unavailable_report(err.to_string()),
        };

        let now = Utc::now();
        let records = entries
            .into_iter()
            .filter_map(|entry| {
                let observation =
                    serde_json::from_str::<OutpostObservation>(&entry.content).ok()?;
                if !matches_scope(&observation, query.scope.as_deref())
                    || !matches_name(&observation, query.name.as_deref())
                    || !matches_path(&observation, query.path.as_deref())
                    || !matches_query(&observation, query.query.as_deref())
                {
                    return None;
                }
                let age_seconds = now
                    .signed_duration_since(observation.observed_at)
                    .num_seconds()
                    .max(0) as u64;
                let stale = query
                    .max_age_seconds
                    .map(|max_age| age_seconds > max_age)
                    .unwrap_or(false);
                Some(RecalledScoutObservation {
                    memory_id: entry.memory_id,
                    observation,
                    significance: entry.significance,
                    confidence: entry.confidence,
                    trust: entry.trust,
                    age_seconds,
                    stale,
                })
            })
            .take(query.limit.max(1))
            .collect::<Vec<_>>();

        let status = if !records.is_empty() && records.iter().all(|record| record.stale) {
            ScoutRecallStatus::Stale
        } else {
            ScoutRecallStatus::Available
        };
        let warning = (status == ScoutRecallStatus::Stale).then(|| {
            "all matching scout observations exceed the requested freshness bound".to_string()
        });
        ScoutRecallReport {
            status,
            records,
            warning,
        }
    }

    fn open_service(&self) -> std::result::Result<MnemosyneService, String> {
        let mut failures = Vec::new();
        let mut attempted = Vec::<&Path>::new();
        for root in [&self.primary_root, &self.fallback_root]
            .into_iter()
            .flatten()
            .map(PathBuf::as_path)
        {
            if attempted.contains(&root) {
                continue;
            }
            attempted.push(root);
            match MnemosyneService::new(root) {
                Ok(service) => return Ok(service),
                Err(err) => failures.push(format!("{}: {err}", root.display())),
            }
        }
        if failures.is_empty() {
            Err("no ARDA_ROOT or SCOUT_MEMORY_FALLBACK_ROOT configured".to_string())
        } else {
            Err(failures.join("; "))
        }
    }
}

fn bridge_event(scope: &str, observation: &OutpostObservation) -> InformantEvent {
    let classification = match observation.classification {
        arda_outpost_protocol::ObservationClassification::RawMeasurement => "raw_measurement",
        arda_outpost_protocol::ObservationClassification::DerivedEstimate => "derived_estimate",
        arda_outpost_protocol::ObservationClassification::SelfReport => "self_report",
        arda_outpost_protocol::ObservationClassification::Default => "default",
        arda_outpost_protocol::ObservationClassification::Unavailable => "unavailable",
        arda_outpost_protocol::ObservationClassification::ExperimentalDerived => {
            "experimental_derived"
        }
    };

    let tags = match &observation.scope {
        arda_outpost_protocol::ObservationScope::Custom(scope_tag) => vec![
            scope_tag.clone(),
            format!("scope:{scope}"),
            format!("observation_scope:{}", observation.scope),
            "observation".to_string(),
        ],
        _ => vec![
            format!("scope:{scope}"),
            format!("observation_scope:{}", observation.scope),
            "observation".to_string(),
        ],
    };

    InformantEvent {
        informant_id: format!(
            "arda-outpost-scout://{}",
            observation.source.trim().replace('/', "-")
        ),
        crate_name: "arda-outpost-scout".to_string(),
        event_type: classification.to_string(),
        ts_utc: observation.observed_at.to_rfc3339(),
        content: serde_json::to_string(observation)
            .expect("OutpostObservation serialization is infallible"),
        confidence_hint: Some(observation.confidence.into()),
        tags: observation_tags(observation, tags),
    }
}

fn observation_tags(observation: &OutpostObservation, mut tags: Vec<String>) -> Vec<String> {
    tags.push(format!("observation_id:{}", observation.id));
    tags.push(format!("authority:{}", observation.authority));
    tags.push(format!("classification:{}", observation.classification));
    if let Some(path) = observation
        .payload
        .get("path")
        .and_then(|value| value.as_str())
    {
        tags.push(format!("path:{path}"));
    }
    if let Some(name) = observation
        .payload
        .get("name")
        .and_then(|value| value.as_str())
    {
        tags.push(format!("name:{name}"));
    }
    tags
}

fn nonempty_env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn unavailable_report(reason: String) -> ScoutRecallReport {
    ScoutRecallReport {
        status: ScoutRecallStatus::Unavailable,
        records: Vec::new(),
        warning: Some(reason),
    }
}

fn matches_scope(observation: &OutpostObservation, expected: Option<&str>) -> bool {
    match expected {
        Some(scope) => observation.scope.to_string().eq_ignore_ascii_case(scope),
        None => true,
    }
}

fn matches_name(observation: &OutpostObservation, expected: Option<&str>) -> bool {
    match expected {
        Some(name) => observation
            .payload
            .get("name")
            .and_then(|value| value.as_str())
            .map(|candidate| candidate.eq_ignore_ascii_case(name))
            .unwrap_or(false),
        None => true,
    }
}

fn matches_path(observation: &OutpostObservation, expected: Option<&str>) -> bool {
    match expected {
        Some(path) => observation
            .payload
            .get("path")
            .and_then(|value| value.as_str())
            .map(|candidate| candidate.contains(path))
            .unwrap_or(false),
        None => true,
    }
}

fn matches_query(observation: &OutpostObservation, expected: Option<&str>) -> bool {
    match expected {
        Some(query) => serde_json::to_string(observation)
            .unwrap_or_else(|_| observation.payload.to_string())
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase()),
        None => true,
    }
}

fn default_credentials() -> Vec<CredentialProposal> {
    vec![
        CredentialProposal {
            key: "ARDA_ROOT".to_string(),
            hint: "opt-in Arda workspace root for scout memory".to_string(),
            scope: "outpost_scout".to_string(),
            notes: vec![
                "Sets the workspace root used to write bridge memory records.".to_string(),
                "Invalid or empty values fall back to advisory fallback state.".to_string(),
            ],
            unlock_code: UnlockCode {
                code_name: "scout_root_unlock".to_string(),
                code_verifier: None,
                expected_prefix: "unlock-scout-memory:".to_string(),
                notes: vec!["Prefix for bridge unlock tokens.".to_string()],
            },
        },
        CredentialProposal {
            key: "SCOUT_MEMORY_FALLBACK_ROOT".to_string(),
            hint: "fallback root when ARDA_ROOT is unavailable".to_string(),
            scope: "outpost_scout".to_string(),
            notes: vec![
                "Allows ephemeral writes without changing the active Arda root.".to_string(),
            ],
            unlock_code: UnlockCode {
                code_name: "scout_fallback_root_unlock".to_string(),
                code_verifier: None,
                expected_prefix: "unlock-scout-memory:".to_string(),
                notes: vec!["Prefix for fallback root unlock tokens.".to_string()],
            },
        },
        CredentialProposal {
            key: "PROMETHEUS_RECALL_WINDOW_HOURS".to_string(),
            hint: "recall window in hours for recent observations".to_string(),
            scope: "outpost_scout".to_string(),
            notes: vec![
                "Sets how far back the memory bridge scans for scout observations.".to_string(),
            ],
            unlock_code: UnlockCode {
                code_name: "scout_recall_window_unlock".to_string(),
                code_verifier: None,
                expected_prefix: "unlock-scout-memory:".to_string(),
                notes: vec!["Prefix for recall window unlock tokens.".to_string()],
            },
        },
    ]
}
