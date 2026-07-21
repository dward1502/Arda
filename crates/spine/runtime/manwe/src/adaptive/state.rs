// sigil: REPAIR
// Persisted state snapshot for adaptive runtime subsystems.
//
// This module owns the in-memory snapshot shape that can be serialized to
// disk by callers. It does not itself perform file I/O so the adaptive
// subtree remains testable without filesystem fixtures.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct AdaptiveSnapshot {
    pub bandit: BanditSnapshot,
    pub quotas: QuotaSnapshot,
    pub probes: ProbeSnapshot,
    pub cache: CacheSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct BanditSnapshot {
    pub arms: BTreeMap<String, BanditArmSnapshot>,
    pub epsilon: f64,
    pub min_observations: u64,
}

#[derive(Debug, Clone, Default)]
pub struct BanditArmSnapshot {
    pub successes: u64,
    pub failures: u64,
    pub last_reward: f64,
}

#[derive(Debug, Clone, Default)]
pub struct QuotaSnapshot {
    pub entries: BTreeMap<String, QuotaEntrySnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct QuotaEntrySnapshot {
    pub limit: u64,
    pub used: u64,
    pub last_reset_utc: u64,
    pub window: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProbeSnapshot {
    pub last_probe_utc: Option<u64>,
    pub provider_states: BTreeMap<String, ProviderProbeSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderProbeSnapshot {
    pub provider_id: String,
    pub state: String,
    pub updated_utc: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CacheSnapshot {
    pub candidate_keys: Vec<String>,
}

pub trait InMemoryStore {
    fn snapshot(&self) -> AdaptiveSnapshot;
    fn restore(&mut self, snapshot: AdaptiveSnapshot);
}
