// sigil: ANKH
use serde::{Deserialize, Serialize};

pub const GOVERNANCE_VALIDATORS: [&str; 5] = [
    "triad",
    "bacon_lite",
    "joulework",
    "love_equation",
    "soterion_trace",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SideEffectClass {
    ReadOnly,
    Mutating,
}

impl SideEffectClass {
    pub fn is_mutating(self) -> bool {
        matches!(self, Self::Mutating)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvocationDisposition {
    AllowReadOnly,
    AllowMutatingWithIdempotency,
    HoldForOperatorReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolMetadata {
    pub tool_id: String,
    pub version: String,
    pub owner: String,
    pub description: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub risk_level: RiskLevel,
    pub side_effect_class: SideEffectClass,
}

impl ToolMetadata {
    pub fn validate(&self) -> Result<(), HarnessError> {
        if self.tool_id.trim().is_empty() {
            return Err(HarnessError::InvalidRequest("tool_id"));
        }
        if self.owner.trim().is_empty() {
            return Err(HarnessError::InvalidRequest("owner"));
        }
        if self.input_schema_ref.trim().is_empty() || self.output_schema_ref.trim().is_empty() {
            return Err(HarnessError::InvalidRequest("schema_ref"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvocationEnvelope {
    pub trace_id: Option<String>,
    pub actor: Option<String>,
    pub idempotency_key: Option<String>,
}

impl InvocationEnvelope {
    pub fn validate(&self, requires_idempotency: bool) -> Result<(), HarnessError> {
        if self.trace_id.as_deref().unwrap_or("").trim().is_empty() {
            return Err(HarnessError::MissingTraceId);
        }
        if self.actor.as_deref().unwrap_or("").trim().is_empty() {
            return Err(HarnessError::MissingActor);
        }
        if requires_idempotency
            && self
                .idempotency_key
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(HarnessError::MissingIdempotencyKey);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultEnvelope<T> {
    pub ok: bool,
    pub error_code: Option<String>,
    pub payload: Option<T>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InvocationPlan {
    pub disposition: InvocationDisposition,
    pub side_effect_class: SideEffectClass,
    pub risk_level: RiskLevel,
    pub idempotency_required: bool,
    pub operator_review_required: bool,
    pub governance_validators: [&'static str; 5],
}

pub fn plan_invocation(metadata: &ToolMetadata) -> InvocationPlan {
    let idempotency_required = metadata.side_effect_class.is_mutating();
    let operator_review_required = matches!(metadata.risk_level, RiskLevel::Critical);
    let disposition = if operator_review_required {
        InvocationDisposition::HoldForOperatorReview
    } else if idempotency_required {
        InvocationDisposition::AllowMutatingWithIdempotency
    } else {
        InvocationDisposition::AllowReadOnly
    };

    InvocationPlan {
        disposition,
        side_effect_class: metadata.side_effect_class,
        risk_level: metadata.risk_level,
        idempotency_required,
        operator_review_required,
        governance_validators: GOVERNANCE_VALIDATORS,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HarnessError {
    MissingTraceId,
    MissingActor,
    MissingIdempotencyKey,
    InvalidRequest(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(risk_level: RiskLevel, side_effect_class: SideEffectClass) -> ToolMetadata {
        ToolMetadata {
            tool_id: String::from("tool.demo"),
            version: String::from("1"),
            owner: String::from("athena"),
            description: String::from("demo"),
            input_schema_ref: String::from("schema/in.json"),
            output_schema_ref: String::from("schema/out.json"),
            risk_level,
            side_effect_class,
        }
    }

    #[test]
    fn read_only_low_risk_tool_does_not_require_idempotency_or_review() {
        let plan = plan_invocation(&metadata(RiskLevel::Low, SideEffectClass::ReadOnly));

        assert_eq!(plan.disposition, InvocationDisposition::AllowReadOnly);
        assert!(!plan.idempotency_required);
        assert!(!plan.operator_review_required);
    }

    #[test]
    fn mutating_tool_requires_idempotency_in_plan() {
        let plan = plan_invocation(&metadata(RiskLevel::High, SideEffectClass::Mutating));

        assert_eq!(
            plan.disposition,
            InvocationDisposition::AllowMutatingWithIdempotency
        );
        assert!(plan.idempotency_required);
        assert!(!plan.operator_review_required);
    }

    #[test]
    fn critical_tool_is_held_for_operator_review() {
        let plan = plan_invocation(&metadata(RiskLevel::Critical, SideEffectClass::Mutating));

        assert_eq!(
            plan.disposition,
            InvocationDisposition::HoldForOperatorReview
        );
        assert!(plan.idempotency_required);
        assert!(plan.operator_review_required);
        assert_eq!(plan.governance_validators.len(), 5);
    }
}
