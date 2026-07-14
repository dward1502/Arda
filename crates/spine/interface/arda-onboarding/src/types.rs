use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueSource {
    Environment,
    EnvFile,
    Default,
    ServiceRegistry,
    OperatorInput,
    Detected,
    Unknown,
}

impl Default for ValueSource {
    fn default() -> Self {
        ValueSource::Unknown
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PathValue {
    pub value: String,
    pub source: ValueSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portable_expression: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UrlValue {
    pub value: String,
    pub source: ValueSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelDefaultValue {
    pub value: String,
    pub source: ValueSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charon_base_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hermes_base_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arda_hud_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_model_base_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_model_default: Option<LocalModelDefaultValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub litellm_proxy_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crawl4ai_url: Option<UrlValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_runtime_url: Option<UrlValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathsSection {
    pub arda_root: PathValue,
    pub home: PathValue,
    pub config_dir: PathValue,
    pub data_dir: PathValue,
    pub cache_dir: PathValue,
    pub runtime_dir: PathValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_cache_root: Option<PathValue>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_homes: BTreeMap<String, PathValue>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub sockets: BTreeMap<String, PathValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemdSection {
    pub environment_file_pattern: String,
    pub user_units_available: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafetySection {
    pub autonomy_posture: String,
    pub mutation_requires_human_gate: bool,
    pub destructive_allowed_by_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptEntry {
    pub path: String,
    pub contract: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderSignupHint {
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) signup_url: Option<String>,
    pub(crate) steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderCheckProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) route_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderActionHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) requires_key: bool,
    pub(crate) no_key_fallback_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderReadinessHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) route_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) local_default_model: Option<String>,
    pub(crate) has_local_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAction {
    pub action_id: String,
    pub action_type: String,
    pub title: String,
    pub command_hint: String,
    pub target_path: Option<String>,
    pub requires_human_gate: bool,
    pub description: String,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePlan {
    pub contract: String,
    pub generated_at_utc: String,
    pub profile: String,
    pub machine_role: String,
    pub gate_status: String,
    pub approval_contract_required: String,
    pub actions: Vec<ServiceAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalReceipt {
    pub contract: String,
    pub approved: bool,
    pub approver: String,
    pub reason: String,
    pub approved_scope: Vec<String>,
    pub approved_at_utc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub contract: String,
    pub action: String,
    pub generated_at_utc: String,
    pub execute: bool,
    pub result: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorProfile {
    pub arda_user: Option<String>,
    pub source: Option<ValueSource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentProfile {
    pub contract: String,
    pub generated_at: String,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorProfile>,
    pub machine_role: String,
    pub paths: PathsSection,
    pub endpoints: EndpointSection,
    pub systemd: SystemdSection,
    pub safety: SafetySection,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_gates: Vec<String>,
    #[serde(default)]
    pub receipts: Vec<ReceiptEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReadinessCheck {
    pub(crate) check_id: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) recommendation: String,
    pub(crate) severity: String,
    pub(crate) status: String,
    pub(crate) title: String,
}

#[derive(Debug, Serialize)]
pub struct ReadinessProjection {
    pub(crate) checks: Vec<ReadinessCheck>,
    pub gate_status: String,
    pub(crate) generated_at_utc: String,
    pub(crate) mode: String,
    pub(crate) mutation_policy: String,
    pub(crate) portability_status: Value,
    pub(crate) runtime: Value,
    pub(crate) schema_version: u32,
    pub(crate) summary: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pass: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warn: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct ProviderInfo {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) enabled: bool,
    pub(crate) access_tier: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) route_hints: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider_profile: Option<ProviderCheckProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) signup_hint: Option<ProviderSignupHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) action_hint: Option<ProviderActionHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) readiness_hint: Option<ProviderReadinessHint>,
    pub(crate) has_default_model: bool,
    pub(crate) model_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) fallback_routes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) model_ids: Vec<String>,
    pub(crate) missing_env: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProviderChecklist {
    pub(crate) generated_at_utc: String,
    pub(crate) profile: String,
    pub(crate) providers_path: String,
    pub(crate) providers: Vec<ProviderInfo>,
    pub(crate) suggested_signatures: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct ParsedProviderEntry {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) enabled: bool,
    pub(crate) access_tier: Option<String>,
    pub(crate) base_url: Option<String>,
    pub(crate) missing_env: Vec<String>,
    pub(crate) has_default_model: bool,
    pub(crate) model_count: usize,
    pub(crate) model_ids: Vec<String>,
    pub(crate) default_model: Option<String>,
    pub(crate) env_key: Option<String>,
    pub(crate) route_hints: BTreeSet<String>,
    pub(crate) payment_class: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DeviceScan {
    pub(crate) generated_at_utc: String,
    pub(crate) host: String,
    pub(crate) platform: String,
    pub(crate) architecture: String,
    pub(crate) container_hint: bool,
    pub(crate) tailscale: Value,
    pub(crate) runtime: Value,
    pub(crate) capabilities: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrerequisiteCheck {
    pub check_id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub detected: String,
    pub recommendation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrerequisiteReport {
    pub contract: String,
    pub generated_at_utc: String,
    pub profile: String,
    pub machine_role: String,
    pub checks: Vec<PrerequisiteCheck>,
    pub summary: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivateConfigEntry {
    pub key: String,
    pub value_preview: String,
    pub source: ValueSource,
    pub required: bool,
    pub secret: bool,
    pub present: bool,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivateConfigStage {
    pub contract: String,
    pub generated_at_utc: String,
    pub target_path: String,
    pub write_policy: String,
    pub entries: Vec<PrivateConfigEntry>,
    pub missing_required: Vec<String>,
    pub proposed_env_path: String,
    pub receipt_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorAnswers {
    pub contract: String,
    pub generated_at_utc: String,
    pub machine_role: String,
    pub profile: String,
    pub autonomy_posture: String,
    pub mutation_requires_human_gate: bool,
    pub enable_hermes_discord: bool,
    pub enable_fleet_discovery: bool,
    pub prefer_local_assistant: bool,
    pub selected_providers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedStep {
    pub step_id: String,
    pub title: String,
    pub status: String,
    pub prompt: String,
    pub evidence: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedSession {
    pub contract: String,
    pub generated_at_utc: String,
    pub profile: String,
    pub machine_role: String,
    pub answers_contract: String,
    pub answers: OperatorAnswers,
    pub steps: Vec<GuidedStep>,
    pub next_actions: Vec<String>,
}
