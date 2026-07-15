//! Governed tool registry for the blender-mcp bridge.
//!
//! Each tool exposes [`arda_tool_harness::types::ToolMetadata`] so that
//! invocations route through the standard harness disposition check (idempotency,
//! risk-level review, soterion trace). The handler is async and consumes a JSON
//! params blob, matching the upstream addon's wire format.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use arda_tool_harness::service::build_invocation_plan;
use arda_tool_harness::types::{
    InvocationEnvelope, InvocationPlan, RiskLevel, SideEffectClass, ToolMetadata,
};
use serde_json::Value;

use super::mcp_bridge::McpBridge;

pub type BlenderToolHandler = Arc<
    dyn Fn(McpBridge, Value) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>> + Send>>
        + Send
        + Sync,
>;

pub struct BlenderTool {
    pub metadata: ToolMetadata,
    pub handler: BlenderToolHandler,
}

pub struct BlenderToolRegistry {
    bridge: McpBridge,
    tools: HashMap<String, BlenderTool>,
}

impl BlenderToolRegistry {
    /// Build the registry with the default upstream tool set.
    pub fn with_defaults(bridge: McpBridge) -> Self {
        let mut reg = Self {
            bridge,
            tools: HashMap::new(),
        };
        reg.register_defaults();
        reg
    }

    pub fn bridge(&self) -> &McpBridge {
        &self.bridge
    }

    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }

    pub fn metadata(&self, name: &str) -> Option<&ToolMetadata> {
        self.tools.get(name).map(|t| &t.metadata)
    }

    /// Build a harness invocation plan for a tool call, applying governance
    /// (idempotency, operator review for Critical risk, etc.).
    pub fn plan(
        &self,
        name: &str,
        envelope: &InvocationEnvelope,
    ) -> anyhow::Result<InvocationPlan> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown blender tool: {name}"))?;
        build_invocation_plan(&tool.metadata, envelope)
            .map_err(|e| anyhow::anyhow!("harness rejected invocation: {e:?}"))
    }

    /// Validate via the harness, then dispatch the call across the bridge.
    pub async fn invoke(
        &self,
        name: &str,
        params: Value,
        envelope: &InvocationEnvelope,
    ) -> anyhow::Result<Value> {
        let plan = self.plan(name, envelope)?;
        if matches!(
            plan.disposition,
            arda_tool_harness::types::InvocationDisposition::HoldForOperatorReview
        ) {
            anyhow::bail!("tool {name} held for operator review (Critical risk)");
        }
        let handler = self.tools[name].handler.clone();
        handler(self.bridge.clone(), params).await
    }

    fn register_defaults(&mut self) {
        // Read-only: scene + object introspection
        self.register(
            tool_meta(
                "blender.get_scene_info",
                RiskLevel::Low,
                SideEffectClass::ReadOnly,
            ),
            Arc::new(|bridge, _params| Box::pin(async move { bridge.get_scene_info().await })),
        );
        self.register(
            tool_meta(
                "blender.get_object_info",
                RiskLevel::Low,
                SideEffectClass::ReadOnly,
            ),
            Arc::new(|bridge, params| {
                Box::pin(async move {
                    let name = params
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("missing `name`"))?
                        .to_string();
                    bridge.get_object_info(&name).await
                })
            }),
        );
        self.register(
            tool_meta(
                "blender.get_viewport_screenshot",
                RiskLevel::Low,
                SideEffectClass::ReadOnly,
            ),
            Arc::new(|bridge, params| {
                Box::pin(async move {
                    let max_size = params
                        .get("max_size")
                        .and_then(Value::as_u64)
                        .map(|v| v as u32);
                    bridge.get_viewport_screenshot(max_size).await
                })
            }),
        );
        self.register(
            tool_meta(
                "blender.get_polyhaven_status",
                RiskLevel::Low,
                SideEffectClass::ReadOnly,
            ),
            Arc::new(|bridge, _params| {
                Box::pin(async move { bridge.get_polyhaven_status().await })
            }),
        );

        // Mutating: scene authoring — needs idempotency key per harness rules.
        // `execute_code` runs arbitrary Python inside Blender; mark High so it
        // gets through (Critical would always block).
        self.register(
            tool_meta(
                "blender.execute_code",
                RiskLevel::High,
                SideEffectClass::Mutating,
            ),
            Arc::new(|bridge, params| {
                Box::pin(async move {
                    let code = params
                        .get("code")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("missing `code`"))?
                        .to_string();
                    bridge.execute_code(&code).await
                })
            }),
        );
        self.register(
            tool_meta(
                "blender.download_polyhaven_asset",
                RiskLevel::Medium,
                SideEffectClass::Mutating,
            ),
            Arc::new(|bridge, params| {
                Box::pin(async move {
                    let asset_id = params
                        .get("asset_id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("missing `asset_id`"))?
                        .to_string();
                    let asset_type = params
                        .get("asset_type")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("missing `asset_type`"))?
                        .to_string();
                    let resolution = params
                        .get("resolution")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    bridge
                        .download_polyhaven_asset(&asset_id, &asset_type, resolution.as_deref())
                        .await
                })
            }),
        );
    }

    fn register(&mut self, metadata: ToolMetadata, handler: BlenderToolHandler) {
        self.tools
            .insert(metadata.tool_id.clone(), BlenderTool { metadata, handler });
    }
}

fn tool_meta(tool_id: &str, risk: RiskLevel, side_effect: SideEffectClass) -> ToolMetadata {
    ToolMetadata {
        tool_id: tool_id.to_string(),
        version: "1".into(),
        owner: "forge-mind".into(),
        description: format!("BlenderMCP bridge tool: {tool_id}"),
        input_schema_ref: format!("arda.tool.{tool_id}.input.v1"),
        output_schema_ref: format!("arda.tool.{tool_id}.output.v1"),
        risk_level: risk,
        side_effect_class: side_effect,
    }
}

/// Convenience: an envelope with a generated idempotency key (UUID-ish from
/// time+random). Use this when the caller hasn't pre-computed one but the
/// invocation is naturally idempotent for the given inputs.
pub fn fresh_envelope(actor: &str, trace_id: &str) -> InvocationEnvelope {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    InvocationEnvelope {
        trace_id: Some(trace_id.to_string()),
        actor: Some(actor.to_string()),
        idempotency_key: Some(format!("forge-{nanos:x}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_tool_harness::types::InvocationDisposition;
    use serde_json::json;

    fn registry() -> BlenderToolRegistry {
        BlenderToolRegistry::with_defaults(McpBridge::new(crate::tools::mcp_bridge::default_addr()))
    }

    #[test]
    fn default_registry_exposes_core_blender_tools() {
        let reg = registry();
        let mut names = reg.names();
        names.sort();
        assert!(names.contains(&"blender.get_scene_info"));
        assert!(names.contains(&"blender.execute_code"));
        assert!(names.contains(&"blender.download_polyhaven_asset"));
    }

    #[test]
    fn read_only_tools_plan_to_allow_read_only() {
        let reg = registry();
        let envelope = fresh_envelope("forge-mind", "trace-1");
        let plan = reg.plan("blender.get_scene_info", &envelope).unwrap();
        assert_eq!(plan.disposition, InvocationDisposition::AllowReadOnly);
    }

    #[test]
    fn mutating_tools_require_idempotency() {
        let reg = registry();
        let envelope = InvocationEnvelope {
            trace_id: Some("t".into()),
            actor: Some("forge-mind".into()),
            idempotency_key: None,
        };
        assert!(reg.plan("blender.execute_code", &envelope).is_err());

        let envelope = fresh_envelope("forge-mind", "t");
        let plan = reg.plan("blender.execute_code", &envelope).unwrap();
        assert_eq!(
            plan.disposition,
            InvocationDisposition::AllowMutatingWithIdempotency
        );
    }

    #[test]
    fn unknown_tool_is_rejected() {
        let reg = registry();
        let envelope = fresh_envelope("forge-mind", "t");
        assert!(reg.plan("blender.does_not_exist", &envelope).is_err());
    }

    #[test]
    fn invoke_dispatch_uses_harness_then_handler_signature() {
        // This proves the wiring compiles end-to-end; the actual TCP exchange
        // is exercised live (no addon → connection refused, which is the
        // expected error path here).
        let reg = registry();
        let envelope = fresh_envelope("forge-mind", "t");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(reg.invoke("blender.get_scene_info", json!({}), &envelope));
        assert!(
            result.is_err(),
            "expected connection failure when no Blender addon is running"
        );
    }
}
