//! Versioned structured evidence extracted from `Task.result`.

use arda_core::Task;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const GOVERNANCE_EVIDENCE_SCHEMA_VERSION: &str = "arda.governance.evidence.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceEvidenceAnchor {
    pub kind: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceEvidence {
    pub schema_version: String,
    #[serde(default)]
    pub evidence_anchors: Vec<GovernanceEvidenceAnchor>,
    #[serde(default)]
    pub action_intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justified_urgency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooperation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defection: Option<f64>,
    #[serde(default)]
    pub disconfirming_evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_boundary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceEvidenceGrade {
    NoEvidence,
    HeuristicOnly,
    StructuredPartial,
    StructuredValidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceScoringSource {
    StructuredEvidence,
    LegacyResultMapping,
    HeuristicFallback,
    MalformedStructuredFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceEvidenceAssessment {
    pub schema_version: String,
    pub grade: GovernanceEvidenceGrade,
    pub scoring_source: GovernanceScoringSource,
    #[serde(default)]
    pub missing_fields: Vec<String>,
    #[serde(default)]
    pub validation_errors: Vec<String>,
}

impl Default for GovernanceEvidenceAssessment {
    fn default() -> Self {
        Self {
            schema_version: GOVERNANCE_EVIDENCE_SCHEMA_VERSION.to_string(),
            grade: GovernanceEvidenceGrade::NoEvidence,
            scoring_source: GovernanceScoringSource::HeuristicFallback,
            missing_fields: Vec::new(),
            validation_errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceEvidenceContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<GovernanceEvidence>,
    pub assessment: GovernanceEvidenceAssessment,
}

impl GovernanceEvidenceContext {
    pub fn uses_validated_structured_evidence(&self) -> bool {
        self.assessment.grade == GovernanceEvidenceGrade::StructuredValidated
    }
}

pub fn assess_governance_evidence(task: &Task) -> GovernanceEvidenceContext {
    let Some(result) = task.result.as_ref() else {
        return fallback_context(
            GovernanceEvidenceGrade::NoEvidence,
            GovernanceScoringSource::HeuristicFallback,
            Vec::new(),
        );
    };
    let Some(object) = result.as_object() else {
        return fallback_context(
            GovernanceEvidenceGrade::NoEvidence,
            GovernanceScoringSource::MalformedStructuredFallback,
            vec!["Task.result must be a JSON object".to_string()],
        );
    };

    if let Some(raw) = object.get("governance_evidence") {
        return match serde_json::from_value::<GovernanceEvidence>(raw.clone()) {
            Ok(evidence) => context_from_evidence(
                evidence,
                GovernanceScoringSource::StructuredEvidence,
                Vec::new(),
            ),
            Err(error) => fallback_context(
                GovernanceEvidenceGrade::NoEvidence,
                GovernanceScoringSource::MalformedStructuredFallback,
                vec![format!("invalid governance_evidence payload: {error}")],
            ),
        };
    }

    match map_legacy_result(result) {
        Some(evidence) => context_from_evidence(
            evidence,
            GovernanceScoringSource::LegacyResultMapping,
            Vec::new(),
        ),
        None => fallback_context(
            GovernanceEvidenceGrade::HeuristicOnly,
            GovernanceScoringSource::HeuristicFallback,
            Vec::new(),
        ),
    }
}

fn context_from_evidence(
    evidence: GovernanceEvidence,
    scoring_source: GovernanceScoringSource,
    mut validation_errors: Vec<String>,
) -> GovernanceEvidenceContext {
    if evidence.schema_version != GOVERNANCE_EVIDENCE_SCHEMA_VERSION {
        validation_errors.push(format!(
            "unsupported schema_version: {}",
            evidence.schema_version
        ));
    }
    if evidence
        .evidence_anchors
        .iter()
        .any(|anchor| anchor.kind.trim().is_empty() || anchor.uri.trim().is_empty())
    {
        validation_errors.push("evidence anchors require non-empty kind and uri".to_string());
    }
    for (field, value) in [
        ("cooperation", evidence.cooperation),
        ("defection", evidence.defection),
    ] {
        if value.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
            validation_errors.push(format!("{field} must be finite and within 0.0..=1.0"));
        }
    }

    let mut missing_fields = Vec::new();
    if evidence.evidence_anchors.is_empty() {
        missing_fields.push("evidence_anchors".to_string());
    }
    if evidence.action_intent.trim().is_empty() {
        missing_fields.push("action_intent".to_string());
    }
    if evidence.cooperation.is_none() {
        missing_fields.push("cooperation".to_string());
    }
    if evidence.defection.is_none() {
        missing_fields.push("defection".to_string());
    }
    if evidence.disconfirming_evidence.is_empty() {
        missing_fields.push("disconfirming_evidence".to_string());
    }
    if evidence
        .risk_boundary
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        missing_fields.push("risk_boundary".to_string());
    }
    if evidence
        .fallback_path
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        missing_fields.push("fallback_path".to_string());
    }

    let grade = if validation_errors.is_empty() && missing_fields.is_empty() {
        GovernanceEvidenceGrade::StructuredValidated
    } else {
        GovernanceEvidenceGrade::StructuredPartial
    };
    GovernanceEvidenceContext {
        evidence: Some(evidence),
        assessment: GovernanceEvidenceAssessment {
            schema_version: GOVERNANCE_EVIDENCE_SCHEMA_VERSION.to_string(),
            grade,
            scoring_source,
            missing_fields,
            validation_errors,
        },
    }
}

fn fallback_context(
    grade: GovernanceEvidenceGrade,
    scoring_source: GovernanceScoringSource,
    validation_errors: Vec<String>,
) -> GovernanceEvidenceContext {
    GovernanceEvidenceContext {
        evidence: None,
        assessment: GovernanceEvidenceAssessment {
            schema_version: GOVERNANCE_EVIDENCE_SCHEMA_VERSION.to_string(),
            grade,
            scoring_source,
            missing_fields: vec![
                "evidence_anchors".to_string(),
                "action_intent".to_string(),
                "cooperation".to_string(),
                "defection".to_string(),
                "disconfirming_evidence".to_string(),
                "risk_boundary".to_string(),
                "fallback_path".to_string(),
            ],
            validation_errors,
        },
    }
}

fn map_legacy_result(result: &Value) -> Option<GovernanceEvidence> {
    let object = result.as_object()?;
    let mut anchors = Vec::new();
    if let Some(values) = object.get("evidence").and_then(Value::as_array) {
        anchors.extend(values.iter().filter_map(Value::as_str).map(|uri| {
            GovernanceEvidenceAnchor {
                kind: "legacy_evidence".to_string(),
                uri: uri.to_string(),
                claim: None,
            }
        }));
    }
    if let Some(path) = object
        .get("provenance")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
    {
        anchors.push(GovernanceEvidenceAnchor {
            kind: "provenance_path".to_string(),
            uri: path.to_string(),
            claim: None,
        });
    }
    for key in ["source", "source_id", "receipt", "receipt_id"] {
        if let Some(uri) = object.get(key).and_then(Value::as_str) {
            anchors.push(GovernanceEvidenceAnchor {
                kind: key.to_string(),
                uri: uri.to_string(),
                claim: None,
            });
        }
    }

    let action_intent =
        string_field(object, &["action_intent", "recommendation", "action"]).unwrap_or_default();
    let risk_boundary = string_field(object, &["risk_boundary", "risk"]);
    let fallback_path = string_field(object, &["fallback_path", "fallback"]);
    let disconfirming_evidence = object
        .get("disconfirming_evidence")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if anchors.is_empty()
        && action_intent.is_empty()
        && risk_boundary.is_none()
        && fallback_path.is_none()
    {
        return None;
    }

    Some(GovernanceEvidence {
        schema_version: GOVERNANCE_EVIDENCE_SCHEMA_VERSION.to_string(),
        evidence_anchors: anchors,
        action_intent,
        justified_urgency: string_field(object, &["justified_urgency", "urgency_reason"]),
        cooperation: number_field(object, "cooperation"),
        defection: number_field(object, "defection"),
        disconfirming_evidence,
        risk_boundary,
        fallback_path,
    })
}

fn string_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn number_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}
