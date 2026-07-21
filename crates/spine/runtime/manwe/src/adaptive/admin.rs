// sigil: REPAIR
// Administrative transport types for adaptive runtime.
//
// This module owns the internal admin surface shapes so the adaptive
// subtree can report mode, capabilities, quotas, and provider health
// without depending on a specific transport implementation.
//
// No real auth/admin exposure is implemented here; callers should gate
// these surfaces behind internal checks before exposing them externally.

use std::collections::BTreeMap;

use crate::adaptive::provider::{HealthState, ProviderCapabilitySummary};
use crate::adaptive::quota::{QuotaEntry, QuotaStore, QuotaWindow};
use crate::adaptive::state::AdaptiveSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminMode {
    Static,
    Adaptive,
}

#[derive(Debug, Clone, Default)]
pub struct AdminCapabilityReport {
    pub mode: AdminMode,
    pub provider_count: usize,
    pub model_count: usize,
    pub healthy_provider_count: usize,
    pub degraded_provider_count: usize,
    pub down_provider_count: usize,
    pub providers: Vec<ProviderCapabilitySummary>,
}

#[derive(Debug, Clone, Default)]
pub struct AdminQuotaReport {
    pub window_counts: BTreeMap<String, usize>,
    pub exhausted_count: usize,
    pub entries: Vec<QuotaEntrySummary>,
}

#[derive(Debug, Clone, Default)]
pub struct QuotaEntrySummary {
    pub facet: String,
    pub window: String,
    pub limit: u64,
    pub used: u64,
    pub remaining: u64,
}

#[derive(Debug, Clone, Default)]
pub struct AdminProviderHealthReport {
    pub provider_count: usize,
    pub healthy_count: usize,
    pub degraded_count: usize,
    pub down_count: usize,
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct AdminSnapshot {
    pub mode: AdminMode,
    pub capabilities: AdminCapabilityReport,
    pub quotas: AdminQuotaReport,
    pub provider_health: AdminProviderHealthReport,
}

impl AdminSnapshot {
    pub fn from_capability_summaries(
        mode: AdminMode,
        providers: &[ProviderCapabilitySummary],
    ) -> Self {
        let mut report = AdminCapabilityReport {
            mode,
            provider_count: providers.len(),
            ..AdminCapabilityReport::default()
        };

        let mut health = AdminProviderHealthReport::default();
        for provider in providers {
            report.model_count = report.model_count.saturating_add(1);
            match provider.health {
                HealthState::Healthy => report.healthy_provider_count = report.healthy_provider_count.saturating_add(1),
                HealthState::Degraded => report.degraded_provider_count = report.degraded_provider_count.saturating_add(1),
                HealthState::Down => report.down_provider_count = report.down_provider_count.saturating_add(1),
                HealthState::Unknown | HealthState::Probing => {}
            }
            health.provider_count = health.provider_count.saturating_add(1);
            match provider.health {
                HealthState::Healthy => health.healthy_count = health.healthy_count.saturating_add(1),
                HealthState::Degraded => health.degraded_count = health.degraded_count.saturating_add(1),
                HealthState::Down => health.down_count = health.down_count.saturating_add(1),
                HealthState::Unknown | HealthState::Probing => health.unknown_count = health.unknown_count.saturating_add(1),
            }
        }

        Self {
            mode,
            capabilities: report,
            quotas: AdminQuotaReport::default(),
            provider_health: health,
        }
    }
}

pub fn build_admin_snapshot(
    mode: AdminMode,
    snapshot: &AdaptiveSnapshot,
    quota_store: &QuotaStore,
) -> AdminSnapshot {
    let mut admin = AdminSnapshot::from_capability_summaries(
        mode,
        &snapshot.probes.provider_states.values().map(|probe| ProviderCapabilitySummary {
            provider_id: probe.provider_id.clone(),
            health: match probe.state.as_str() {
                "healthy" => HealthState::Healthy,
                "degraded" => HealthState::Degraded,
                "down" => HealthState::Down,
                _ => HealthState::Unknown,
            },
            features: crate::adaptive::provider::ProviderFeatures::default(),
            rate_limits: crate::adaptive::provider::RateLimits::default(),
            token_caps: crate::adaptive::provider::TokenCaps::default(),
            has_api_key: true,
            in_cooldown: false,
            last_error: None,
        }).collect::<Vec<_>>(),
    );

    let mut quota_report = AdminQuotaReport::default();
    for (key, entry) in quota_store.snapshot() {
        quota_report.entries.push(QuotaEntrySummary {
            facet: key.clone(),
            window: format!("{window:?}", window = entry.window),
            limit: entry.limit,
            used: entry.used,
            remaining: entry.remaining(),
        });
        if entry.exhausted() {
            quota_report.exhausted_count = quota_report.exhausted_count.saturating_add(1);
        }
        let window_key = format!("{window:?}", window = entry.window);
        quota_report.window_counts.insert(window_key, quota_report.window_counts.get(&window_key).copied().unwrap_or_default().saturating_add(1));
    }
    admin.quotas = quota_report;

    admin
}
