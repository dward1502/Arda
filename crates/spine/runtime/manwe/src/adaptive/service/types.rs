// sigil: adaptive service spine
//
// Owns the CharonService struct plus stable re-exports/facade accessors.
// Implementation methods live in their owning submodules; this file only
// declares fields and simple accessors needed by those modules.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

pub use crate::adaptive::types::{
    CharonRequestEnvelope, ModelCapabilities, ModelState, ProviderState, RouteDecision,
    RouteGovernance, RouteGovernanceLens, RouteLoveEquationGuard,
};

#[derive(Debug, Clone)]
pub struct CharonService {
    pub root: PathBuf,
    pub providers: Arc<RwLock<Vec<ProviderState>>>,
    pub event_writer: crate::adaptive::service::event_writer::EventWriter,
    pub mnemosyne: Option<crate::adaptive::service::service_events::MnemosyneClient>,
    pub state_path: PathBuf,
    pub governance_events_path: PathBuf,
    pub package_runtime_signals_path: PathBuf,
    pub lane_fitness_path: PathBuf,
    pub provider_runtime_state_path: PathBuf,
    pub provider_capability_receipts_path: PathBuf,
    pub tool_fit_ledger_path: PathBuf,
    pub route_history: Arc<RwLock<VecDeque<crate::adaptive::service::route_sessions::RouteHistoryEntry>>>,
    pub route_sessions: Arc<RwLock<BTreeMap<String, crate::adaptive::service::route_sessions::StickyRouteSession>>>,
    pub charon_eval_receipts_path: PathBuf,
    pub bandit: crate::adaptive::service::bandit::BanditStore,
    pub agent_quota_windows: crate::adaptive::service::agent_quotas::AgentQuotaWindows,
    pub route_candidate_cache: crate::adaptive::service::route_candidate_cache::RouteCandidateCache,
    pub http_clients: Arc<RwLock<Option<crate::adaptive::service::http_clients::HttpClientCache>>>,
    pub sticky_sessions: Arc<RwLock<BTreeMap<String, crate::adaptive::service::route_sessions::StickyRouteSession>>>,
    pub capability_receipts_file: Arc<RwLock<Option<crate::adaptive::service::capabilities::ProviderCapabilityReceiptsFile>>>,
}

impl CharonService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            root: root.clone(),
            providers: Arc::new(RwLock::new(Vec::new())),
            event_writer: crate::adaptive::service::event_writer::EventWriter::new(
                root.join("events"),
                root.join("governance_events.jsonl"),
            ),
            mnemosyne: None,
            state_path: root.join("state.jsonl"),
            governance_events_path: root.join("governance_events.jsonl"),
            package_runtime_signals_path: root.join("package_runtime_signals.json"),
            lane_fitness_path: root.join("lane_fitness.json"),
            provider_runtime_state_path: root.join("provider_runtime_state.json"),
            provider_capability_receipts_path: root.join("provider_capability_receipts.json"),
            tool_fit_ledger_path: root.join("tool_fit_ledger.jsonl"),
            route_history: Arc::new(RwLock::new(VecDeque::new())),
            route_sessions: Arc::new(RwLock::new(BTreeMap::new())),
            charon_eval_receipts_path: root.join("model_eval_receipts.jsonl"),
            bandit: crate::adaptive::service::bandit::BanditStore::new(
                root.join("bandit_state.json"),
            ),
            agent_quota_windows: crate::adaptive::service::agent_quotas::AgentQuotaWindows::new(),
            route_candidate_cache: crate::adaptive::service::route_candidate_cache::RouteCandidateCache::new(),
            http_clients: Arc::new(RwLock::new(None)),
            sticky_sessions: Arc::new(RwLock::new(BTreeMap::new())),
            capability_receipts_file: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn providers_read(&self) -> RwLockReadGuard<'_, Vec<ProviderState>> {
        self.providers.read().await
    }

    pub async fn providers(&self) -> RwLockReadGuard<'_, Vec<ProviderState>> {
        self.providers.read().await
    }

    pub fn package_runtime_signals_path(&self) -> PathBuf {
        self.package_runtime_signals_path.clone()
    }

    pub fn provider_capability_receipts_path(&self) -> PathBuf {
        self.provider_capability_receipts_path.clone()
    }

    pub fn socket_path(&self) -> PathBuf {
        self.root.join("manwe.sock")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config/governance/charon.providers.toml")
    }

    pub fn bootstrap_state_path(&self) -> PathBuf {
        self.root.join("bootstrap_state.json")
    }

    pub fn charon_eval_receipts_path(&self) -> PathBuf {
        self.charon_eval_receipts_path.clone()
    }

    pub async fn persist_provider_runtime_state(&self) -> arda_core::error::Result<()> {
        let _ = self.provider_runtime_state_path;
        Ok(())
    }

    pub async fn persist_provider_runtime_state_snapshot(
        &self,
        _providers: &[ProviderState],
    ) -> arda_core::error::Result<()> {
        let _ = self.provider_runtime_state_path;
        Ok(())
    }

    pub fn metrics(&self) -> crate::adaptive::service::metrics::CharonMetrics {
        crate::adaptive::service::metrics::CharonMetrics::default()
    }

    pub async fn recent_state_events(
        &self,
        _limit: usize,
    ) -> Vec<serde_json::Value> {
        let _ = self.state_path;
        Vec::new()
    }

    pub fn provider_capability_summary(
        &self,
    ) -> crate::adaptive::service::capabilities::ProviderCapabilitySummary {
        crate::adaptive::service::capabilities::ProviderCapabilitySummary::default()
    }

    pub fn read_lane_fitness_snapshot(
        &self,
    ) -> Option<crate::adaptive::service::route_policy::LaneFitnessSnapshot> {
        let _ = self.lane_fitness_path;
        None
    }
}

pub struct StdDuration(std::time::Duration);

impl From<std::time::Duration> for StdDuration {
    fn from(value: std::time::Duration) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for StdDuration {
    type Target = std::time::Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
