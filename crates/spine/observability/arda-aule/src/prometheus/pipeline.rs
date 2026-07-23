#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Arda CEO Pipeline
//!
//! Simplified orchestration - routes tasks to agents

mod fleet_routing;
mod local_execution;
mod preflight;
mod runtime_status;
mod support;

use crate::core_link::CoreAutonomyProfile;
use crate::council::CouncilGateConfig;
use crate::heartbeat::{select_heartbeat_mode, HeartbeatState};
use crate::orders::OrderStore;
use crate::registry::AgentRosterSnapshot;
use crate::thought::ThoughtLedger;
use arda_core::error::Result;
use arda_core::ledger::Ledger;
use arda_core::router::Router;
use arda_core::task::Task;
use annunimas_fleet::{
    EdgeDispatcher, EdgeHealthMonitor, FleetCapacityManager, ProviderTokenTracker,
};
use arda_vaire::MnemosyneService;

pub struct Pipeline {
    router: Router,
    ledger: Ledger,
    joule_budget: u64,
    autonomy: Option<CoreAutonomyProfile>,
    confidence_threshold: f64,
    heartbeat: HeartbeatState,
    roster: Option<AgentRosterSnapshot>,
    thought_ledger: Option<ThoughtLedger>,
    order_store: Option<OrderStore>,
    council_config: CouncilGateConfig,
    mnemosyne: Option<MnemosyneService>,
    fleet_manager: Option<FleetCapacityManager>,
    edge_dispatcher: Option<EdgeDispatcher>,
    provider_tracker: ProviderTokenTracker,
    health_monitor: Option<EdgeHealthMonitor>,
}

impl Pipeline {
    pub fn new(router: Router, ledger: Ledger, joule_budget: u64) -> Self {
        let fleet_manager = FleetCapacityManager::new(".").ok();
        let health_monitor = fleet_manager.as_ref().map(|_| EdgeHealthMonitor::new("."));

        Self {
            router,
            ledger,
            joule_budget,
            autonomy: None,
            confidence_threshold: 0.0,
            heartbeat: select_heartbeat_mode(None),
            roster: None,
            thought_ledger: ThoughtLedger::from_default_or_fallback().ok(),
            order_store: OrderStore::from_default_or_fallback().ok(),
            council_config: CouncilGateConfig::default(),
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
            fleet_manager,
            edge_dispatcher: Some(EdgeDispatcher::new()),
            provider_tracker: {
                let tracker = ProviderTokenTracker::new(".");
                let _ = tracker.load_state();
                tracker
            },
            health_monitor,
        }
    }

    pub fn with_core_link(
        router: Router,
        ledger: Ledger,
        joule_budget: u64,
        core_root: impl AsRef<std::path::Path>,
    ) -> Self {
        let autonomy = CoreAutonomyProfile::load(&core_root);
        let heartbeat = select_heartbeat_mode(autonomy.as_ref());
        let roster = AgentRosterSnapshot::from_world_file(
            core_root.as_ref().join("state").join("world.json"),
            300,
        );
        let fleet_manager = FleetCapacityManager::new(".").ok();
        let health_monitor = fleet_manager.as_ref().map(|_| EdgeHealthMonitor::new("."));

        Self {
            router,
            ledger,
            joule_budget,
            autonomy,
            confidence_threshold: 0.75,
            heartbeat,
            roster,
            thought_ledger: ThoughtLedger::from_default_or_fallback().ok(),
            order_store: OrderStore::from_default_or_fallback().ok(),
            council_config: CouncilGateConfig::default(),
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
            fleet_manager,
            edge_dispatcher: Some(EdgeDispatcher::new()),
            provider_tracker: {
                let tracker = ProviderTokenTracker::new(".");
                let _ = tracker.load_state();
                tracker
            },
            health_monitor,
        }
    }

    pub async fn submit(&self, mut task: Task) -> Result<Task> {
        self.record_task_intake(&task)?;

        let Some(confidence) = self.apply_governance_preflight(&mut task).await? else {
            return Ok(task);
        };

        if let Some(task) = self.try_fleet_or_external_route(&mut task).await? {
            return Ok(task);
        }

        self.route_and_execute_locally(&mut task, confidence)
            .await?;

        Ok(task)
    }
}
