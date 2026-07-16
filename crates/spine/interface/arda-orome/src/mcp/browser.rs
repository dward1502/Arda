// sigil: REPAIR
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use crate::mcp::tools::{McpTool, McpToolGovernance};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaywrightBridgeContract {
    pub schema_version: String,
    pub runtime: String,
    pub command: String,
    pub profile_dir: String,
    pub artifact_dir: String,
    pub log_path: String,
    pub pid_path: String,
    pub isolation: serde_json::Value,
    pub approvals: serde_json::Value,
}

pub fn playwright_bridge_contract() -> PlaywrightBridgeContract {
    let state_root = env::var("ARDA_PLAYWRIGHT_STATE_DIR")
        .unwrap_or_else(|_| "data/mcp/playwright".to_string());
    PlaywrightBridgeContract {
        schema_version: "arda.mcp.playwright.v1".to_string(),
        runtime: "stdio_subprocess".to_string(),
        command: env::var("ARDA_PLAYWRIGHT_MCP_CMD")
            .unwrap_or_else(|_| "npx @playwright/mcp@latest --stdio".to_string()),
        profile_dir: env::var("ARDA_PLAYWRIGHT_PROFILE_DIR")
            .unwrap_or_else(|_| format!("{state_root}/profile")),
        artifact_dir: env::var("ARDA_PLAYWRIGHT_ARTIFACT_DIR")
            .unwrap_or_else(|_| format!("{state_root}/artifacts")),
        log_path: env::var("ARDA_PLAYWRIGHT_LOG_PATH")
            .unwrap_or_else(|_| format!("{state_root}/bridge.log")),
        pid_path: env::var("ARDA_PLAYWRIGHT_PID_PATH")
            .unwrap_or_else(|_| format!("{state_root}/bridge.pid")),
        isolation: json!({
            "browser_profile_isolated": true,
            "artifacts_confined": true,
            "recommended_network_boundary": "tailscale_or_loopback_only",
            "selinux_note": "run on local host with confined state dirs; expose only through MCP governance surface"
        }),
        approvals: json!({
            "navigation_requires_approval": true,
            "network_allowed_flag_required": true,
            "triad_metadata_required": true,
            "destructive_allowed": false
        }),
    }
}

pub fn playwright_tool() -> McpTool {
    McpTool::new(
        "browser.playwright.session",
        "Governed Playwright MCP browser session bridge for audited navigation and page interaction.",
    )
    .with_schema(json!({
        "type": "object",
        "required": ["action", "target"],
        "properties": {
            "action": {
                "type": "string",
                "enum": ["navigate", "snapshot", "extract", "click", "type"]
            },
            "target": { "type": "string" },
            "session_id": { "type": "string" },
            "selector": { "type": "string" },
            "text": { "type": "string" }
        }
    }))
    .with_governance(McpToolGovernance {
        schema_version: "arda.mcp.tool.v1".to_string(),
        sigil: "∇◈".to_string(),
        requires_approval: true,
        allows_network: true,
        allows_destructive: false,
        triad_required: true,
    })
}
