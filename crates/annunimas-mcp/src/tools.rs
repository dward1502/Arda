// sigil: REPAIR
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub governance: McpToolGovernance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolGovernance {
    pub schema_version: String,
    pub sigil: String,
    pub requires_approval: bool,
    pub allows_network: bool,
    pub allows_destructive: bool,
    pub triad_required: bool,
}

impl McpTool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({}),
            governance: McpToolGovernance::default(),
        }
    }

    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn with_governance(mut self, governance: McpToolGovernance) -> Self {
        self.governance = governance;
        self
    }
}

impl Default for McpToolGovernance {
    fn default() -> Self {
        Self {
            schema_version: "annunimas.mcp.tool.v1".to_string(),
            sigil: "∇◈".to_string(),
            requires_approval: false,
            allows_network: false,
            allows_destructive: false,
            triad_required: true,
        }
    }
}

pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>;

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, (McpTool, ToolHandler)>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, tool: McpTool, handler: ToolHandler) {
        let mut tools = self.tools.write().await;
        tools.insert(tool.name.clone(), (tool, handler));
    }

    pub async fn get(&self, name: &str) -> Option<McpTool> {
        let tools = self.tools.read().await;
        tools.get(name).map(|(t, _)| t.clone())
    }

    pub async fn list(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().map(|(t, _)| t.clone()).collect()
    }

    pub async fn call(&self, name: &str, input: serde_json::Value) -> Option<serde_json::Value> {
        let tools = self.tools.read().await;
        tools.get(name).map(|(_, h)| h(input))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: msg.into(),
            data: None,
        }
    }

    pub fn method_not_found(msg: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
            data: None,
        }
    }
}

impl McpResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, error: McpError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum McpMethod {
    Initialize,
    ToolsList,
    ToolsCall,
    ResourcesList,
    ResourcesRead,
    ResourcesSubscribe,
    PromptList,
    PromptGet,
}

impl FromStr for McpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "initialize" => Ok(Self::Initialize),
            "tools/list" => Ok(Self::ToolsList),
            "tools/call" => Ok(Self::ToolsCall),
            "resources/list" => Ok(Self::ResourcesList),
            "resources/read" => Ok(Self::ResourcesRead),
            "resources/subscribe" => Ok(Self::ResourcesSubscribe),
            "prompts/list" => Ok(Self::PromptList),
            "prompts/get" => Ok(Self::PromptGet),
            _ => Err(()),
        }
    }
}
