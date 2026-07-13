// sigil: REPAIR
use std::collections::HashMap;
use std::str::FromStr;

use crate::tools::{McpError, McpMethod, McpRequest, McpResponse, McpTool, ToolRegistry};
use annunimas_core::task::Task;
use annunimas_governance::{bacon_lite_validate, triad_validate};
use annunimas_plutus::{JouleWorkUnit, LoveEquation, PlutusService};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct McpServer {
    tools: Arc<ToolRegistry>,
    resources: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    prompts: Arc<RwLock<HashMap<String, String>>>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(ToolRegistry::new()),
            resources: Arc::new(RwLock::new(HashMap::new())),
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn tools(&self) -> Arc<ToolRegistry> {
        self.tools.clone()
    }

    pub async fn register_tool(
        &self,
        tool: McpTool,
        handler: impl Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    ) {
        self.tools.register(tool, Arc::new(handler)).await;
    }

    pub async fn register_resource(&self, uri: impl Into<String>, content: serde_json::Value) {
        let mut resources = self.resources.write().await;
        resources.insert(uri.into(), content);
    }

    pub async fn register_prompt(&self, name: impl Into<String>, template: impl Into<String>) {
        let mut prompts = self.prompts.write().await;
        prompts.insert(name.into(), template.into());
    }

    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        let id = request.id;

        let method = match McpMethod::from_str(&request.method) {
            Ok(m) => m,
            Err(_) => {
                return McpResponse::error(id, McpError::method_not_found(&request.method));
            }
        };

        match method {
            McpMethod::Initialize => {
                let result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "annunimas-mcp",
                        "version": "0.1.0"
                    },
                    "governance": {
                        "schema_version": "annunimas.mcp.v1",
                        "sigil": "∇◈",
                        "policy_gate": {
                            "approval_token_supported": true,
                            "destructive_calls_require_approval": true,
                            "network_calls_require_explicit_allow": true,
                            "triad_metadata_supported": true
                        }
                    }
                });
                McpResponse::success(id, result)
            }
            McpMethod::ToolsList => {
                let tools = self.tools.list().await;
                let result = serde_json::json!({
                    "tools": tools
                });
                McpResponse::success(id, result)
            }
            McpMethod::ToolsCall => {
                let params = request.params.unwrap_or_default();
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(Default::default()));

                match self.tools.get(name).await {
                    Some(tool) => {
                        if let Err(error) = validate_governance(&tool, &params) {
                            return McpResponse::error(id, error);
                        }
                        match self.tools.call(name, args).await {
                            Some(result) => {
                                let runtime_governance =
                                    evaluate_runtime_governance(name, &tool, &params);
                                emit_mcp_work_signal_background(name, &runtime_governance);
                                McpResponse::success(
                                    id,
                                    serde_json::json!({
                                        "tool": name,
                                        "result": result,
                                        "governance": tool.governance,
                                        "runtime_governance": runtime_governance
                                    }),
                                )
                            }
                            None => McpResponse::error(
                                id,
                                McpError::method_not_found(format!("Tool not found: {}", name)),
                            ),
                        }
                    }
                    None => McpResponse::error(
                        id,
                        McpError::method_not_found(format!("Tool not found: {}", name)),
                    ),
                }
            }
            McpMethod::ResourcesList => {
                let resources = self.resources.read().await;
                let list: Vec<_> = resources
                    .keys()
                    .map(|k| {
                        serde_json::json!({
                            "uri": k,
                            "name": k,
                            "description": format!("Resource: {}", k)
                        })
                    })
                    .collect();
                let result = serde_json::json!({ "resources": list });
                McpResponse::success(id, result)
            }
            McpMethod::ResourcesRead => {
                let params = request.params.unwrap_or_default();
                let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");

                let resources = self.resources.read().await;
                match resources.get(uri) {
                    Some(value) => {
                        let result = serde_json::json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": value.to_string()
                            }]
                        });
                        McpResponse::success(id, result)
                    }
                    None => McpResponse::error(
                        id,
                        McpError::method_not_found(format!("Resource not found: {}", uri)),
                    ),
                }
            }
            McpMethod::PromptList => {
                let prompts = self.prompts.read().await;
                let list: Vec<_> = prompts
                    .keys()
                    .map(|k| {
                        serde_json::json!({
                            "name": k,
                            "description": format!("Prompt: {}", k)
                        })
                    })
                    .collect();
                let result = serde_json::json!({ "prompts": list });
                McpResponse::success(id, result)
            }
            _ => McpResponse::error(id, McpError::method_not_found(&request.method)),
        }
    }

    pub async fn handle_json(&self, json: &str) -> Result<McpResponse, serde_json::Error> {
        let request: McpRequest = serde_json::from_str(json)?;
        Ok(self.handle_request(request).await)
    }
}

fn validate_governance(tool: &McpTool, params: &serde_json::Value) -> Result<(), McpError> {
    let governance = params.get("governance").cloned().unwrap_or_default();
    let sigil = governance
        .get("sigil")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if sigil != tool.governance.sigil {
        return Err(McpError::invalid_params(format!(
            "governance sigil mismatch for tool {}",
            tool.name
        )));
    }

    if tool.governance.requires_approval
        && governance
            .get("approval_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(McpError::invalid_params(format!(
            "approval token required for tool {}",
            tool.name
        )));
    }

    if tool.governance.allows_network
        && !governance
            .get("network_allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        return Err(McpError::invalid_params(format!(
            "network allowance required for tool {}",
            tool.name
        )));
    }

    if tool.governance.allows_destructive
        && !governance
            .get("destructive_allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        return Err(McpError::invalid_params(format!(
            "destructive allowance required for tool {}",
            tool.name
        )));
    }

    if tool.governance.triad_required
        && governance
            .get("triad")
            .and_then(|v| v.as_object())
            .is_none()
    {
        return Err(McpError::invalid_params(format!(
            "triad metadata required for tool {}",
            tool.name
        )));
    }

    Ok(())
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

fn evaluate_runtime_governance(
    tool_name: &str,
    tool: &McpTool,
    params: &serde_json::Value,
) -> serde_json::Value {
    let mut task = Task::new(
        format!(
            "call MCP tool {} because external access requested",
            tool_name
        ),
        "tool_call",
    );
    task.clarifications_resolved = if params.get("governance").is_some() {
        1
    } else {
        0
    };
    task.joule_cost_estimated = if tool.governance.requires_approval {
        1.2
    } else {
        0.7
    };
    task.joule_cost_actual = if tool.governance.allows_network {
        1.1
    } else {
        0.6
    };
    let triad = triad_validate(&task, None);
    let bacon_lite = bacon_lite_validate(&task);
    let resonance = if tool.governance.allows_network {
        0.74
    } else {
        0.61
    };
    let attention = bacon_lite.confidence.clamp(0.0, 1.0);
    let reciprocity = if triad.passed { 0.76 } else { 0.42 };
    let love_score =
        LoveEquation::new().calculate("mcp", tool_name, resonance, attention, reciprocity);
    serde_json::json!({
        "triad_passed": triad.passed,
        "triad_scores": {
            "aurelius": triad.aurelius_score,
            "bacon": triad.bacon_score,
            "sun_tzu": triad.sun_tzu_score
        },
        "bacon_lite": {
            "passed": bacon_lite.passed,
            "confidence": bacon_lite.confidence
        },
        "love_equation_guard": {
            "resonance": resonance,
            "attention": attention,
            "reciprocity": reciprocity,
            "score": love_score
        },
        "joulework_estimated": task.joule_cost_actual
    })
}

fn emit_mcp_work_signal_background(tool_name: &str, runtime_governance: &serde_json::Value) {
    let tool_name = tool_name.to_string();
    let amount = runtime_governance
        .get("joulework_estimated")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5)
        .max(0.25);
    tokio::spawn(async move {
        if let Ok(plutus) = PlutusService::from_default_or_workspace_fallback() {
            let _ = plutus
                .track_work(
                    "mcp",
                    amount,
                    JouleWorkUnit::Reasoning,
                    Some(format!("tool:{tool_name}")),
                )
                .await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{McpRequest, McpToolGovernance};
    use serde_json::json;

    #[tokio::test]
    async fn initialize_returns_server_capabilities() {
        let server = McpServer::new();
        let response = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "initialize".to_string(),
                params: None,
            })
            .await;

        assert!(response.error.is_none());
        assert_eq!(
            response
                .result
                .as_ref()
                .and_then(|v| v.get("serverInfo"))
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("annunimas-mcp")
        );
        assert_eq!(
            response
                .result
                .as_ref()
                .and_then(|v| v.get("governance"))
                .and_then(|v| v.get("sigil"))
                .and_then(|v| v.as_str()),
            Some("∇◈")
        );
    }

    #[tokio::test]
    async fn tool_registry_and_call_round_trip() {
        let server = McpServer::new();
        server
            .register_tool(
                McpTool::new("echo", "Echo input"),
                |payload| json!({ "echo": payload }),
            )
            .await;

        let list = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(2)),
                method: "tools/list".to_string(),
                params: None,
            })
            .await;
        assert!(list.error.is_none());
        assert_eq!(
            list.result
                .as_ref()
                .and_then(|v| v.get("tools"))
                .and_then(|v| v.as_array())
                .map(|items| items.len()),
            Some(1)
        );

        let call = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(3)),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "echo",
                    "arguments": { "msg": "hello" },
                    "governance": {
                        "sigil": "∇◈",
                        "triad": { "status": "pass" }
                    }
                })),
            })
            .await;
        assert!(call.error.is_none());
        assert_eq!(
            call.result
                .as_ref()
                .and_then(|v| v.get("result"))
                .and_then(|v| v.get("echo"))
                .and_then(|v| v.get("msg"))
                .and_then(|v| v.as_str()),
            Some("hello")
        );
    }

    #[tokio::test]
    async fn governance_blocks_missing_approval_or_triad_metadata() {
        let server = McpServer::new();
        server
            .register_tool(
                McpTool::new("danger", "Sensitive tool").with_governance(McpToolGovernance {
                    requires_approval: true,
                    allows_network: true,
                    ..Default::default()
                }),
                |_payload| json!({ "ok": true }),
            )
            .await;

        let denied = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(9)),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "danger",
                    "arguments": {}
                })),
            })
            .await;
        assert!(denied.result.is_none());
        assert_eq!(denied.error.as_ref().map(|e| e.code), Some(-32602));

        let allowed = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(10)),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "danger",
                    "arguments": {},
                    "governance": {
                        "sigil": "∇◈",
                        "approval_token": "approved-by-warden",
                        "network_allowed": true,
                        "triad": { "status": "pass" }
                    }
                })),
            })
            .await;
        assert!(allowed.error.is_none());
    }

    #[tokio::test]
    async fn resources_and_prompts_are_listed_and_read() {
        let server = McpServer::new();
        server
            .register_resource("core://state/world", json!({"status":"READY"}))
            .await;
        server
            .register_prompt("operator_summary", "Summarize system status")
            .await;

        let resources = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(4)),
                method: "resources/list".to_string(),
                params: None,
            })
            .await;
        assert!(resources.error.is_none());

        let read = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(5)),
                method: "resources/read".to_string(),
                params: Some(json!({ "uri": "core://state/world" })),
            })
            .await;
        assert!(read.error.is_none());
        assert!(read
            .result
            .as_ref()
            .and_then(|v| v.get("contents"))
            .and_then(|v| v.as_array())
            .is_some());

        let prompts = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(6)),
                method: "prompts/list".to_string(),
                params: None,
            })
            .await;
        assert!(prompts.error.is_none());
        assert_eq!(
            prompts
                .result
                .as_ref()
                .and_then(|v| v.get("prompts"))
                .and_then(|v| v.as_array())
                .map(|items| items.len()),
            Some(1)
        );
    }

    #[tokio::test]
    async fn invalid_methods_and_missing_tools_return_errors() {
        let server = McpServer::new();

        let invalid = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(7)),
                method: "nope".to_string(),
                params: None,
            })
            .await;
        assert!(invalid.result.is_none());
        assert_eq!(invalid.error.as_ref().map(|e| e.code), Some(-32601));

        let missing_tool = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(8)),
                method: "tools/call".to_string(),
                params: Some(json!({ "name": "missing" })),
            })
            .await;
        assert!(missing_tool.result.is_none());
        assert_eq!(missing_tool.error.as_ref().map(|e| e.code), Some(-32601));
    }
}
