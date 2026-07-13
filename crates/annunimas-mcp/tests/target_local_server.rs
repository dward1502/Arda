use annunimas_mcp::tools::McpToolGovernance;
use annunimas_mcp::{
    connector_catalog_response, init_mcp_server, notebooklm_connector_contract,
    playwright_bridge_contract, reddit_connector_contract, McpRequest, McpResponse, McpServer,
    McpTool,
};
use serde_json::json;

fn required_governance() -> serde_json::Value {
    json!({
        "sigil": "∇◈",
        "approval_token": "approved-by-warden",
        "network_allowed": true,
        "triad": { "status": "pass" }
    })
}

fn error_code(response: &McpResponse) -> Option<i32> {
    response.error.as_ref().map(|error| error.code)
}

#[tokio::test]
async fn initialized_server_exposes_playwright_bridge_contract_and_tool() -> Result<(), String> {
    let server = init_mcp_server().await;

    let tools = server.tools().list().await;
    assert!(tools
        .iter()
        .any(|tool| tool.name == "browser.playwright.session"));

    let contract = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!("contract")),
            method: "resources/read".to_owned(),
            params: Some(json!({ "uri": "bridge://playwright/contract" })),
        })
        .await;

    assert!(contract.error.is_none());
    let text = contract
        .result
        .as_ref()
        .and_then(|value| value.get("contents"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|value| value.get("text"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| "contract response omitted content text".to_owned())?;
    assert!(text.contains("annunimas.mcp.playwright.v1"));
    assert!(text.contains("npx @playwright/mcp@latest --stdio"));

    Ok(())
}

#[tokio::test]
async fn tool_calls_require_matching_governance_before_dispatch() {
    let server = McpServer::new();
    server
        .register_tool(
            McpTool::new("browser.audit", "Governed browser audit").with_governance(
                McpToolGovernance {
                    requires_approval: true,
                    allows_network: true,
                    ..Default::default()
                },
            ),
            |payload| json!({ "accepted": payload }),
        )
        .await;

    let denied = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(1)),
            method: "tools/call".to_owned(),
            params: Some(json!({
                "name": "browser.audit",
                "arguments": { "target": "https://example.invalid" },
                "governance": { "sigil": "wrong" }
            })),
        })
        .await;
    assert_eq!(error_code(&denied), Some(-32602));

    let allowed = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(2)),
            method: "tools/call".to_owned(),
            params: Some(json!({
                "name": "browser.audit",
                "arguments": { "target": "https://example.invalid" },
                "governance": required_governance()
            })),
        })
        .await;
    assert!(allowed.error.is_none());
    assert_eq!(
        allowed
            .result
            .as_ref()
            .and_then(|value| value.get("governance"))
            .and_then(|value| value.get("requires_approval"))
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[tokio::test]
async fn resource_and_prompt_surfaces_return_json_rpc_success_shapes() {
    let server = McpServer::new();
    server
        .register_resource("core://audit/status", json!({ "active_blockers": 0 }))
        .await;
    server
        .register_prompt("audit_summary", "Summarize the current audit state")
        .await;

    let resources = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(3)),
            method: "resources/list".to_owned(),
            params: None,
        })
        .await;
    assert!(resources.error.is_none());
    assert_eq!(
        resources
            .result
            .as_ref()
            .and_then(|value| value.get("resources"))
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let prompts = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(4)),
            method: "prompts/list".to_owned(),
            params: None,
        })
        .await;
    assert!(prompts.error.is_none());
    assert_eq!(
        prompts
            .result
            .as_ref()
            .and_then(|value| value.get("prompts"))
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );
}

#[test]
fn playwright_bridge_contract_uses_portable_default_state_paths() {
    let contract = playwright_bridge_contract();

    assert_eq!(contract.schema_version, "annunimas.mcp.playwright.v1");
    assert_eq!(contract.runtime, "stdio_subprocess");
    assert_eq!(contract.profile_dir, "data/mcp/playwright/profile");
    assert_eq!(contract.artifact_dir, "data/mcp/playwright/artifacts");
    assert_eq!(contract.log_path, "data/mcp/playwright/bridge.log");
    assert_eq!(contract.pid_path, "data/mcp/playwright/bridge.pid");
    assert_eq!(
        contract.approvals["network_allowed_flag_required"].as_bool(),
        Some(true)
    );
}

#[test]
fn external_source_connector_contracts_are_read_only_and_athena_gated() {
    let notebooklm = notebooklm_connector_contract();
    assert_eq!(notebooklm.source_id, "notebook_lm");
    assert_eq!(notebooklm.command, "npx");
    assert!(notebooklm
        .args
        .iter()
        .any(|arg| arg == "notebooklm-mcp@latest"));
    assert!(notebooklm
        .tool_policy
        .allowed_tools
        .iter()
        .any(|tool| tool == "ask_question"));
    assert!(notebooklm
        .tool_policy
        .blocked_tools
        .iter()
        .any(|tool| tool == "add_source"));
    assert_eq!(notebooklm.tool_policy.destructive_allowed, false);
    assert_eq!(notebooklm.athena_lane.task_promotion_allowed, false);
    assert_eq!(notebooklm.athena_lane.receipt_required_before_task, true);

    let reddit = reddit_connector_contract();
    assert_eq!(reddit.source_id, "reddit");
    assert!(reddit
        .tool_policy
        .allowed_tools
        .iter()
        .any(|tool| tool == "search_reddit"));
    assert!(reddit
        .tool_policy
        .blocked_tools
        .iter()
        .any(|tool| tool == "create_post"));
    assert_eq!(reddit.tool_policy.network_allowed_by_default, false);
    assert_eq!(reddit.athena_lane.task_promotion_allowed, false);
}

#[tokio::test]
async fn initialized_server_exposes_external_source_connector_catalog() -> Result<(), String> {
    let server = init_mcp_server().await;
    let response = server
        .handle_request(McpRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!("external-source-catalog")),
            method: "resources/read".to_owned(),
            params: Some(json!({ "uri": "mcp://external-sources/connectors" })),
        })
        .await;

    assert!(response.error.is_none());
    let text = response
        .result
        .as_ref()
        .and_then(|value| value.get("contents"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|value| value.get("text"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| "connector catalog resource omitted content text".to_owned())?;
    assert!(text.contains("annunimas.mcp.external_source_catalog.v1"));
    assert!(text.contains("notebooklm-mcp@latest"));
    assert!(text.contains("reddit-mcp-server"));

    Ok(())
}

#[test]
fn connector_catalog_response_can_filter_to_one_source() {
    let catalog = connector_catalog_response(Some("reddit"));
    let connectors_len = catalog
        .get("connectors")
        .and_then(|value| value.as_array())
        .map(|items| items.len());
    assert_eq!(connectors_len, Some(1));
    assert_eq!(
        catalog
            .get("connectors")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("source_id"))
            .and_then(|value| value.as_str()),
        Some("reddit")
    );
    assert_eq!(
        catalog
            .get("task_promotion_allowed")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
}
