use annunimas_tool_harness::contract::contract;
use annunimas_tool_harness::service::{
    build_invocation_plan, status, validate_governance_baseline, validate_invocation,
};
use annunimas_tool_harness::types::{
    InvocationDisposition, InvocationEnvelope, RiskLevel, SideEffectClass, ToolMetadata,
};

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
        "core/state/annunimas-tool-harness.json"
    );
    assert!(validate_governance_baseline(base.governance).ready);
}

#[test]
fn service_status_reports_governance_ready() {
    let report = status();
    assert!(report.governance_ready);
}

#[test]
fn mutating_tool_requires_trace_actor_and_idempotency() {
    let metadata = ToolMetadata {
        tool_id: "tool.demo".into(),
        version: "1".into(),
        owner: "athena".into(),
        description: "demo".into(),
        input_schema_ref: "schema/in.json".into(),
        output_schema_ref: "schema/out.json".into(),
        risk_level: RiskLevel::High,
        side_effect_class: SideEffectClass::Mutating,
    };
    let envelope = InvocationEnvelope {
        trace_id: Some("trace-1".into()),
        actor: Some("athena".into()),
        idempotency_key: Some("idem-1".into()),
    };
    assert!(validate_invocation(&metadata, &envelope).is_ok());
}

#[test]
fn critical_mutating_tool_plan_requires_review_and_idempotency() {
    let metadata = ToolMetadata {
        tool_id: "tool.critical".into(),
        version: "1".into(),
        owner: "hades".into(),
        description: "critical mutating demo".into(),
        input_schema_ref: "schema/in.json".into(),
        output_schema_ref: "schema/out.json".into(),
        risk_level: RiskLevel::Critical,
        side_effect_class: SideEffectClass::Mutating,
    };
    let envelope = InvocationEnvelope {
        trace_id: Some("trace-1".into()),
        actor: Some("hades".into()),
        idempotency_key: Some("idem-1".into()),
    };

    let plan = build_invocation_plan(&metadata, &envelope).expect("plan");
    assert_eq!(
        plan.disposition,
        InvocationDisposition::HoldForOperatorReview
    );
    assert!(plan.idempotency_required);
    assert!(plan.operator_review_required);
}
