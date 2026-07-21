#![cfg(feature = "full-cli")]
use crate::core_link::CoreAutonomyProfile;
use crate::heartbeat::select_heartbeat_mode;
use crate::orders::{EscalationEvent, OrderStore, RuntimeReconcileSummary};
use crate::registry::AgentRosterSnapshot;
use crate::service::{prometheus_home, PrometheusService};
use crate::thought::ThoughtLedger;
use annunimas_core::error::Result;
use annunimas_mnemosyne::MnemosyneService;
use chrono::{DateTime, Utc};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

impl PrometheusService {
    pub fn from_core(core_root: impl AsRef<Path>) -> Result<Self> {
        let core_root = core_root.as_ref().to_path_buf();
        let profile = CoreAutonomyProfile::load(&core_root);
        let heartbeat = select_heartbeat_mode(profile.as_ref());
        let roster = AgentRosterSnapshot::from_world_file(core_root.join("state/world.json"), 300);
        let thought_ledger = ThoughtLedger::from_default_or_fallback()?;
        let order_store = OrderStore::from_default_or_fallback()?;
        let council_events_path = prometheus_home().join("council_fanout.jsonl");
        let execution_intents_path = prometheus_home().join("execution_intents.jsonl");
        let execution_intents_recovery_path =
            prometheus_home().join("execution_intents_recovery_last.json");
        if let Some(parent) = council_events_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&council_events_path);
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&execution_intents_path);
        let _ = Self::write_execution_intents_recovery(
            &execution_intents_path,
            &execution_intents_recovery_path,
        );

        Ok(Self {
            core_root,
            profile,
            heartbeat,
            roster,
            thought_ledger,
            order_store,
            council_events_path,
            execution_intents_path,
            execution_intents_recovery_path,
            confidence_threshold: 0.75,
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
        })
    }

    #[cfg(test)]
    pub(crate) fn from_core_for_test(
        core_root: impl AsRef<Path>,
        prometheus_home: impl AsRef<Path>,
        minds_home: impl AsRef<Path>,
    ) -> Result<Self> {
        let core_root = core_root.as_ref().to_path_buf();
        let prometheus_home = prometheus_home.as_ref().to_path_buf();
        let minds_home = minds_home.as_ref().to_path_buf();
        let profile = CoreAutonomyProfile::load(&core_root);
        let heartbeat = select_heartbeat_mode(profile.as_ref());
        let roster = AgentRosterSnapshot::from_world_file(core_root.join("state/world.json"), 300);
        let thought_ledger = ThoughtLedger::new(&minds_home)?;
        let order_store = OrderStore::new(&prometheus_home)?;
        let council_events_path = prometheus_home.join("council_fanout.jsonl");
        let execution_intents_path = prometheus_home.join("execution_intents.jsonl");
        let execution_intents_recovery_path =
            prometheus_home.join("execution_intents_recovery_last.json");
        if let Some(parent) = council_events_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&council_events_path);
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&execution_intents_path);
        let _ = Self::write_execution_intents_recovery(
            &execution_intents_path,
            &execution_intents_recovery_path,
        );

        Ok(Self {
            core_root,
            profile,
            heartbeat,
            roster,
            thought_ledger,
            order_store,
            council_events_path,
            execution_intents_path,
            execution_intents_recovery_path,
            confidence_threshold: 0.75,
            mnemosyne: None,
        })
    }

    pub fn roster(&self) -> Option<AgentRosterSnapshot> {
        self.load_roster().or_else(|| self.roster.clone())
    }

    pub fn thoughts(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        self.thought_ledger.recent(limit)
    }

    pub fn escalations(
        &self,
        limit: usize,
        include_resolved: bool,
    ) -> Result<Vec<EscalationEvent>> {
        self.order_store.list_escalations(include_resolved, limit)
    }

    pub fn resolve_escalation(&self, escalation_id: &str, note: &str) -> Result<EscalationEvent> {
        self.order_store.resolve_escalation(escalation_id, note)
    }

    pub fn reconcile_runtime(
        &self,
        cutoff: DateTime<Utc>,
        apply: bool,
        note: &str,
    ) -> Result<RuntimeReconcileSummary> {
        self.order_store
            .reconcile_stale_runtime(cutoff, apply, note)
    }

    pub fn core_root(&self) -> &Path {
        &self.core_root
    }

    pub fn socket_path(&self) -> PathBuf {
        if let Ok(socket) = std::env::var("ANNUNIMAS_PROMETHEUS_SOCKET") {
            return PathBuf::from(socket);
        }
        prometheus_home().join("prometheus.sock")
    }

    pub fn http_addr(&self) -> String {
        format!("{}:{}", "127.0.0.1", 5113)
    }

    pub fn _profile(&self) -> Option<&CoreAutonomyProfile> {
        self.profile.as_ref()
    }

    pub(crate) fn load_roster(&self) -> Option<AgentRosterSnapshot> {
        self.load_supervisor_roster().or_else(|| {
            AgentRosterSnapshot::from_world_file(self.core_root.join("state/world.json"), 300)
        })
    }

    pub(crate) fn load_supervisor_roster(&self) -> Option<AgentRosterSnapshot> {
        AgentRosterSnapshot::from_supervisor_state_file(
            prometheus_home().join("supervisor/state.json"),
        )
    }
}
