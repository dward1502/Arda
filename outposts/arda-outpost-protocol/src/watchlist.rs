//! Product-owned research question/watchlist contracts.
//!
//! These records describe product intent and lifecycle only. Warden dispatch,
//! observation, Varda evaluation, Vairë knowledge, and Aulë proposal receipts
//! remain owned by the governed backend contracts in [`crate::research`].

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const WATCHLIST_SCHEMA_VERSION: &str = "arda.warden.watchlist.v1";

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum WatchlistError {
    #[error("watchlist field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("watchlist has expired")]
    Expired,
    #[error("watchlist is retired")]
    Retired,
    #[error("watchlist lifecycle transition is invalid")]
    InvalidTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchlistState {
    Enabled,
    Paused,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WatchlistCadence {
    Manual,
    Interval { every_seconds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WatchlistSourcePolicy {
    pub policy_id: String,
    pub allowed_sources: Vec<String>,
    pub max_sources_per_run: usize,
    pub allow_private_targets: bool,
}

impl WatchlistSourcePolicy {
    fn validate(&self) -> Result<(), WatchlistError> {
        if self.policy_id.trim().is_empty()
            || self.allowed_sources.is_empty()
            || self.max_sources_per_run == 0
            || self
                .allowed_sources
                .iter()
                .any(|source| source.trim().is_empty())
        {
            return Err(WatchlistError::InvalidField("source_policy"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WatchlistEvidenceRequirements {
    pub minimum_canonical_sources: usize,
    pub require_canonical_fetch: bool,
    pub max_source_age_seconds: u64,
}

impl WatchlistEvidenceRequirements {
    fn validate(&self) -> Result<(), WatchlistError> {
        if self.minimum_canonical_sources == 0 || self.max_source_age_seconds == 0 {
            return Err(WatchlistError::InvalidField("evidence_requirements"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionPolicy {
    RequireDisclosure,
    BlockApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WatchlistBudgets {
    pub max_results: usize,
    pub max_fetch_bytes: usize,
    pub max_tokens: usize,
    pub max_attempts: usize,
}

impl WatchlistBudgets {
    fn validate(&self) -> Result<(), WatchlistError> {
        if self.max_results == 0
            || self.max_fetch_bytes == 0
            || self.max_tokens == 0
            || self.max_attempts == 0
        {
            return Err(WatchlistError::InvalidField("budgets"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WatchlistNotificationPolicy {
    pub enabled: bool,
    pub destination: Option<String>,
}

impl WatchlistNotificationPolicy {
    fn validate(&self) -> Result<(), WatchlistError> {
        if self.enabled
            && self
                .destination
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(WatchlistError::InvalidField("notification_policy"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResearchQuestion {
    pub schema_version: String,
    pub question_id: String,
    pub owner: String,
    pub question: String,
    pub rationale: String,
    pub tags: Vec<String>,
    pub cadence: WatchlistCadence,
    pub expires_at_utc: DateTime<Utc>,
    pub source_policy: WatchlistSourcePolicy,
    pub evidence_requirements: WatchlistEvidenceRequirements,
    pub contradiction_policy: ContradictionPolicy,
    pub budgets: WatchlistBudgets,
    pub notification_policy: WatchlistNotificationPolicy,
    pub state: WatchlistState,
    /// IDs returned by the governed backend; this product contract creates none.
    pub backend_suggestion_ids: Vec<String>,
}

impl ResearchQuestion {
    pub fn new(
        owner: impl Into<String>,
        question: impl Into<String>,
        rationale: impl Into<String>,
        tags: Vec<String>,
        cadence: WatchlistCadence,
        expires_at_utc: DateTime<Utc>,
        source_policy: WatchlistSourcePolicy,
        evidence_requirements: WatchlistEvidenceRequirements,
        contradiction_policy: ContradictionPolicy,
        budgets: WatchlistBudgets,
        notification_policy: WatchlistNotificationPolicy,
    ) -> Result<Self, WatchlistError> {
        let question_record = Self {
            schema_version: WATCHLIST_SCHEMA_VERSION.to_owned(),
            question_id: Uuid::new_v4().to_string(),
            owner: owner.into(),
            question: question.into(),
            rationale: rationale.into(),
            tags,
            cadence,
            expires_at_utc,
            source_policy,
            evidence_requirements,
            contradiction_policy,
            budgets,
            notification_policy,
            state: WatchlistState::Enabled,
            backend_suggestion_ids: Vec::new(),
        };
        question_record.validate_at(Utc::now())?;
        Ok(question_record)
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), WatchlistError> {
        if self.schema_version != WATCHLIST_SCHEMA_VERSION
            || self.question_id.trim().is_empty()
            || self.owner.trim().is_empty()
            || self.question.trim().is_empty()
            || self.rationale.trim().is_empty()
            || self.tags.iter().any(|tag| tag.trim().is_empty())
        {
            return Err(WatchlistError::InvalidField("identity"));
        }
        match self.cadence {
            WatchlistCadence::Manual => {}
            WatchlistCadence::Interval { every_seconds } if every_seconds > 0 => {}
            WatchlistCadence::Interval { .. } => {
                return Err(WatchlistError::InvalidField("cadence"));
            }
        }
        self.source_policy.validate()?;
        self.evidence_requirements.validate()?;
        self.budgets.validate()?;
        self.notification_policy.validate()?;
        if self.state == WatchlistState::Retired {
            return Err(WatchlistError::Retired);
        }
        if self.expires_at_utc <= now {
            return Err(WatchlistError::Expired);
        }
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), WatchlistError> {
        if self.state == WatchlistState::Retired {
            return Err(WatchlistError::InvalidTransition);
        }
        self.state = WatchlistState::Paused;
        Ok(())
    }

    pub fn resume(&mut self, now: DateTime<Utc>) -> Result<(), WatchlistError> {
        if self.state == WatchlistState::Retired {
            return Err(WatchlistError::InvalidTransition);
        }
        if self.expires_at_utc <= now {
            return Err(WatchlistError::Expired);
        }
        self.state = WatchlistState::Enabled;
        Ok(())
    }

    pub fn retire(&mut self) {
        self.state = WatchlistState::Retired;
    }

    pub fn with_expiry_after(mut self, duration: Duration) -> Self {
        self.expires_at_utc = Utc::now() + duration;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResearchWatchlist {
    pub schema_version: String,
    pub watchlist_id: String,
    pub name: String,
    pub question_ids: Vec<String>,
    pub state: WatchlistState,
}

impl ResearchWatchlist {
    pub fn new(name: impl Into<String>, question_ids: Vec<String>) -> Result<Self, WatchlistError> {
        let name = name.into();
        if name.trim().is_empty()
            || question_ids.is_empty()
            || question_ids.iter().any(|id| id.trim().is_empty())
        {
            return Err(WatchlistError::InvalidField("watchlist"));
        }
        Ok(Self {
            schema_version: WATCHLIST_SCHEMA_VERSION.to_owned(),
            watchlist_id: Uuid::new_v4().to_string(),
            name,
            question_ids,
            state: WatchlistState::Enabled,
        })
    }

    pub fn pause(&mut self) -> Result<(), WatchlistError> {
        if self.state == WatchlistState::Retired {
            return Err(WatchlistError::InvalidTransition);
        }
        self.state = WatchlistState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), WatchlistError> {
        if self.state == WatchlistState::Retired {
            return Err(WatchlistError::InvalidTransition);
        }
        self.state = WatchlistState::Enabled;
        Ok(())
    }

    pub fn retire(&mut self) {
        self.state = WatchlistState::Retired;
    }
}
