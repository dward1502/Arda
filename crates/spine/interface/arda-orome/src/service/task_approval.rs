use super::council::redact_operating_room_body;
use super::*;
use std::collections::BTreeMap;

const TASK_APPROVAL_PROPOSAL_SCHEMA: &str = "arda.hermes.task_approval_proposal.v1";
const TASK_APPROVAL_PACKET_SCHEMA: &str = "arda.hermes.task_approval_packet.v1";

impl HermesService {
    pub fn create_task_approval_proposal(
        &self,
        scope: &str,
        risk: CommsEventRisk,
        action_summary: &str,
        task_id: Option<&str>,
        requested_by: &str,
        discord_thread_id: Option<&str>,
    ) -> Result<TaskApprovalProposal> {
        let proposal_id = format!("tap_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let task_ref = task_id
            .map(canonical_task_ref)
            .unwrap_or_else(|| format!("task:{proposal_id}"));
        let action_summary = redact_operating_room_body(action_summary);
        let mut delivery_metadata = BTreeMap::new();
        if let Some(thread_id) = discord_thread_id.and_then(non_empty_trimmed) {
            delivery_metadata.insert(
                "discord_thread_id".to_string(),
                serde_json::Value::String(thread_id.to_string()),
            );
        }
        let proposal = TaskApprovalProposal {
            schema_version: TASK_APPROVAL_PROPOSAL_SCHEMA.to_string(),
            proposal_id: proposal_id.clone(),
            task_ref: task_ref.clone(),
            scope: normalize_task_approval_scope(scope),
            risk,
            action_summary,
            requested_by: requested_by.trim().to_string(),
            canonical_refs: vec![task_ref, proposal_id],
            delivery_metadata,
            created_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.decision_prompts_path, &proposal)?;
        Ok(proposal)
    }

    pub fn render_task_approval_projection(
        &self,
        proposal: &TaskApprovalProposal,
        surface: &str,
    ) -> TaskApprovalProjection {
        let normalized_surface = normalize_projection_surface(surface);
        let delivery_metadata = if normalized_surface == "discord" {
            proposal.delivery_metadata.clone()
        } else {
            BTreeMap::new()
        };
        let title = format!("Task approval required: {}", proposal.task_ref);
        let body = format!(
            "Task: {}\nScope: {}\nRisk: {}\nAction: {}\nProposal: {}",
            proposal.task_ref,
            proposal.scope,
            format_task_risk(&proposal.risk),
            proposal.action_summary,
            proposal.proposal_id
        );
        TaskApprovalProjection {
            surface: normalized_surface,
            proposal_id: proposal.proposal_id.clone(),
            task_ref: proposal.task_ref.clone(),
            title,
            body,
            delivery_metadata,
        }
    }

    pub fn record_task_approval_packet(
        &self,
        proposal: &TaskApprovalProposal,
        approved_by: &str,
        receipt_id: &str,
    ) -> Result<TaskApprovalPacket> {
        let packet = TaskApprovalPacket {
            schema_version: TASK_APPROVAL_PACKET_SCHEMA.to_string(),
            approval_id: format!("tapr_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
            proposal_id: proposal.proposal_id.clone(),
            task_ref: proposal.task_ref.clone(),
            scope: proposal.scope.clone(),
            risk: proposal.risk.clone(),
            action_summary: proposal.action_summary.clone(),
            receipt_id: receipt_id.trim().to_string(),
            approved_by: approved_by.trim().to_string(),
            delivery_metadata: proposal.delivery_metadata.clone(),
            approved_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.decision_responses_path, &packet)?;
        self.record_decision_hop(
            "task_approval_recorded",
            "canonical",
            &proposal.scope,
            approved_by,
            Some(&proposal.proposal_id),
            None,
            Some(&proposal.action_summary),
            Some(receipt_id),
            true,
            None,
        );
        Ok(packet)
    }
}

fn canonical_task_ref(task_id: &str) -> String {
    let trimmed = task_id.trim();
    if trimmed.starts_with("task:") {
        trimmed.to_string()
    } else {
        format!("task:{trimmed}")
    }
}

fn normalize_task_approval_scope(scope: &str) -> String {
    let normalized = scope.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    if normalized.is_empty() {
        "general".to_string()
    } else {
        normalized
    }
}

fn normalize_projection_surface(surface: &str) -> String {
    match surface.trim().to_ascii_lowercase().as_str() {
        "discord" => "discord".to_string(),
        "arda" | "arda_hud" | "hud" => "arda".to_string(),
        "terminal" | "cli" | "tui" => "terminal".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "terminal".to_string(),
    }
}

fn format_task_risk(risk: &CommsEventRisk) -> &'static str {
    match risk {
        CommsEventRisk::Low => "low",
        CommsEventRisk::Medium => "medium",
        CommsEventRisk::High => "high",
    }
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
