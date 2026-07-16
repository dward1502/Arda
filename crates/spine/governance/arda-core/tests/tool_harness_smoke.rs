// Consumer-side delegation tests: contract/service smoke are now validated
// through `arda-core`'s internal `tool_contract` surface. This file only
// asserts that the migrated smoke contract remains available there.

use arda_core::tool_contract::service::build_invocation_plan;
use arda_core::tool_contract::types::{InvocationEnvelope, RiskLevel, SideEffectClass, ToolMetadata};

#[test]
fn sovereign_baseline_contract_is_migrated() {
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

    assert!(build_invocation_plan(&metadata, &envelope).is_ok());
}
