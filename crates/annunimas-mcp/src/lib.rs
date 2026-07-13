// sigil: REPAIR
//! Annunimas MCP - Model Context Protocol exposure for agents
//!
//! Exposes Annunimas agents as MCP tools for external access

pub mod browser;
pub mod external_sources;
pub mod protocol;
pub mod server;
pub mod tools;

pub use browser::{playwright_bridge_contract, playwright_tool, PlaywrightBridgeContract};
pub use external_sources::{
    connector_catalog_response, external_source_connector_tool, external_source_mcp_connectors,
    notebooklm_connector_contract, reddit_connector_contract, ExternalSourceMcpConnector,
};
pub use server::McpServer;
pub use tools::{McpMethod, McpRequest, McpResponse, McpTool, ToolRegistry};

/// Initialize MCP server with agent registry
pub async fn init_mcp_server() -> McpServer {
    let server = McpServer::new();
    let contract = playwright_bridge_contract();
    let external_source_catalog = connector_catalog_response(None);
    server
        .register_resource(
            "bridge://playwright/contract",
            serde_json::to_value(&contract).unwrap_or_else(
                |_| serde_json::json!({ "error": "playwright contract serialization failed" }),
            ),
        )
        .await;
    server
        .register_tool(playwright_tool(), move |payload| {
            serde_json::json!({
                "bridge": "playwright-mcp",
                "status": "contract_defined",
                "payload": payload,
                "contract": contract,
            })
        })
        .await;
    server
        .register_resource("mcp://external-sources/connectors", external_source_catalog)
        .await;
    server
        .register_tool(external_source_connector_tool(), |payload| {
            let source_id = payload.get("source_id").and_then(|value| value.as_str());
            connector_catalog_response(source_id)
        })
        .await;
    server
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn init_server_registers_playwright_bridge_surfaces() {
        let server = init_mcp_server().await;

        let tools = server.tools().list().await;
        assert!(tools
            .iter()
            .any(|tool| tool.name == "browser.playwright.session"));
        assert!(tools
            .iter()
            .any(|tool| tool.name == "external_source.connectors.catalog"));

        let response = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "resources/read".to_string(),
                params: Some(json!({ "uri": "bridge://playwright/contract" })),
            })
            .await;
        assert!(response.error.is_none());
        let text = response
            .result
            .as_ref()
            .and_then(|v| v.get("contents"))
            .and_then(|v| v.as_array())
            .and_then(|items| items.first())
            .and_then(|v| v.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(text.contains("annunimas.mcp.playwright.v1"));
        assert!(text.contains("npx @playwright/mcp@latest --stdio"));
    }
}
