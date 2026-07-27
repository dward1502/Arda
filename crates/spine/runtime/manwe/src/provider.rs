//! Minimal provider catalog for the gateway bootstrap.
//!
//! This is intentionally small: keep `ProviderCatalog` to one upstream per
//! placeholder slot so it reassembles the real config later.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

pub type ProviderId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub id: ProviderId,
    pub name: String,
    pub node_id: Option<String>,
    pub model_id: String,
    pub base_url: String,
    pub api_key_env: Option<String>,
    pub transport: ProviderTransport,
    pub capabilities: ProviderCapabilities,
    pub health_url: Option<String>,
    pub models_url: Option<String>,
    pub role: Option<String>,
    pub resource_group: Option<String>,
    pub resource_group_concurrency: Option<usize>,
    pub context_window: Option<usize>,
    #[serde(default)]
    pub access_tier: String,
    #[serde(default)]
    pub in_cooldown: bool,
    #[serde(default)]
    pub healthy: bool,
    pub model_observed: Option<bool>,
    pub probe_latency_ms: Option<u64>,
    pub last_probe_utc: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTransport {
    OpenAICompatible,
    AnthropicMessages,
    LocalHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_calls: bool,
    pub structured_output: bool,
}

impl ProviderDefinition {
    #[cfg(test)]
    pub fn openai_compatible(
        id: impl Into<String>,
        name: impl Into<String>,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            node_id: None,
            model_id: model_id.into(),
            base_url: base_url.into(),
            api_key_env: Some("ARDA_MANWE_PLACEHOLDER_API_KEY".into()),
            transport: ProviderTransport::OpenAICompatible,
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: true,
            },
            health_url: None,
            models_url: None,
            role: None,
            resource_group: None,
            resource_group_concurrency: None,
            context_window: None,
            access_tier: "local".to_string(),
            in_cooldown: false,
            healthy: true,
            model_observed: None,
            probe_latency_ms: None,
            last_probe_utc: None,
            last_error: None,
        }
    }

    fn from_fleet_node(node_id: impl Into<String>, node: &FleetNode) -> Self {
        let id = node.manwe_provider_id.clone();
        Self {
            id: id.clone(),
            name: node.display_name.clone().unwrap_or_else(|| id.clone()),
            node_id: Some(node_id.into()),
            model_id: node
                .runtime_model_alias
                .clone()
                .or_else(|| node.expected_models.first().cloned())
                .unwrap_or_default(),
            base_url: normalize_runtime_url(
                node.base_url.as_deref().unwrap_or_default(),
                node.runtime_port,
            ),
            api_key_env: None,
            transport: ProviderTransport::OpenAICompatible,
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calls: node.supports_tools.unwrap_or(false),
                structured_output: node.supports_structured_output.unwrap_or(false),
            },
            health_url: node.health_url.clone(),
            models_url: node.models_url.clone(),
            role: node.role.clone(),
            resource_group: node.hostname.clone(),
            resource_group_concurrency: node.resource_group_concurrency.filter(|limit| *limit > 0),
            context_window: node.runtime_context_window,
            access_tier: "local".to_string(),
            in_cooldown: false,
            healthy: false,
            model_observed: None,
            probe_latency_ms: None,
            last_probe_utc: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetNode {
    #[serde(default)]
    manwe_provider_id: String,
    display_name: Option<String>,
    role: Option<String>,
    hostname: Option<String>,
    resource_group_concurrency: Option<usize>,
    enrollment_status: Option<String>,
    base_url: Option<String>,
    runtime_port: Option<u16>,

    models_url: Option<String>,
    health_url: Option<String>,
    #[serde(default)]
    expected_models: Vec<String>,
    runtime_model_alias: Option<String>,
    runtime_context_window: Option<usize>,
    supports_tools: Option<bool>,
    supports_structured_output: Option<bool>,
    llm_runtime: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderRejectionDiagnostic {
    pub provider_id: String,
    pub model_id: String,
    pub reasons: Vec<&'static str>,
}

fn normalize_runtime_url(base_url: &str, runtime_port: Option<u16>) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(port) = runtime_port {
        let candidate = trimmed
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        if candidate.contains(':') {
            return format!("http://{candidate}");
        }
        return format!("http://127.0.0.1:{port}");
    }
    trimmed.to_string()
}

#[derive(Debug, Clone, Default)]
pub struct ProviderCatalog {
    by_id: HashMap<String, ProviderDefinition>,
}

impl ProviderCatalog {
    #[cfg(test)]
    pub fn new(injected: Vec<ProviderDefinition>) -> Self {
        let mut by_id = HashMap::new();
        for entry in injected {
            by_id.insert(entry.id.clone(), entry);
        }
        Self { by_id }
    }

    pub fn empty() -> Self {
        Self {
            by_id: HashMap::new(),
        }
    }

    pub fn insert(&mut self, provider: ProviderDefinition) {
        self.by_id.insert(provider.id.clone(), provider);
    }

    #[cfg(test)]
    pub fn get(&self, id: &str) -> Option<&ProviderDefinition> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ProviderDefinition)> {
        self.by_id.iter()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn healthy_count(&self) -> usize {
        self.by_id
            .values()
            .filter(|provider| provider.healthy)
            .count()
    }

    #[cfg(test)]
    pub fn resolve(
        &self,
        requested_model: &str,
        adaptive: bool,
        task_type: &str,
        required_context: usize,
    ) -> Option<&ProviderDefinition> {
        self.resolve_with_policy(
            requested_model,
            adaptive,
            task_type,
            required_context,
            false,
        )
    }

    pub fn resolve_with_policy(
        &self,
        requested_model: &str,
        adaptive: bool,
        task_type: &str,
        required_context: usize,
        local_only: bool,
    ) -> Option<&ProviderDefinition> {
        if let Some((provider_id, _)) = requested_model.split_once('/') {
            if provider_id != "local" {
                if let Some(provider) = self.by_id.get(provider_id) {
                    return provider_eligible(provider, task_type, required_context, local_only)
                        .then_some(provider);
                }
                return None;
            }
        }

        if let Some(provider) = self.by_id.values().find(|provider| {
            provider.model_id == requested_model
                && provider_eligible(provider, task_type, required_context, local_only)
        }) {
            return Some(provider);
        }

        if !adaptive
            && requested_model != "auto"
            && requested_model != "default"
            && requested_model != "local/auto"
        {
            return None;
        }

        self.by_id
            .values()
            .filter(|provider| provider_eligible(provider, task_type, required_context, local_only))
            .max_by_key(|provider| provider_score(provider, task_type))
    }

    /// Select the best equivalent provider outside the selected provider's
    /// resource group. This is used only after the selected group is observed
    /// at capacity; callers retain the original selection when no equivalent
    /// alternate exists so normal bounded queueing remains the fallback.
    pub fn resolve_alternate_resource_group(
        &self,
        selected: &ProviderDefinition,
        requested_model: &str,
        task_type: &str,
        required_context: usize,
        local_only: bool,
    ) -> Option<&ProviderDefinition> {
        let selected_group = selected
            .resource_group
            .as_deref()
            .unwrap_or(selected.id.as_str());
        let generic_request = matches!(requested_model, "auto" | "default" | "local/auto");

        self.by_id
            .values()
            .filter(|provider| provider.id != selected.id)
            .filter(|provider| {
                provider
                    .resource_group
                    .as_deref()
                    .unwrap_or(provider.id.as_str())
                    != selected_group
            })
            .filter(|provider| generic_request || provider.model_id == selected.model_id)
            .filter(|provider| provider_eligible(provider, task_type, required_context, local_only))
            .max_by_key(|provider| provider_score(provider, task_type))
    }

    pub fn rejection_diagnostics(
        &self,
        task_type: &str,
        required_context: usize,
        local_only: bool,
    ) -> Vec<ProviderRejectionDiagnostic> {
        let mut diagnostics = self
            .by_id
            .values()
            .filter_map(|provider| {
                let reasons =
                    provider_rejection_reasons(provider, task_type, required_context, local_only);
                (!reasons.is_empty()).then(|| ProviderRejectionDiagnostic {
                    provider_id: provider.id.clone(),
                    model_id: provider.model_id.clone(),
                    reasons,
                })
            })
            .collect::<Vec<_>>();
        diagnostics.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
        diagnostics
    }

    pub async fn probe_all(&mut self, client: &reqwest::Client) {
        let mut probes = tokio::task::JoinSet::new();
        for provider in self.by_id.values() {
            let id = provider.id.clone();
            let model_id = provider.model_id.clone();
            let url = provider
                .models_url
                .clone()
                .or_else(|| provider.health_url.clone());
            let client = client.clone();
            probes.spawn(async move {
                let Some(url) = url.filter(|url| !url.trim().is_empty()) else {
                    return (
                        id,
                        false,
                        None,
                        None,
                        Some("no probe URL configured".to_string()),
                    );
                };
                let started = Instant::now();
                let result = client.get(url).timeout(Duration::from_secs(4)).send().await;
                let latency_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                match result {
                    Ok(response) if response.status().is_success() => {
                        let body = response.text().await.unwrap_or_default();
                        let observed = model_id.is_empty() || body.contains(&model_id);
                        let error = (!observed).then(|| {
                            format!("expected model {model_id} missing from live catalog")
                        });
                        (id, observed, Some(observed), Some(latency_ms), error)
                    }
                    Ok(response) => (
                        id,
                        false,
                        None,
                        Some(latency_ms),
                        Some(format!("probe HTTP {}", response.status())),
                    ),
                    Err(error) => (id, false, None, Some(latency_ms), Some(error.to_string())),
                }
            });
        }

        while let Some(result) = probes.join_next().await {
            let Ok((id, healthy, model_observed, latency_ms, last_error)) = result else {
                continue;
            };
            if let Some(provider) = self.by_id.get_mut(&id) {
                provider.healthy = healthy;
                provider.model_observed = model_observed;
                provider.probe_latency_ms = latency_ms;
                provider.last_probe_utc = Some(chrono::Utc::now().to_rfc3339());
                provider.last_error = last_error;
            }
        }
    }

    #[cfg(test)]
    pub fn local_placeholder(&self) -> Option<&ProviderDefinition> {
        self.get("local_placeholder")
    }
    pub fn from_fleet_config(path: impl AsRef<Path>) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::empty();
        };
        Self::from_fleet_config_direct(text)
    }

    pub fn from_fleet_config_direct(text: impl AsRef<str>) -> Self {
        let nodes = parse_fleet_nodes(text.as_ref());
        let mut catalog = Self::empty();
        for (index, node) in nodes.into_iter().enumerate() {
            let status = node
                .enrollment_status
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if status != "active" && status != "active_staging" {
                continue;
            }
            let runtime_status = node.llm_runtime.as_deref().unwrap_or_default();
            if runtime_status.contains("inactive") || node.manwe_provider_id.is_empty() {
                continue;
            }
            let key = format!("fleet_{index}");
            catalog.insert(ProviderDefinition::from_fleet_node(key, &node));
        }
        catalog
    }
}

fn provider_eligible(
    provider: &ProviderDefinition,
    task_type: &str,
    required_context: usize,
    local_only: bool,
) -> bool {
    provider_rejection_reasons(provider, task_type, required_context, local_only).is_empty()
}

fn provider_rejection_reasons(
    provider: &ProviderDefinition,
    task_type: &str,
    required_context: usize,
    local_only: bool,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if local_only && provider.access_tier != "local" {
        reasons.push("local_only_policy");
    }
    if provider.in_cooldown {
        reasons.push("provider_cooldown");
    }
    if !provider.healthy {
        reasons.push("provider_unhealthy");
    }
    let task = task_type.to_ascii_lowercase();
    let context_floor = if task.contains("code") {
        required_context.max(64_000)
    } else {
        required_context
    };
    if provider.context_window.unwrap_or_default() < context_floor {
        reasons.push("context_window_too_small");
    }
    if task.contains("code") && !provider.capabilities.tool_calls {
        reasons.push("tool_calls_unsupported");
    }
    let role = provider
        .role
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if task.contains("vision") && !role.contains("vision") {
        reasons.push("vision_unsupported");
    }
    reasons
}

fn provider_score(provider: &ProviderDefinition, task_type: &str) -> i64 {
    let role = provider
        .role
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let model = provider.model_id.to_ascii_lowercase();
    let task = task_type.to_ascii_lowercase();
    let mut score = 100_i64;
    if task.contains("code") && (role.contains("coder") || model.contains("coder")) {
        score += 100;
    }
    if (task.contains("reason") || task.contains("research"))
        && (role.contains("bonsai27") || model.contains("bonsai-27"))
    {
        score += 90;
    }
    if task.contains("vision") && role.contains("vision") {
        score += 120;
    }
    if role.contains("guardhouse") {
        score -= 30;
    }
    score += provider.context_window.unwrap_or_default().min(262_144) as i64 / 4096;
    score -= provider.probe_latency_ms.unwrap_or(4_000).min(4_000) as i64 / 100;
    score
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetConfig {
    nodes: Vec<FleetNode>,
}

fn parse_fleet_nodes(text: &str) -> Vec<FleetNode> {
    toml::from_str::<FleetConfig>(text)
        .map(|fleet| fleet.nodes)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_filters_inactive_and_unenrolled_nodes() {
        let sample = r#"
[[nodes]]
manwe_provider_id = "edge_core"
display_name = "Core Hub"
enrollment_status = "active"
base_url = "http://core:9337/v1"
runtime_port = 9337
expected_models = ["LFM"]

[[nodes]]
manwe_provider_id = "edge_guardhouse"
enrollment_status = "inactive"
base_url = "http://warden:1234/v1"

[[nodes]]
manwe_provider_id = "edge_laptop"
llm_runtime = "local_voice_stt_operator"
base_url = "http://laptop:1234/v1"
"#;
        let catalog = ProviderCatalog::from_fleet_config_direct(sample);
        let ids: Vec<_> = catalog.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["edge_core"]);
    }

    #[test]
    fn builds_catalog_from_fleet_text() {
        let sample = r#"
[[nodes]]
manwe_provider_id = "edge_core"
enrollment_status = "active"
base_url = "http://core:9337/v1"
runtime_port = 9337
resource_group_concurrency = 2
expected_models = ["LFM"]
"#;
        let catalog = ProviderCatalog::from_fleet_config_direct(sample);
        assert_eq!(catalog.len(), 1);
        let provider = catalog.get("edge_core").expect("edge_core catalog");
        assert_eq!(provider.model_id, "LFM");
        assert_eq!(provider.base_url, "http://core:9337/v1");
        assert_eq!(provider.health_url, None);
        assert_eq!(provider.resource_group_concurrency, Some(2));
    }

    #[test]
    fn direct_fleet_catalog_is_empty_when_config_is_empty() {
        let catalog = ProviderCatalog::from_fleet_config_direct("");
        assert!(catalog.is_empty());
        assert!(catalog.local_placeholder().is_none());
    }

    #[test]
    fn adaptive_resolution_prefers_task_fit_and_enforces_context() {
        let sample = r#"
[[nodes]]
manwe_provider_id = "general"
role = "backbone_fast_general"
hostname = "shared-host"
enrollment_status = "active"
base_url = "http://general:8000/v1"
models_url = "http://general:8000/v1/models"
expected_models = ["general-model"]
runtime_context_window = 32768

[[nodes]]
manwe_provider_id = "coder"
role = "backbone_coder"
hostname = "shared-host"
enrollment_status = "active"
base_url = "http://coder:8001/v1"
models_url = "http://coder:8001/v1/models"
expected_models = ["coder-model"]
runtime_context_window = 65536
supports_tools = true
"#;
        let mut catalog = ProviderCatalog::from_fleet_config_direct(sample);
        for provider in catalog.by_id.values_mut() {
            provider.healthy = true;
            provider.probe_latency_ms = Some(20);
        }

        let selected = catalog
            .resolve("auto", true, "code", 32_000)
            .expect("code provider");
        assert_eq!(selected.id, "coder");
        assert_eq!(selected.resource_group.as_deref(), Some("shared-host"));
        assert!(catalog.resolve("general", false, "chat", 64_000).is_none());
    }

    #[test]
    fn adaptive_resolution_can_move_to_an_equivalent_alternate_resource_group() {
        let mut primary = ProviderDefinition::openai_compatible(
            "primary",
            "Primary",
            "shared-model",
            "http://primary:8000/v1",
        );
        primary.resource_group = Some("gpu-a".to_string());
        primary.context_window = Some(32_768);
        primary.probe_latency_ms = Some(10);
        let mut same_group = ProviderDefinition::openai_compatible(
            "same-group",
            "Same group",
            "shared-model",
            "http://same-group:8001/v1",
        );
        same_group.resource_group = Some("gpu-a".to_string());
        same_group.context_window = Some(32_768);
        same_group.probe_latency_ms = Some(1);
        let mut alternate = ProviderDefinition::openai_compatible(
            "alternate",
            "Alternate",
            "shared-model",
            "http://alternate:8002/v1",
        );
        alternate.resource_group = Some("gpu-b".to_string());
        alternate.context_window = Some(32_768);
        alternate.probe_latency_ms = Some(20);
        let catalog = ProviderCatalog::new(vec![primary.clone(), same_group, alternate]);

        let selected = catalog
            .resolve_alternate_resource_group(&primary, "auto", "chat", 4_096, false)
            .expect("alternate resource group");
        assert_eq!(selected.id, "alternate");
    }

    #[test]
    fn vision_requests_do_not_fall_back_to_text_only_lanes() {
        let mut text = ProviderDefinition::openai_compatible(
            "general",
            "General",
            "general-model",
            "http://general:8000/v1",
        );
        text.role = Some("backbone_fast_general".to_string());
        text.context_window = Some(32_768);
        let catalog = ProviderCatalog::new(vec![text]);

        assert!(catalog.resolve("auto", true, "vision", 4_096).is_none());
    }

    #[test]
    fn local_only_route_does_not_escape_to_cloud_during_local_failure() {
        let mut cloud = ProviderDefinition::openai_compatible(
            "cloud",
            "Cloud",
            "cloud-model",
            "https://cloud.example/v1",
        );
        cloud.access_tier = "cloud".to_string();
        let mut local = ProviderDefinition::openai_compatible(
            "edge_core",
            "Core",
            "local-model",
            "http://127.0.0.1:9337/v1",
        );
        local.access_tier = "local".to_string();
        local.healthy = false;
        let catalog = ProviderCatalog::new(vec![cloud, local]);

        assert!(catalog
            .resolve_with_policy("local/auto", true, "code", 64_000, true)
            .is_none());
    }

    #[test]
    fn cooled_cloud_does_not_block_eligible_local_tool_route() {
        let mut cloud = ProviderDefinition::openai_compatible(
            "cloud",
            "Cloud",
            "cloud-model",
            "https://cloud.example/v1",
        );
        cloud.access_tier = "cloud".to_string();
        cloud.in_cooldown = true;
        let mut local = ProviderDefinition::openai_compatible(
            "edge_core",
            "Core",
            "local-model",
            "http://127.0.0.1:9337/v1",
        );
        local.access_tier = "local".to_string();
        local.context_window = Some(131_072);
        local.capabilities.tool_calls = true;
        let catalog = ProviderCatalog::new(vec![cloud, local]);

        let selected = catalog
            .resolve_with_policy("local/auto", true, "code", 80_000, true)
            .expect("eligible local provider");
        assert_eq!(selected.id, "edge_core");
    }

    #[test]
    fn rejection_diagnostics_are_stable_and_actionable() {
        let mut local = ProviderDefinition::openai_compatible(
            "edge_light",
            "Light",
            "small-model",
            "http://127.0.0.1:9337/v1",
        );
        local.access_tier = "local".to_string();
        local.context_window = Some(32_768);
        local.capabilities.tool_calls = false;
        let catalog = ProviderCatalog::new(vec![local]);

        let diagnostics = catalog.rejection_diagnostics("code", 64_000, true);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].provider_id, "edge_light");
        assert_eq!(
            diagnostics[0].reasons,
            vec!["context_window_too_small", "tool_calls_unsupported"]
        );
    }
}
