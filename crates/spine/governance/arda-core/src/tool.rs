// sigil: REPAIR
use crate::error::Result as CrateResult;
use crate::tool_contract::types::{RiskLevel, SideEffectClass, ToolMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an external tool/capability the system can use.
/// Loaded from registry.toml at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub repo: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub category: String,
    pub status: String,
    pub description: String,
    pub image: Option<String>,
    pub port: Option<u16>,
    pub install: Option<String>,
    pub sigil: Option<String>, // New: Soterion sigil for nav/search (e.g., "𓁿")
}

impl ToolEntry {
    pub fn harness_metadata(&self, tool_name: &str) -> ToolMetadata {
        ToolMetadata {
            tool_id: tool_name.to_string(),
            version: "v1".to_string(),
            owner: self.category.clone(),
            description: self.description.clone(),
            input_schema_ref: format!("Arda.tool.{tool_name}.input.v1"),
            output_schema_ref: format!("Arda.tool.{tool_name}.output.v1"),
            risk_level: inferred_risk_level(self),
            side_effect_class: inferred_side_effect_class(self),
        }
    }

    pub fn is_harness_ready(&self, tool_name: &str) -> bool {
        self.harness_metadata(tool_name).validate().is_ok()
    }
}

/// Registry of all known tools, loaded from registry.toml
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Load from a parsed TOML table
    pub fn from_toml(table: &toml::Table) -> CrateResult<Self> {
        let mut tools = HashMap::new();
        if let Some(tools_table) = table.get("tools").and_then(|v| v.as_table()) {
            for (name, value) in tools_table {
                let entry: ToolEntry = value.clone().try_into().map_err(|e: toml::de::Error| {
                    crate::error::ArdaError::Config(format!(
                        "Failed to parse tool '{}': {}",
                        name, e
                    ))
                })?;
                tools.insert(name.clone(), entry);
            }
        }
        Ok(Self { tools })
    }

    pub fn get(&self, name: &str) -> Option<&ToolEntry> {
        self.tools.get(name)
    }

    pub fn list_active(&self) -> Vec<(&String, &ToolEntry)> {
        self.tools
            .iter()
            .filter(|(_, entry)| entry.status == "active")
            .collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<(&String, &ToolEntry)> {
        self.tools
            .iter()
            .filter(|(_, entry)| entry.category == category)
            .collect()
    }
    pub fn get_by_sigil(&self, sigil: &str) -> Vec<(&String, &ToolEntry)> {
        self.tools
            .iter()
            .filter(|(_, entry)| entry.sigil.as_deref() == Some(sigil))
            .collect()
    }

    pub fn harness_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.get(name).map(|entry| entry.harness_metadata(name))
    }

    pub fn list_harness_ready(&self) -> Vec<(String, ToolMetadata)> {
        self.tools
            .iter()
            .filter_map(|(name, entry)| {
                let metadata = entry.harness_metadata(name);
                metadata.validate().ok().map(|_| (name.clone(), metadata))
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn inferred_risk_level(entry: &ToolEntry) -> RiskLevel {
    match entry.category.as_str() {
        "browser" | "communication" | "edge-runtime" => RiskLevel::High,
        "ingestion" | "llm-routing" | "model-selection" => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}

fn inferred_side_effect_class(entry: &ToolEntry) -> SideEffectClass {
    match entry.tool_type.as_str() {
        "plugin" | "library" => SideEffectClass::ReadOnly,
        _ => SideEffectClass::Mutating,
    }
}

#[cfg(test)]
mod tests {
    use super::ToolRegistry;

    #[test]
    fn harness_metadata_is_derived_for_registry_tool() {
        let table = toml::toml! {
            [tools.crawl4ai]
            repo = "unclecode/crawl4ai"
            type = "docker"
            image = "unclecode/crawl4ai:latest"
            port = 11235
            category = "ingestion"
            status = "active"
            description = "LLM-friendly web crawler"
        };

        let registry = ToolRegistry::from_toml(&table).expect("registry");
        let metadata = registry
            .harness_metadata("crawl4ai")
            .expect("crawl4ai metadata");
        assert_eq!(metadata.tool_id, "crawl4ai");
        assert_eq!(metadata.owner, "ingestion");
        assert!(registry
            .get("crawl4ai")
            .expect("entry")
            .is_harness_ready("crawl4ai"));
    }
}
