//! Tool-contract service helpers. Re-exports the core contract validation so
//! downstream crates can statically assert harness readiness without pulling
//! the retired `arda-tool-harness` crate.

use crate::tool_contract::types::{InvocationEnvelope, InvocationPlan, ToolMetadata};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceBaseline {
    pub triad_required: bool,
    pub bacon_lite_required: bool,
    pub joulework_required: bool,
    pub love_equation_required: bool,
    pub soterion_trace_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinuityBaseline {
    pub task_ledger_linked: bool,
    pub memory_checkpoint_expected: bool,
    pub arda_visibility_defined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceStatus {
    pub governance_ready: bool,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        Self {
            governance_ready: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceError {
    pub message: &'static str,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

pub fn validate_governance_baseline(baseline: &GovernanceBaseline) -> bool {
    baseline.triad_required
        && baseline.bacon_lite_required
        && baseline.joulework_required
        && baseline.love_equation_required
        && baseline.soterion_trace_required
}

pub fn build_invocation_plan(
    metadata: &ToolMetadata,
    envelope: &InvocationEnvelope,
) -> Result<InvocationPlan, ServiceError> {
    if metadata.side_effect_class.is_mutating() {
        let idempotency = envelope
            .idempotency_key
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        if !idempotency {
            return Err(ServiceError {
                message: "mutation requires idempotency",
            });
        }
    }

    Ok(crate::tool_contract::types::plan_invocation(metadata))
}

pub fn validate_invocation(
    metadata: &ToolMetadata,
    envelope: &InvocationEnvelope,
) -> Result<(), ServiceError> {
    let plan = build_invocation_plan(metadata, envelope)?;
    let requires_idempotency = plan.idempotency_required;
    envelope
        .validate(requires_idempotency)
        .map_err(|_| ServiceError {
            message: "invocation envelope validation failed",
        })?;
    Ok(())
}

pub fn status() -> ServiceStatus {
    ServiceStatus::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_contract::types::InvocationDisposition;

    fn high_metadata() -> ToolMetadata {
        ToolMetadata {
            tool_id: "tool.critical".into(),
            version: "1".into(),
            owner: "hades".into(),
            description: "critical mutating demo".into(),
            input_schema_ref: "schema/in.json".into(),
            output_schema_ref: "schema/out.json".into(),
            risk_level: crate::tool_contract::types::RiskLevel::Critical,
            side_effect_class: crate::tool_contract::types::SideEffectClass::Mutating,
        }
    }

    #[test]
    fn mutating_tool_requires_idempotency_key() {
        assert!(build_invocation_plan(&high_metadata(), &InvocationEnvelope::default()).is_err());
    }

    #[test]
    fn operator_review_plan_requires_full_envelope() {
        let envelope = InvocationEnvelope {
            trace_id: Some("trace-1".into()),
            actor: Some("hades".into()),
            idempotency_key: Some("idem-1".into()),
        };
        let plan = build_invocation_plan(&high_metadata(), &envelope)
            .expect("plans critical mutating invocation");
        assert_eq!(
            plan.disposition,
            InvocationDisposition::HoldForOperatorReview
        );
        assert!(plan.idempotency_required);
        assert!(plan.operator_review_required);
    }
}
