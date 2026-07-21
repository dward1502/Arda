// sigil: REPAIR
// Adaptive quota types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotaWindow {
    Minute,
    Day,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotaScopeKind {
    Provider,
    Model,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaFacet {
    pub kind: QuotaScopeKind,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub session: Option<String>,
}

impl QuotaFacet {
    pub fn as_key(&self) -> String {
        match self.kind {
            QuotaScopeKind::Provider => format!("provider:{}", self.provider.as_deref().unwrap_or("")),
            QuotaScopeKind::Model => format!(
                "model:{}:{}",
                self.provider.as_deref().unwrap_or(""),
                self.model.as_deref().unwrap_or("")
            ),
            QuotaScopeKind::Session => format!("session:{}", self.session.as_deref().unwrap_or("")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaError {
    Exhausted { window: QuotaWindow, limit: u64, used: u64 },
}

impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaError::Exhausted { window, limit, used } => {
                write!(f, "quota exhausted window={window:?} limit={limit} used={used}")
            }
        }
    }
}

impl std::error::Error for QuotaError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaEntry {
    pub limit: u64,
    pub used: u64,
    pub last_reset_utc: u64,
    pub window: QuotaWindow,
}

impl QuotaEntry {
    pub fn new(limit: u64, window: QuotaWindow) -> Self {
        Self { limit, used: 0, last_reset_utc: 0, window }
    }

    pub fn maybe_reset(&mut self, now_utc: u64) {
        let should_reset = match self.window {
            QuotaWindow::Minute => now_utc.saturating_sub(self.last_reset_utc) >= 60,
            QuotaWindow::Day => now_utc.saturating_sub(self.last_reset_utc) >= 86_400,
        };
        if should_reset {
            self.used = 0;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QuotaLedger {
    entries: std::collections::BTreeMap<String, QuotaEntry>,
}

impl QuotaLedger {
    pub fn entry_mut(
        &mut self,
        facet: &QuotaFacet,
        window: QuotaWindow,
        limit: u64,
    ) -> &mut QuotaEntry {
        self.entries
            .entry(format!("{}:{window:?}", facet.as_key()))
            .or_insert_with(|| QuotaEntry::new(limit, window))
    }

    pub fn release(&mut self, facet: &QuotaFacet, window: QuotaWindow, limit: u64, amount: u64) {
        let entry = self.entry_mut(facet, window, limit);
        let now_utc = 0;
        entry.maybe_reset(now_utc);
        entry.used = entry.used.saturating_sub(amount);
    }
}

#[derive(Debug, Default)]
pub(super) struct AgentQuotaWindows {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, AgentQuotaWindow>>,
}

impl Clone for AgentQuotaWindows {
    fn clone(&self) -> Self {
        Self {
            entries: std::sync::Mutex::new(
                self.entries
                    .lock()
                    .ok()
                    .map(|guard| guard.clone())
                    .unwrap_or_default(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct AgentQuotaWindow {
    pub minute: Option<u64>,
    pub day: Option<u64>,
    pub minute_used: u64,
    pub day_used: u64,
}

impl AgentQuotaWindow {
    pub(super) fn within_limits(&self, limits: &AgentQuotaLimits) -> bool {
        limits.minute.map_or(true, |limit| self.minute_used < limit)
            && limits.day.map_or(true, |limit| self.day_used < limit)
    }

    pub(super) fn reserve_agent_quota(&mut self, _provider: &crate::adaptive::service::types::ProviderState, _req: &crate::adaptive::service::types::ManweRequestEnvelope) {
        let _ = self;
    }
}

pub(super) struct AgentQuotaLimits {
    pub minute: Option<u64>,
    pub day: Option<u64>,
}

pub(super) fn agent_quota_limits(
    _provider: &crate::adaptive::service::types::ProviderState,
    _req: &crate::adaptive::service::types::ManweRequestEnvelope,
) -> AgentQuotaLimits {
    AgentQuotaLimits { minute: None, day: None }
}

pub(super) fn agent_quota_key(provider: &str, agent_id: &str) -> String {
    format!("{}:{agent_id}", provider)
}
