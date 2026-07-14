use annunimas_forge_mind::contract::{contract, ArtifactPolicy};
use annunimas_forge_mind::service::{
    next_stage, status, validate_governance_baseline, workflow_plan,
};
use annunimas_forge_mind::workflow::{EngineeringDomain, ForgeWorkItem, WorkflowStage};

#[test]
fn sovereign_baseline_contract_is_present() {
    let base = contract();
    assert!(base.governance.triad_required);
    assert!(base.governance.bacon_lite_required);
    assert!(base.governance.joulework_required);
    assert!(base.governance.love_equation_required);
    assert!(base.continuity.task_ledger_linked);
    assert!(base.continuity.memory_checkpoint_expected);
    assert_eq!(
        base.state_export_path,
        "core/state/annunimas-forge-mind.json"
    );
    assert!(validate_governance_baseline(base.governance).ready);
}

#[test]
fn service_status_reports_governance_ready() {
    let report = status();
    assert!(report.governance_ready);
}

#[test]
fn workflow_requires_research_before_build() {
    let item = ForgeWorkItem {
        domain: EngineeringDomain::SoftwareSystems,
        description: "Test item".to_string(),
        has_research: false,
        has_build_artifact: false,
        target_output: vec![],
    };
    assert_eq!(next_stage(&item), WorkflowStage::Research);
}

#[test]
fn workflow_plan_requires_verification_after_build_artifact() {
    let item = ForgeWorkItem {
        domain: EngineeringDomain::PhysicalFabrication,
        description: "Test item".to_string(),
        has_research: true,
        has_build_artifact: true,
        target_output: vec![],
    };

    let plan = workflow_plan(&item);
    assert_eq!(plan.next_stage, WorkflowStage::Verify);
    assert_eq!(plan.artifact_policy, ArtifactPolicy::VerificationRequired);
}
