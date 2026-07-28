//! Governed external-source MCP connector contracts.
//!
//! These records intentionally describe the safe Rust-side policy boundary for
//! external source MCP servers. They do not spawn third-party Node packages,
//! install dependencies, or perform authentication. Runtime bridges can consume
//! these contracts to expose only read/query tools into ATHENA source intake.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

use crate::mcp::tools::{McpTool, McpToolGovernance};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalSourceMcpConnector {
    pub schema_version: String,
    pub source_id: String,
    pub display_name: String,
    pub upstream_reference: String,
    pub package: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub athena_lane: AthenaLanePolicy,
    pub tool_policy: ExternalSourceToolPolicy,
    pub auth_boundary: AuthBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AthenaLanePolicy {
    pub schema_version: String,
    pub lane_id: String,
    pub source_type: String,
    pub promotion_status: String,
    pub task_promotion_allowed: bool,
    pub receipt_required_before_task: bool,
    pub evidence_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalSourceToolPolicy {
    pub mode: String,
    pub allowed_tools: Vec<String>,
    pub blocked_tools: Vec<String>,
    pub disabled_tools_env: Option<String>,
    pub network_allowed_by_default: bool,
    pub destructive_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthBoundary {
    pub auth_required: bool,
    pub credential_storage: String,
    pub forbidden_repo_artifacts: Vec<String>,
    pub explicit_user_approval_required_for: Vec<String>,
}

/// Canonical evidence receipt for the single enabled external-source pilot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalSourceReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub idempotency_key: String,
    pub source_id: String,
    pub authority: String,
    pub privacy_classification: String,
    pub canonical_url: String,
    pub captured_at_utc: String,
    pub outcome: String,
    pub failure_reason: Option<String>,
    pub evidence: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalSourceReceiptValidation {
    pub schema_version: String,
    pub source_id: String,
    pub valid: bool,
    pub task_promotion_allowed: bool,
    pub reasons: Vec<String>,
}

/// Validate the canonical Reddit pilot receipt. Other external lanes remain
/// disabled until they receive an explicit, separately reviewed validator.
pub fn validate_external_source_receipt(
    receipt: &ExternalSourceReceipt,
) -> ExternalSourceReceiptValidation {
    let connector = reddit_connector_contract();
    let mut reasons = Vec::new();
    if receipt.schema_version != "arda.athena.external_source_receipt.v1" {
        reasons.push("unsupported_schema_version".to_string());
    }
    if receipt.source_id != connector.source_id {
        reasons.push("source_lane_disabled".to_string());
    }
    if receipt.receipt_id.trim().is_empty() {
        reasons.push("missing_receipt_id".to_string());
    }
    if receipt.idempotency_key.trim().is_empty() {
        reasons.push("missing_idempotency_key".to_string());
    }
    if receipt.authority != "athena_external_source_pilot" {
        reasons.push("invalid_authority".to_string());
    }
    if receipt.privacy_classification != "public" {
        reasons.push("unsupported_privacy_classification".to_string());
    }
    if !(receipt.canonical_url.starts_with("https://")
        || receipt.canonical_url.starts_with("http://"))
    {
        reasons.push("invalid_canonical_url".to_string());
    }
    if !receipt.captured_at_utc.contains('T') {
        reasons.push("invalid_captured_at_utc".to_string());
    }
    if !matches!(receipt.outcome.as_str(), "success" | "failure") {
        reasons.push("invalid_outcome".to_string());
    }
    if receipt.outcome == "failure"
        && receipt
            .failure_reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
    {
        reasons.push("missing_failure_reason".to_string());
    }
    for required in &connector.athena_lane.evidence_required {
        if required == "captured_at_utc" {
            continue;
        }
        if receipt
            .evidence
            .get(required)
            .is_none_or(|value| value.trim().is_empty())
        {
            reasons.push(format!("missing_evidence:{required}"));
        }
    }
    let valid = reasons.is_empty();
    ExternalSourceReceiptValidation {
        schema_version: "arda.athena.external_source_receipt_validation.v1".to_string(),
        source_id: receipt.source_id.clone(),
        valid,
        task_promotion_allowed: valid && receipt.outcome == "success",
        reasons,
    }
}

pub fn external_source_mcp_connectors() -> Vec<ExternalSourceMcpConnector> {
    vec![notebooklm_connector_contract(), reddit_connector_contract()]
}

pub fn notebooklm_connector_contract() -> ExternalSourceMcpConnector {
    let mut env = BTreeMap::new();
    env.insert("HEADLESS".to_string(), "true".to_string());
    env.insert(
        "NOTEBOOKLM_BROWSER_CHANNEL".to_string(),
        "chromium".to_string(),
    );
    env.insert("NOTEBOOKLM_PROFILE".to_string(), "standard".to_string());
    env.insert(
        "NOTEBOOKLM_DISABLED_TOOLS".to_string(),
        "re_auth,cleanup_data,generate_audio,download_audio".to_string(),
    );

    ExternalSourceMcpConnector {
        schema_version: "arda.mcp.external_source_connector.v1".to_string(),
        source_id: "notebook_lm".to_string(),
        display_name: "NotebookLM MCP".to_string(),
        upstream_reference: "https://github.com/PleasePrompto/notebooklm-mcp".to_string(),
        package: "notebooklm-mcp@latest".to_string(),
        transport: "stdio".to_string(),
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "notebooklm-mcp@latest".to_string()],
        env,
        athena_lane: AthenaLanePolicy {
            schema_version: "arda.athena.external_source_lane.v1".to_string(),
            lane_id: "notebook_lm".to_string(),
            source_type: "agent_generated_synthesis".to_string(),
            promotion_status: "blocked_until_canonical_source_receipt".to_string(),
            task_promotion_allowed: false,
            receipt_required_before_task: true,
            evidence_required: vec![
                "notebook_url_or_id".to_string(),
                "question_or_query".to_string(),
                "citation_or_source_anchor".to_string(),
                "captured_at_utc".to_string(),
            ],
        },
        tool_policy: ExternalSourceToolPolicy {
            mode: "read_query_only".to_string(),
            allowed_tools: vec![
                "get_health".to_string(),
                "list_notebooks".to_string(),
                "get_notebook".to_string(),
                "search_notebooks".to_string(),
                "select_notebook".to_string(),
                "ask_question".to_string(),
                "list_sessions".to_string(),
            ],
            blocked_tools: vec![
                "add_notebook".to_string(),
                "update_notebook".to_string(),
                "add_source".to_string(),
                "setup_auth".to_string(),
                "re_auth".to_string(),
                "cleanup_data".to_string(),
                "generate_audio".to_string(),
                "get_audio_status".to_string(),
                "download_audio".to_string(),
                "close_session".to_string(),
                "reset_session".to_string(),
            ],
            disabled_tools_env: Some("NOTEBOOKLM_DISABLED_TOOLS".to_string()),
            network_allowed_by_default: false,
            destructive_allowed: false,
        },
        auth_boundary: AuthBoundary {
            auth_required: true,
            credential_storage: "local browser profile outside the repo".to_string(),
            forbidden_repo_artifacts: vec![
                "Google passwords".to_string(),
                "raw cookies".to_string(),
                "OAuth tokens".to_string(),
                "browser session exports".to_string(),
            ],
            explicit_user_approval_required_for: vec![
                "visible-browser setup_auth".to_string(),
                "re_auth".to_string(),
                "adding or updating NotebookLM sources".to_string(),
                "audio generation or download".to_string(),
            ],
        },
    }
}

pub fn reddit_connector_contract() -> ExternalSourceMcpConnector {
    ExternalSourceMcpConnector {
        schema_version: "arda.mcp.external_source_connector.v1".to_string(),
        source_id: "reddit".to_string(),
        display_name: "Reddit MCP".to_string(),
        upstream_reference: "https://github.com/jordanburke/reddit-mcp-server".to_string(),
        package: "reddit-mcp-server".to_string(),
        transport: "stdio".to_string(),
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "reddit-mcp-server".to_string()],
        env: BTreeMap::new(),
        athena_lane: AthenaLanePolicy {
            schema_version: "arda.athena.external_source_lane.v1".to_string(),
            lane_id: "reddit".to_string(),
            source_type: "public_forum_context".to_string(),
            promotion_status: "blocked_until_canonical_source_receipt".to_string(),
            task_promotion_allowed: false,
            receipt_required_before_task: true,
            evidence_required: vec![
                "subreddit_or_post_url".to_string(),
                "query_or_sort".to_string(),
                "post_or_comment_ids".to_string(),
                "captured_at_utc".to_string(),
            ],
        },
        tool_policy: ExternalSourceToolPolicy {
            mode: "read_only".to_string(),
            allowed_tools: vec![
                "get_reddit_post".to_string(),
                "get_top_posts".to_string(),
                "browse_subreddit".to_string(),
                "get_user_info".to_string(),
                "get_user_posts".to_string(),
                "get_user_comments".to_string(),
                "get_subreddit_info".to_string(),
                "get_trending_subreddits".to_string(),
                "get_post_comments".to_string(),
                "search_reddit".to_string(),
            ],
            blocked_tools: vec![
                "create_post".to_string(),
                "reply_to_post".to_string(),
                "edit_post".to_string(),
                "edit_comment".to_string(),
                "delete_post".to_string(),
                "delete_comment".to_string(),
            ],
            disabled_tools_env: None,
            network_allowed_by_default: false,
            destructive_allowed: false,
        },
        auth_boundary: AuthBoundary {
            auth_required: false,
            credential_storage:
                "anonymous read mode preferred; user credentials only after explicit approval"
                    .to_string(),
            forbidden_repo_artifacts: vec![
                "Reddit client secrets".to_string(),
                "Reddit passwords".to_string(),
                "refresh tokens".to_string(),
            ],
            explicit_user_approval_required_for: vec![
                "authenticated Reddit credentials".to_string(),
                "creating posts or comments".to_string(),
                "editing or deleting content".to_string(),
            ],
        },
    }
}

pub fn external_source_connector_tool() -> McpTool {
    McpTool::new(
        "external_source.connectors.catalog",
        "Return Arda-governed MCP connector contracts for ATHENA external-source intake.",
    )
    .with_schema(json!({
        "type": "object",
        "properties": {
            "source_id": {
                "type": "string",
                "enum": ["notebook_lm", "reddit"]
            }
        }
    }))
    .with_governance(McpToolGovernance {
        schema_version: "arda.mcp.tool.v1".to_string(),
        sigil: "∇◈".to_string(),
        requires_approval: false,
        allows_network: false,
        allows_destructive: false,
        triad_required: true,
    })
}

pub fn connector_catalog_response(source_id: Option<&str>) -> serde_json::Value {
    let connectors: Vec<_> = external_source_mcp_connectors()
        .into_iter()
        .filter(|connector| {
            source_id
                .map(|requested| connector.source_id == requested)
                .unwrap_or(true)
        })
        .collect();

    json!({
        "schema_version": "arda.mcp.external_source_catalog.v1",
        "task_promotion_allowed": false,
        "network_performed_by_catalog_tool": false,
        "connectors": connectors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_reddit_receipt() -> ExternalSourceReceipt {
        ExternalSourceReceipt {
            schema_version: "arda.athena.external_source_receipt.v1".to_string(),
            receipt_id: "receipt-1".to_string(),
            idempotency_key: "reddit:example:2026-07-27".to_string(),
            source_id: "reddit".to_string(),
            authority: "athena_external_source_pilot".to_string(),
            privacy_classification: "public".to_string(),
            canonical_url: "https://reddit.com/r/rust/comments/example".to_string(),
            captured_at_utc: "2026-07-27T00:00:00Z".to_string(),
            outcome: "success".to_string(),
            failure_reason: None,
            evidence: BTreeMap::from([
                (
                    "subreddit_or_post_url".to_string(),
                    "https://reddit.com/r/rust/comments/example".to_string(),
                ),
                ("query_or_sort".to_string(), "top/week".to_string()),
                ("post_or_comment_ids".to_string(), "example".to_string()),
            ]),
        }
    }

    #[test]
    fn canonical_reddit_receipt_enables_only_the_validated_pilot() {
        let receipt = complete_reddit_receipt();
        let validation = validate_external_source_receipt(&receipt);
        assert!(validation.valid);
        assert!(validation.task_promotion_allowed);

        let mut notebooklm = receipt;
        notebooklm.source_id = "notebook_lm".to_string();
        let blocked = validate_external_source_receipt(&notebooklm);
        assert!(!blocked.valid);
        assert!(!blocked.task_promotion_allowed);
        assert!(blocked
            .reasons
            .contains(&"source_lane_disabled".to_string()));
    }

    #[test]
    fn incomplete_reddit_receipt_cannot_promote_tasks() {
        let mut receipt = complete_reddit_receipt();
        receipt.evidence.clear();
        let validation = validate_external_source_receipt(&receipt);
        assert!(!validation.valid);
        assert!(!validation.task_promotion_allowed);
        assert!(validation
            .reasons
            .iter()
            .any(|reason| reason.starts_with("missing_evidence:")));
    }

    #[test]
    fn well_formed_failure_receipt_is_auditable_but_cannot_promote() {
        let mut receipt = complete_reddit_receipt();
        receipt.outcome = "failure".to_string();
        receipt.failure_reason = Some("upstream_timeout".to_string());
        let validation = validate_external_source_receipt(&receipt);
        assert!(validation.valid);
        assert!(!validation.task_promotion_allowed);
    }
}
