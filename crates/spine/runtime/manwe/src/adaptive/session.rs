// sigil: REPAIR
// Sessions and bounded history for adaptive route selection.
//
// This module tracks recent providers per session key, last successes, and
// last failure buckets. All history structures are bounded so they do not
// grow without limit.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, SystemTime};

use crate::adaptive::candidate::{ProviderId, RouteCandidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionKey {
    pub agent_id: &'static str,
    pub task_type: &'static str,
    pub priority: &'static str,
}

impl Default for SessionKey {
    fn default() -> Self {
        Self {
            agent_id: "default",
            task_type: "default",
            priority: "normal",
        }
    }
}

impl SessionKey {
    pub const fn new(agent_id: &'static str, task_type: &'static str, priority: &'static str) -> Self {
        Self {
            agent_id,
            task_type,
            priority,
        }
    }

    pub fn as_stable_key(&self) -> String {
        format!(
            "{}::{}::{}",
            self.agent_id, self.task_type, self.priority
        )
    }
}

impl PartialOrd for SessionKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SessionKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_stable_key().cmp(&other.as_stable_key())
    }
}

impl std::hash::Hash for SessionKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_stable_key().hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct ProviderSessionRecord {
    pub provider_id: ProviderId,
    pub first_seen_utc: String,
    pub last_seen_utc: String,
    pub successes: u32,
    pub failures: u32,
    pub last_failure_reason: Option<String>,
    pub last_success_utc: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionHistoryEntry {
    pub route_candidate_key: String,
    pub outcome: SessionOutcome,
    pub latency_ms: Option<u64>,
    pub occurred_utc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SessionOutcome {
    #[default]
    Success,
    Failure,
    Fallback,
}

#[derive(Debug, Clone, Default)]
pub struct BoundedSessionHistory {
    pub entries: VecDeque<SessionHistoryEntry>,
    pub max_entries: usize,
}

impl BoundedSessionHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries: max_entries.max(1),
        }
    }

    pub fn push(&mut self, entry: SessionHistoryEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn recent_failures(&self, limit: usize) -> Vec<&SessionHistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.outcome == SessionOutcome::Failure)
            .rev()
            .take(limit)
            .collect()
    }

    pub fn recent_successes(&self, limit: usize) -> Vec<&SessionHistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.outcome == SessionOutcome::Success)
            .rev()
            .take(limit)
            .collect()
    }

    pub fn recent_fallback_count(&self, limit: usize) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.outcome == SessionOutcome::Fallback)
            .rev()
            .take(limit)
            .count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub key: SessionKey,
    pub provider_records: BTreeMap<ProviderId, ProviderSessionRecord>,
    pub route_history: BoundedSessionHistory,
}

impl SessionState {
    pub fn new(key: SessionKey, max_history: usize) -> Self {
        Self {
            key,
            provider_records: BTreeMap::new(),
            route_history: BoundedSessionHistory::new(max_history),
        }
    }

    pub fn ensure_provider_record(&mut self, provider_id: &ProviderId) -> &mut ProviderSessionRecord {
        let now = now_utc();
        self.provider_records
            .entry(provider_id.clone())
            .or_insert_with(|| ProviderSessionRecord {
                provider_id: provider_id.clone(),
                first_seen_utc: now.clone(),
                last_seen_utc: now.clone(),
                successes: 0,
                failures: 0,
                last_failure_reason: None,
                last_success_utc: None,
            })
    }

    pub fn record_success(&mut self, candidate: &RouteCandidate) {
        let now = now_utc();
        let record = self.ensure_provider_record(&candidate.provider_id);
        record.successes = record.successes.saturating_add(1);
        record.last_seen_utc = now.clone();
        record.last_success_utc = Some(now);
        self.route_history.push(SessionHistoryEntry {
            route_candidate_key: candidate.key(),
            outcome: SessionOutcome::Success,
            latency_ms: None,
            occurred_utc: now,
        });
    }

    pub fn record_failure(
        &mut self,
        candidate: &RouteCandidate,
        reason: Option<String>,
    ) {
        let now = now_utc();
        let record = self.ensure_provider_record(&candidate.provider_id);
        record.failures = record.failures.saturating_add(1);
        record.last_seen_utc = now.clone();
        record.last_failure_reason = reason;
        self.route_history.push(SessionHistoryEntry {
            route_candidate_key: candidate.key(),
            outcome: SessionOutcome::Failure,
            latency_ms: None,
            occurred_utc: now,
        });
    }

    pub fn record_fallback(&mut self, candidate: &RouteCandidate) {
        let now = now_utc();
        self.ensure_provider_record(&candidate.provider_id);
        self.route_history.push(SessionHistoryEntry {
            route_candidate_key: candidate.key(),
            outcome: SessionOutcome::Fallback,
            latency_ms: None,
            occurred_utc: now,
        });
    }

    pub fn recent_provider_failures(&self, provider_id: &ProviderId, limit: usize) -> usize {
        self.route_history
            .entries
            .iter()
            .rev()
            .take(limit)
            .filter(|entry| {
                entry.outcome == SessionOutcome::Failure
                    && entry.route_candidate_key.starts_with(provider_id)
            })
            .count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionRegistry {
    pub sessions: BTreeMap<String, SessionState>,
    pub max_history_per_session: usize,
}

impl SessionRegistry {
    pub fn new(max_history_per_session: usize) -> Self {
        Self {
            sessions: BTreeMap::new(),
            max_history_per_session: max_history_per_session.max(1),
        }
    }

    pub fn session_mut(&mut self, key: SessionKey) -> &mut SessionState {
        let stable_key = key.as_stable_key();
        self.sessions
            .entry(stable_key)
            .or_insert_with(|| SessionState::new(key, self.max_history_per_session))
    }
}

fn now_utc() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
