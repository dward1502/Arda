// sigil: ANKH
use crate::contract::{contract, ForgeMindRole, GovernanceBaseline};
use crate::workflow::{ForgeWorkItem, ForgeWorkflowPlan, WorkflowStage};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnnunimasForgeMindStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub role: ForgeMindRole,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub domains_total: usize,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct GovernanceValidation {
    pub ready: bool,
    pub required_validators_total: usize,
}

pub fn status() -> AnnunimasForgeMindStatus {
    let base = contract();
    let governance_ready = validate_governance_baseline(base.governance).ready
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;
    AnnunimasForgeMindStatus {
        crate_name: "annunimas-forge-mind",
        realm: base.realm,
        productizable: base.productizable,
        role: base.role,
        state_export_path: base.state_export_path,
        governance_ready,
        domains_total: 5,
    }
}

pub fn next_stage(item: &ForgeWorkItem) -> WorkflowStage {
    item.next_stage()
}

pub fn workflow_plan(item: &ForgeWorkItem) -> ForgeWorkflowPlan {
    item.plan()
}

pub fn validate_governance_baseline(governance: GovernanceBaseline) -> GovernanceValidation {
    let required_validators_total = [
        governance.triad_required,
        governance.bacon_lite_required,
        governance.joulework_required,
        governance.love_equation_required,
        governance.soterion_trace_required,
    ]
    .into_iter()
    .filter(|required| *required)
    .count();

    GovernanceValidation {
        ready: required_validators_total == 5,
        required_validators_total,
    }
}

pub struct ForgePlanner;

impl Default for ForgePlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgePlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn create_plan(&self, item: &ForgeWorkItem) -> Result<ForgeWorkflowPlan, anyhow::Error> {
        Ok(item.plan())
    }
}

pub struct ForgeExecutor;

impl Default for ForgeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, plan: ForgeWorkflowPlan) -> Result<(), anyhow::Error> {
        // For now, just print the plan. In the future, this will interact with Blender.
        println!("Executing forge plan for domain: {:?}", plan.domain);
        println!("Description: {}", plan.work_item.description);
        Ok(())
    }
}

pub struct ForgeMind {
    planner: ForgePlanner,
    executor: ForgeExecutor,
}

impl Default for ForgeMind {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeMind {
    pub fn new() -> Self {
        Self {
            planner: ForgePlanner::new(),
            executor: ForgeExecutor::new(),
        }
    }

    pub fn forge(&self, item: ForgeWorkItem) -> Result<ForgeWorkflowPlan, anyhow::Error> {
        let plan = self.planner.create_plan(&item)?;
        self.executor.execute(plan.clone())?;
        Ok(plan)
    }
}
