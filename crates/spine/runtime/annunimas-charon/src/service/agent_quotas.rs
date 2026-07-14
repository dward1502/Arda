use super::CharonService;
use crate::types::{CharonRequestEnvelope, ProviderState};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub(super) struct AgentQuotaWindows {
    entries: Mutex<BTreeMap<String, AgentQuotaWindow>>,
}

impl AgentQuotaWindows {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
struct AgentQuotaWindow {
    minute_started_utc: String,
    minute_used: u64,
    day_started_utc: String,
    day_used: u64,
}

#[derive(Debug, Clone, Copy)]
struct AgentQuotaLimits {
    minute: Option<u64>,
    day: Option<u64>,
}

impl CharonService {
    pub(super) fn provider_agent_quota_available(
        &self,
        provider: &ProviderState,
        req: &CharonRequestEnvelope,
    ) -> bool {
        let limits = agent_quota_limits(provider, req);
        if limits.minute.is_none() && limits.day.is_none() {
            return true;
        }
        let now = Utc::now();
        let key = agent_quota_key(&provider.id, &req.agent_id);
        let Ok(mut entries) = self.agent_quota_windows.entries.lock() else {
            return true;
        };
        let entry = entries
            .entry(key)
            .or_insert_with(|| AgentQuotaWindow::new(now));
        entry.refresh(now);
        limits.minute.is_none_or(|limit| entry.minute_used < limit)
            && limits.day.is_none_or(|limit| entry.day_used < limit)
    }

    pub(super) fn reserve_agent_quota(
        &self,
        provider: &ProviderState,
        req: &CharonRequestEnvelope,
    ) {
        let limits = agent_quota_limits(provider, req);
        if limits.minute.is_none() && limits.day.is_none() {
            return;
        }
        let now = Utc::now();
        let key = agent_quota_key(&provider.id, &req.agent_id);
        let Ok(mut entries) = self.agent_quota_windows.entries.lock() else {
            return;
        };
        let entry = entries
            .entry(key)
            .or_insert_with(|| AgentQuotaWindow::new(now));
        entry.refresh(now);
        if limits.minute.is_some() {
            entry.minute_used = entry.minute_used.saturating_add(1);
        }
        if limits.day.is_some() {
            entry.day_used = entry.day_used.saturating_add(1);
        }
    }
}

impl AgentQuotaWindow {
    fn new(now: chrono::DateTime<Utc>) -> Self {
        Self {
            minute_started_utc: now.to_rfc3339(),
            minute_used: 0,
            day_started_utc: now.to_rfc3339(),
            day_used: 0,
        }
    }

    fn refresh(&mut self, now: chrono::DateTime<Utc>) {
        if window_expired(&self.minute_started_utc, now, Duration::seconds(60)) {
            self.minute_started_utc = now.to_rfc3339();
            self.minute_used = 0;
        }
        if window_expired(&self.day_started_utc, now, Duration::seconds(86_400)) {
            self.day_started_utc = now.to_rfc3339();
            self.day_used = 0;
        }
    }
}

fn window_expired(started_utc: &str, now: chrono::DateTime<Utc>, window: Duration) -> bool {
    chrono::DateTime::parse_from_rfc3339(started_utc)
        .map(|started| now - started.with_timezone(&Utc) >= window)
        .unwrap_or(true)
}

fn agent_quota_key(provider_id: &str, agent_id: &str) -> String {
    format!("{provider_id}:{agent_id}")
}

fn agent_quota_limits(provider: &ProviderState, req: &CharonRequestEnvelope) -> AgentQuotaLimits {
    let mut limits = agent_quota_limits_for_request(req);
    if limits.minute.is_none() {
        limits.minute = provider_fraction_limit(
            provider.requests_per_minute,
            "ANNUNIMAS_CHARON_AGENT_MINUTE_QUOTA_FRACTION",
        );
    }
    if limits.day.is_none() {
        limits.day = provider_fraction_limit(
            provider.requests_per_day,
            "ANNUNIMAS_CHARON_AGENT_DAY_QUOTA_FRACTION",
        );
    }
    limits
}

fn agent_quota_limits_for_request(req: &CharonRequestEnvelope) -> AgentQuotaLimits {
    AgentQuotaLimits {
        minute: option_u64(&req.options, "agent_requests_per_minute")
            .or_else(|| option_u64(&req.options, "per_agent_requests_per_minute"))
            .or_else(|| env_u64("ANNUNIMAS_CHARON_AGENT_REQUESTS_PER_MINUTE")),
        day: option_u64(&req.options, "agent_requests_per_day")
            .or_else(|| option_u64(&req.options, "per_agent_requests_per_day"))
            .or_else(|| env_u64("ANNUNIMAS_CHARON_AGENT_REQUESTS_PER_DAY")),
    }
}

fn option_u64(options: &serde_json::Value, key: &str) -> Option<u64> {
    options
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value > 0)
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn provider_fraction_limit(provider_limit: Option<u64>, env_key: &str) -> Option<u64> {
    let provider_limit = provider_limit.filter(|value| *value > 0)?;
    let fraction = std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)?;
    Some(((provider_limit as f64) * fraction).ceil().max(1.0) as u64)
}
