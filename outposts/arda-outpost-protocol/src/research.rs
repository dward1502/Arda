//! Versioned, advisory-only Warden research receipt contracts.

use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

pub const RESEARCH_SCHEMA_VERSION: &str = "arda.warden.research.v1";
pub const ADVISORY_RESEARCH_AUTHORITY: &str = "advisory_only";

#[derive(Debug, thiserror::Error)]
pub enum ResearchReceiptError {
    #[error("research receipt has invalid {0}")]
    InvalidField(&'static str),
    #[error("research suggestion expired")]
    Expired,
    #[error("research receipt parent mismatch")]
    ParentMismatch,
    #[error("research URL must be HTTP(S) with a host")]
    InvalidUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DispatchDisposition {
    Accepted,
    Rejected,
    Expired,
    Failed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchSuggestion {
    pub schema_version: String,
    pub suggestion_id: String,
    pub idempotency_key: String,
    pub created_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub query: String,
    pub max_results: usize,
    pub budget_bytes: usize,
    pub authority: String,
}

impl ResearchSuggestion {
    pub fn new(
        query: impl Into<String>,
        idempotency_key: impl Into<String>,
        created_at_utc: DateTime<Utc>,
        expires_at_utc: DateTime<Utc>,
        max_results: usize,
        budget_bytes: usize,
    ) -> Result<Self, ResearchReceiptError> {
        let query = query.into();
        let idempotency_key = idempotency_key.into();
        if query.trim().is_empty()
            || idempotency_key.trim().is_empty()
            || max_results == 0
            || budget_bytes == 0
        {
            return Err(ResearchReceiptError::InvalidField("suggestion"));
        }
        if expires_at_utc <= created_at_utc {
            return Err(ResearchReceiptError::InvalidField("expiry"));
        }
        Ok(Self {
            schema_version: RESEARCH_SCHEMA_VERSION.to_owned(),
            suggestion_id: Uuid::new_v4().to_string(),
            idempotency_key,
            created_at_utc,
            expires_at_utc,
            query,
            max_results,
            budget_bytes,
            authority: ADVISORY_RESEARCH_AUTHORITY.to_owned(),
        })
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), ResearchReceiptError> {
        if self.schema_version != RESEARCH_SCHEMA_VERSION
            || self.authority != ADVISORY_RESEARCH_AUTHORITY
        {
            return Err(ResearchReceiptError::InvalidField("authority_or_schema"));
        }
        if self.expires_at_utc <= now {
            return Err(ResearchReceiptError::Expired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchDispatch {
    pub schema_version: String,
    pub dispatch_id: String,
    pub idempotency_key: String,
    pub suggestion_id: String,
    pub dispatched_at_utc: DateTime<Utc>,
    pub max_attempts: usize,
    pub disposition: DispatchDisposition,
    pub authority: String,
}

impl ResearchDispatch {
    pub fn accepted(
        suggestion: &ResearchSuggestion,
        idempotency_key: impl Into<String>,
        dispatched_at_utc: DateTime<Utc>,
        max_attempts: usize,
    ) -> Result<Self, ResearchReceiptError> {
        let idempotency_key = idempotency_key.into();
        if max_attempts == 0 || idempotency_key.trim().is_empty() {
            return Err(ResearchReceiptError::InvalidField("dispatch"));
        }
        Ok(Self {
            schema_version: RESEARCH_SCHEMA_VERSION.to_owned(),
            dispatch_id: Uuid::new_v4().to_string(),
            idempotency_key,
            suggestion_id: suggestion.suggestion_id.clone(),
            dispatched_at_utc,
            max_attempts,
            disposition: DispatchDisposition::Accepted,
            authority: ADVISORY_RESEARCH_AUTHORITY.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalObservationReceipt {
    pub schema_version: String,
    pub observation_id: String,
    pub suggestion_id: String,
    pub dispatch_id: String,
    pub observed_at_utc: DateTime<Utc>,
    pub normalized_url: String,
    pub content_hash: String,
    pub provenance_hash: String,
    pub authority: String,
}

impl ExternalObservationReceipt {
    pub fn completed(
        suggestion: &ResearchSuggestion,
        dispatch: &ResearchDispatch,
        url: &str,
        content_hash: String,
        provenance_hash: String,
        observed_at_utc: DateTime<Utc>,
    ) -> Result<Self, ResearchReceiptError> {
        if dispatch.suggestion_id != suggestion.suggestion_id {
            return Err(ResearchReceiptError::ParentMismatch);
        }
        if !valid_hash(&content_hash) || !valid_hash(&provenance_hash) {
            return Err(ResearchReceiptError::InvalidField("hash"));
        }
        Ok(Self {
            schema_version: RESEARCH_SCHEMA_VERSION.to_owned(),
            observation_id: Uuid::new_v4().to_string(),
            suggestion_id: suggestion.suggestion_id.clone(),
            dispatch_id: dispatch.dispatch_id.clone(),
            observed_at_utc,
            normalized_url: normalize_http_url(url)?,
            content_hash,
            provenance_hash,
            authority: ADVISORY_RESEARCH_AUTHORITY.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcknowledgementReceipt {
    pub schema_version: String,
    pub acknowledgement_id: String,
    pub suggestion_id: String,
    pub dispatch_id: String,
    pub observation_id: String,
    pub acknowledged_at_utc: DateTime<Utc>,
    pub disposition: DispatchDisposition,
    pub authority: String,
}

impl AcknowledgementReceipt {
    pub fn completed(
        suggestion: &ResearchSuggestion,
        dispatch: &ResearchDispatch,
        observation: &ExternalObservationReceipt,
        acknowledged_at_utc: DateTime<Utc>,
    ) -> Result<Self, ResearchReceiptError> {
        if observation.suggestion_id != suggestion.suggestion_id
            || observation.dispatch_id != dispatch.dispatch_id
        {
            return Err(ResearchReceiptError::ParentMismatch);
        }
        Ok(Self {
            schema_version: RESEARCH_SCHEMA_VERSION.to_owned(),
            acknowledgement_id: Uuid::new_v4().to_string(),
            suggestion_id: suggestion.suggestion_id.clone(),
            dispatch_id: dispatch.dispatch_id.clone(),
            observation_id: observation.observation_id.clone(),
            acknowledged_at_utc,
            disposition: DispatchDisposition::Completed,
            authority: ADVISORY_RESEARCH_AUTHORITY.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedResearchChain {
    pub suggestion: ResearchSuggestion,
    pub dispatch: ResearchDispatch,
    pub observation: ExternalObservationReceipt,
    pub acknowledgement: AcknowledgementReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchCursor {
    pub stream: String,
    pub sequence: u64,
    pub last_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResearchReceiptLedger {
    path: PathBuf,
    cursor_path: PathBuf,
}

/// Durable, append-only ingress for Aulë/Watchlist suggestions.
///
/// Queue records are suggestions before a Warden dispatch exists, while the
/// complete-chain ledger is only published after observation and
/// acknowledgement validation.
#[derive(Debug, Clone)]
pub struct ResearchSuggestionLedger {
    path: PathBuf,
    cursor_path: PathBuf,
}

impl ResearchReceiptLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ResearchReceiptError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| ResearchReceiptError::InvalidField("ledger_directory"))?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| ResearchReceiptError::InvalidField("ledger"))?;
        let cursor_path = path.with_extension("cursor.json");
        Ok(Self { path, cursor_path })
    }

    pub fn complete_chains(&self) -> Result<Vec<PersistedResearchChain>, ResearchReceiptError> {
        fs::read_to_string(&self.path)
            .map_err(|_| ResearchReceiptError::InvalidField("ledger"))?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line)
                    .map_err(|_| ResearchReceiptError::InvalidField("ledger_record"))
            })
            .collect()
    }

    pub fn append_complete_chain(
        &self,
        suggestion: &ResearchSuggestion,
        dispatch: &ResearchDispatch,
        observation: &ExternalObservationReceipt,
        acknowledgement: &AcknowledgementReceipt,
        now: DateTime<Utc>,
    ) -> Result<PersistedResearchChain, ResearchReceiptError> {
        validate_research_chain(suggestion, dispatch, observation, acknowledgement, now)?;
        if let Some(existing) = self.complete_chains()?.into_iter().find(|record| {
            record.suggestion.idempotency_key == suggestion.idempotency_key
                || record.dispatch.idempotency_key == dispatch.idempotency_key
        }) {
            if existing.observation.content_hash == observation.content_hash
                && existing.observation.provenance_hash == observation.provenance_hash
            {
                return Ok(existing);
            }
            return Err(ResearchReceiptError::InvalidField(
                "duplicate_idempotency_key",
            ));
        }
        let record = PersistedResearchChain {
            suggestion: suggestion.clone(),
            dispatch: dispatch.clone(),
            observation: observation.clone(),
            acknowledgement: acknowledgement.clone(),
        };
        let encoded = serde_json::to_string(&record)
            .map_err(|_| ResearchReceiptError::InvalidField("ledger_record"))?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|_| ResearchReceiptError::InvalidField("ledger"))?;
        file.write_all(encoded.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_data())
            .map_err(|_| ResearchReceiptError::InvalidField("ledger"))?;
        Ok(record)
    }

    pub fn read_cursor(&self, stream: &str) -> Result<ResearchCursor, ResearchReceiptError> {
        if stream.trim().is_empty() {
            return Err(ResearchReceiptError::InvalidField("cursor_stream"));
        }
        match fs::read_to_string(&self.cursor_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|_| ResearchReceiptError::InvalidField("cursor")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ResearchCursor {
                stream: stream.to_owned(),
                sequence: 0,
                last_id: None,
            }),
            Err(_) => Err(ResearchReceiptError::InvalidField("cursor")),
        }
    }

    pub fn advance_cursor(
        &self,
        stream: &str,
        sequence: u64,
        last_id: impl Into<String>,
    ) -> Result<(), ResearchReceiptError> {
        let current = self.read_cursor(stream)?;
        let last_id = last_id.into();
        if sequence <= current.sequence || last_id.trim().is_empty() {
            return Err(ResearchReceiptError::InvalidField("cursor_regression"));
        }
        let next = ResearchCursor {
            stream: stream.to_owned(),
            sequence,
            last_id: Some(last_id),
        };
        let temp_path = self.cursor_path.with_extension("cursor.json.tmp");
        let encoded =
            serde_json::to_vec(&next).map_err(|_| ResearchReceiptError::InvalidField("cursor"))?;
        fs::write(&temp_path, encoded).map_err(|_| ResearchReceiptError::InvalidField("cursor"))?;
        fs::rename(temp_path, &self.cursor_path)
            .map_err(|_| ResearchReceiptError::InvalidField("cursor"))
    }
}

impl ResearchSuggestionLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ResearchReceiptError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| ResearchReceiptError::InvalidField("ledger_directory"))?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| ResearchReceiptError::InvalidField("suggestion_ledger"))?;
        Ok(Self {
            cursor_path: path.with_extension("cursor.json"),
            path,
        })
    }

    pub fn suggestions(&self) -> Result<Vec<ResearchSuggestion>, ResearchReceiptError> {
        let contents = fs::read_to_string(&self.path)
            .map_err(|_| ResearchReceiptError::InvalidField("suggestion_ledger"))?;
        contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line)
                    .map_err(|_| ResearchReceiptError::InvalidField("suggestion_record"))
            })
            .collect()
    }

    pub fn append(
        &self,
        suggestion: &ResearchSuggestion,
    ) -> Result<ResearchSuggestion, ResearchReceiptError> {
        suggestion.validate_at(suggestion.created_at_utc)?;
        if let Some(existing) = self
            .suggestions()?
            .into_iter()
            .find(|item| item.idempotency_key == suggestion.idempotency_key)
        {
            return Ok(existing);
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|_| ResearchReceiptError::InvalidField("suggestion_ledger"))?;
        serde_json::to_writer(&mut file, suggestion)
            .map_err(|_| ResearchReceiptError::InvalidField("suggestion_record"))?;
        file.write_all(b"\n")
            .and_then(|_| file.sync_data())
            .map_err(|_| ResearchReceiptError::InvalidField("suggestion_ledger"))?;
        Ok(suggestion.clone())
    }

    pub fn read_cursor(&self, stream: &str) -> Result<ResearchCursor, ResearchReceiptError> {
        if stream.trim().is_empty() {
            return Err(ResearchReceiptError::InvalidField("cursor_stream"));
        }
        match fs::read_to_string(&self.cursor_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|_| ResearchReceiptError::InvalidField("cursor")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ResearchCursor {
                stream: stream.to_owned(),
                sequence: 0,
                last_id: None,
            }),
            Err(_) => Err(ResearchReceiptError::InvalidField("cursor")),
        }
    }

    pub fn advance_cursor(
        &self,
        stream: &str,
        sequence: u64,
        last_id: impl Into<String>,
    ) -> Result<(), ResearchReceiptError> {
        let current = self.read_cursor(stream)?;
        let last_id = last_id.into();
        if sequence <= current.sequence || last_id.trim().is_empty() {
            return Err(ResearchReceiptError::InvalidField("cursor_regression"));
        }
        let next = ResearchCursor {
            stream: stream.to_owned(),
            sequence,
            last_id: Some(last_id),
        };
        let temp_path = self.cursor_path.with_extension("cursor.json.tmp");
        let encoded =
            serde_json::to_vec(&next).map_err(|_| ResearchReceiptError::InvalidField("cursor"))?;
        fs::write(&temp_path, encoded).map_err(|_| ResearchReceiptError::InvalidField("cursor"))?;
        fs::rename(temp_path, &self.cursor_path)
            .map_err(|_| ResearchReceiptError::InvalidField("cursor"))
    }
}

pub fn validate_research_chain(
    suggestion: &ResearchSuggestion,
    dispatch: &ResearchDispatch,
    observation: &ExternalObservationReceipt,
    acknowledgement: &AcknowledgementReceipt,
    now: DateTime<Utc>,
) -> Result<(), ResearchReceiptError> {
    suggestion.validate_at(now)?;
    if dispatch.schema_version != RESEARCH_SCHEMA_VERSION
        || observation.schema_version != RESEARCH_SCHEMA_VERSION
        || acknowledgement.schema_version != RESEARCH_SCHEMA_VERSION
        || dispatch.authority != ADVISORY_RESEARCH_AUTHORITY
        || observation.authority != ADVISORY_RESEARCH_AUTHORITY
        || acknowledgement.authority != ADVISORY_RESEARCH_AUTHORITY
        || dispatch.suggestion_id != suggestion.suggestion_id
        || observation.suggestion_id != suggestion.suggestion_id
        || observation.dispatch_id != dispatch.dispatch_id
        || acknowledgement.suggestion_id != suggestion.suggestion_id
        || acknowledgement.dispatch_id != dispatch.dispatch_id
        || acknowledgement.observation_id != observation.observation_id
    {
        return Err(ResearchReceiptError::ParentMismatch);
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalize_http_url(input: &str) -> Result<String, ResearchReceiptError> {
    let mut url = Url::parse(input).map_err(|_| ResearchReceiptError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ResearchReceiptError::InvalidUrl);
    }
    url.set_fragment(None);
    let mut pairs = url.query_pairs().into_owned().collect::<Vec<_>>();
    pairs.sort();
    if pairs.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(
            pairs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
    }
    Ok(url.to_string())
}
