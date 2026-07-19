#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Read-only Arandur planner boundary for normalized objective packets.

use super::governance_policy::GovernanceGate;
use super::source_registry::{ArandurSourceType, SourceRegistry};
use serde::Serialize;

pub const OBJECTIVE_PACKET_CONTRACT: &str = "arda.arandur.objective_packet.v1";
pub const OBJECTIVE_PACKET_REPORT_CONTRACT: &str = "arda.arandur.objective_packet_report.v1";

#[derive(Debug, Clone, Serialize)]
pub struct ObjectivePacket {
    pub contract: String,
    pub packet_id: String,
    pub source_contract: String,
    pub source_type: ArandurSourceType,
    pub source_path: String,
    pub source_record_id: String,
    pub candidate_id: String,
    pub title: String,
    pub owner: Option<String>,
    pub priority: Option<String>,
    pub governance_class: String,
    pub review_gate: GovernanceGate,
    pub acceptance_criteria: Vec<String>,
    pub approval_packet_id: Option<String>,
    pub completion_receipt_path: Option<String>,
    pub blocked_reason_code: Option<String>,
    pub selected: bool,
    pub canonical_queue_mutation_allowed: bool,
}

#[derive(Debug, Clone)]
pub struct ObjectivePacketInput {
    pub source_path: String,
    pub source_record_id: String,
    pub candidate_id: String,
    pub title: String,
    pub owner: Option<String>,
    pub priority: Option<String>,
    pub governance_class: String,
    pub review_gate: GovernanceGate,
    pub acceptance_criteria: Vec<String>,
    pub approval_packet_id: Option<String>,
    pub completion_receipt_path: Option<String>,
    pub blocked_reason_code: Option<String>,
    pub selected: bool,
}

impl ObjectivePacket {
    pub fn from_report(
        source_contract: impl Into<String>,
        source_type: ArandurSourceType,
        input: ObjectivePacketInput,
    ) -> Self {
        let packet_id = format!(
            "objective_packet:{}:{}",
            input.source_record_id, input.candidate_id
        );
        Self {
            contract: OBJECTIVE_PACKET_CONTRACT.into(),
            packet_id,
            source_contract: source_contract.into(),
            source_type,
            source_path: input.source_path,
            source_record_id: input.source_record_id,
            candidate_id: input.candidate_id,
            title: input.title,
            owner: input.owner,
            priority: input.priority,
            governance_class: input.governance_class,
            review_gate: input.review_gate,
            acceptance_criteria: input.acceptance_criteria,
            approval_packet_id: input.approval_packet_id,
            completion_receipt_path: input.completion_receipt_path,
            blocked_reason_code: input.blocked_reason_code,
            selected: input.selected,
            canonical_queue_mutation_allowed: false,
        }
    }

    #[cfg(test)]
    fn test_packet(candidate_id: &str, review_gate: GovernanceGate, selected: bool) -> Self {
        Self::from_report(
            "arda.test.source.v1",
            ArandurSourceType::Unknown,
            ObjectivePacketInput {
                source_path: "/tmp/source.jsonl".into(),
                source_record_id: candidate_id.into(),
                candidate_id: candidate_id.into(),
                title: format!("Candidate {candidate_id}"),
                owner: Some("prometheus".into()),
                priority: Some("medium".into()),
                governance_class: "test".into(),
                review_gate,
                acceptance_criteria: Vec::new(),
                approval_packet_id: None,
                completion_receipt_path: None,
                blocked_reason_code: if selected {
                    None
                } else {
                    Some("blocked_for_test".into())
                },
                selected,
            },
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectivePacketReport {
    pub contract: String,
    pub mutation_policy: String,
    pub packet_count: usize,
    pub selected_packet_count: usize,
    pub selected_candidate_id: Option<String>,
    pub canonical_queue_mutation_allowed: bool,
    pub packets: Vec<ObjectivePacket>,
}

impl ObjectivePacketReport {
    pub fn read_only(packets: Vec<ObjectivePacket>) -> Self {
        let selected_packet_count = packets.iter().filter(|packet| packet.selected).count();
        let selected_candidate_id = if selected_packet_count == 1 {
            packets
                .iter()
                .find(|packet| packet.selected)
                .map(|packet| packet.candidate_id.clone())
        } else {
            None
        };
        Self {
            contract: OBJECTIVE_PACKET_REPORT_CONTRACT.into(),
            mutation_policy: "read_only_report_only".into(),
            packet_count: packets.len(),
            selected_packet_count,
            selected_candidate_id,
            canonical_queue_mutation_allowed: false,
            packets,
        }
    }
}

pub fn acceptance_criteria_from_report(
    blocked_reason_code: Option<&str>,
    rejection_reason: Option<&str>,
    selected_reason: Option<&str>,
    completion_receipt_path: Option<&str>,
) -> Vec<String> {
    let mut criteria = Vec::new();
    if let Some(reason) = blocked_reason_code.filter(|reason| !reason.trim().is_empty()) {
        criteria.push(format!("resolve_blocked_reason:{reason}"));
    }
    if let Some(reason) = rejection_reason.filter(|reason| !reason.trim().is_empty()) {
        criteria.push(format!("selection_status:{reason}"));
    }
    if let Some(reason) = selected_reason.filter(|reason| !reason.trim().is_empty()) {
        criteria.push(format!("selected_reason:{reason}"));
    }
    if let Some(path) = completion_receipt_path.filter(|path| !path.trim().is_empty()) {
        criteria.push(format!("completion_receipt:{path}"));
    }
    if criteria.is_empty() {
        criteria.push("packet_normalized_for_operator_review".into());
    }
    criteria
}

pub fn source_contract_and_type_for_path(
    source_registry: &SourceRegistry,
    source_path: &str,
) -> (String, ArandurSourceType) {
    source_registry
        .sources
        .iter()
        .find(|source| source.path.to_string_lossy() == source_path)
        .map(|source| (source.contract.clone(), source.source_type.clone()))
        .unwrap_or_else(|| ("unknown".into(), ArandurSourceType::Unknown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_normalizes_candidate_report_into_objective_packet_contract() {
        let packet = ObjectivePacket::from_report(
            "arda.arandur.recommendations.v1",
            ArandurSourceType::ArandurRecommendation,
            ObjectivePacketInput {
                source_path: "/repo/data/arandur/recommendations.jsonl".into(),
                source_record_id: "reco-1".into(),
                candidate_id: "cand-1".into(),
                title: "Validate autonomy gate".into(),
                owner: Some("prometheus".into()),
                priority: Some("high".into()),
                governance_class: "arandur_recommendation".into(),
                review_gate: GovernanceGate::ReviewRequired,
                acceptance_criteria: vec!["operator approval packet is present".into()],
                approval_packet_id: None,
                completion_receipt_path: None,
                blocked_reason_code: Some(
                    "review_gated_recommendation_requires_operator_review".into(),
                ),
                selected: false,
            },
        );

        assert_eq!(packet.contract, OBJECTIVE_PACKET_CONTRACT);
        assert_eq!(
            packet.source_contract,
            "arda.arandur.recommendations.v1"
        );
        assert_eq!(packet.source_type, ArandurSourceType::ArandurRecommendation);
        assert_eq!(packet.source_record_id, "reco-1");
        assert_eq!(packet.candidate_id, "cand-1");
        assert_eq!(packet.review_gate, GovernanceGate::ReviewRequired);
        assert_eq!(
            packet.blocked_reason_code.as_deref(),
            Some("review_gated_recommendation_requires_operator_review")
        );
        assert!(!packet.canonical_queue_mutation_allowed);
    }

    #[test]
    fn read_only_packet_report_surface_is_report_only_and_preserves_exactly_one_selection() {
        let packets = vec![
            ObjectivePacket::test_packet("selected", GovernanceGate::SafeAutonomous, true),
            ObjectivePacket::test_packet("blocked", GovernanceGate::HumanRequired, false),
        ];

        let report = ObjectivePacketReport::read_only(packets);

        assert_eq!(report.contract, OBJECTIVE_PACKET_REPORT_CONTRACT);
        assert_eq!(report.mutation_policy, "read_only_report_only");
        assert_eq!(report.packet_count, 2);
        assert_eq!(report.selected_packet_count, 1);
        assert_eq!(report.selected_candidate_id.as_deref(), Some("selected"));
        assert!(!report.canonical_queue_mutation_allowed);
    }
}
