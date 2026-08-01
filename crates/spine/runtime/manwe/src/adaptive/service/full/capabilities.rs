use super::{paths, CharonService};
use crate::adaptive::types::ProviderState;
use arda_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProviderCapabilityReceiptsFile {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub generated_at_utc: String,
    #[serde(default)]
    pub receipts: BTreeMap<String, ProviderModelCapabilityReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProviderModelCapabilityReceipt {
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub updated_at_utc: String,
    #[serde(default)]
    pub capabilities: BTreeMap<String, CapabilityReceiptEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct CapabilityReceiptEntry {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub observed_at_utc: String,
    #[serde(default)]
    pub expires_at_utc: String,
    #[serde(default)]
    pub outcome_class: String,
    #[serde(default)]
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCapabilitySummary {
    pub receipt_model_count: usize,
    pub models_with_failed_tool_receipts: usize,
    pub models_with_failed_structured_output_receipts: usize,
    pub models_with_failed_streaming_receipts: usize,
    pub recent_capability_failures: usize,
    pub providers_with_no_capability_evidence: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilityView {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub summary: ProviderCapabilitySummary,
    pub providers: Vec<ProviderCapabilityProviderView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilityProviderView {
    pub provider_id: String,
    pub enabled: bool,
    pub access_tier: String,
    pub evidence_state: String,
    pub models: Vec<ProviderCapabilityModelView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilityModelView {
    pub model_id: String,
    pub is_default: bool,
    pub healthy: bool,
    pub capabilities: BTreeMap<String, CapabilityReceiptView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityReceiptView {
    pub state: String,
    pub observed_at_utc: Option<String>,
    pub expires_at_utc: Option<String>,
    pub outcome_class: Option<String>,
    pub status_code: Option<u16>,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProviderCandidatesFile {
    #[serde(default)]
    schema_version: String,
    #[serde(default)]
    provider_candidate: Vec<ProviderCandidate>,
    #[serde(default)]
    probe_budget: ProbeBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProbeBudget {
    #[serde(default)]
    active_capability_probes_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProviderCandidate {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    free_kind: String,
    #[serde(default)]
    requires_adapter: bool,
    #[serde(default)]
    access_tier_candidate: String,
    #[serde(default)]
    priority: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPromotionGuardView {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub active_capability_probes_enabled: bool,
    pub candidates: Vec<ProviderPromotionCandidateView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPromotionCandidateView {
    pub id: String,
    pub name: String,
    pub status: String,
    pub free_kind: String,
    pub access_tier_candidate: String,
    pub requires_adapter: bool,
    pub promotion_ready: bool,
    pub reasons: Vec<String>,
}

impl CharonService {
    pub(crate) fn capability_receipts_file(&self) -> ProviderCapabilityReceiptsFile {
        fs::read_to_string(self.provider_capability_receipts_path())
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub(crate) async fn provider_capability_view(&self) -> Result<ProviderCapabilityView> {
        let providers = self.providers_read().await;
        let receipts = self.capability_receipts_file();
        Ok(build_provider_capability_view(&providers, receipts))
    }

    pub(crate) fn provider_capability_summary(
        &self,
        providers: &[ProviderState],
    ) -> ProviderCapabilitySummary {
        build_provider_capability_summary(providers, &self.capability_receipts_file())
    }

    pub(crate) fn provider_promotion_guard_view(&self) -> ProviderPromotionGuardView {
        let candidates = read_provider_candidates();
        let receipts = self.capability_receipts_file();
        build_provider_promotion_guard_view(candidates, receipts)
    }
}

fn build_provider_capability_view(
    providers: &[ProviderState],
    receipts: ProviderCapabilityReceiptsFile,
) -> ProviderCapabilityView {
    let summary = build_provider_capability_summary(providers, &receipts);
    let now = Utc::now();
    let mut provider_views = Vec::new();
    for provider in providers {
        let mut models = Vec::new();
        let mut provider_has_evidence = false;
        for model in &provider.models {
            let key = receipt_key(&provider.id, &model.id);
            let receipt = receipts.receipts.get(&key);
            if receipt.is_some() {
                provider_has_evidence = true;
            }
            let mut caps = BTreeMap::new();
            for capability in ["basic_chat", "tools", "structured_output", "streaming"] {
                caps.insert(
                    capability.to_string(),
                    receipt
                        .and_then(|receipt| receipt.capabilities.get(capability))
                        .map(|entry| receipt_view(entry, now))
                        .unwrap_or_else(|| CapabilityReceiptView {
                            state: "unknown".to_string(),
                            observed_at_utc: None,
                            expires_at_utc: None,
                            outcome_class: None,
                            status_code: None,
                            expired: false,
                        }),
                );
            }
            models.push(ProviderCapabilityModelView {
                model_id: model.id.clone(),
                is_default: model.is_default,
                healthy: model.healthy && !model.in_cooldown,
                capabilities: caps,
            });
        }
        provider_views.push(ProviderCapabilityProviderView {
            provider_id: provider.id.clone(),
            enabled: provider.enabled,
            access_tier: provider.access_tier.clone(),
            evidence_state: if provider_has_evidence {
                "observed".to_string()
            } else {
                "unknown".to_string()
            },
            models,
        });
    }
    ProviderCapabilityView {
        schema_version: "annunimas.charon.provider-capability-view.v1".to_string(),
        generated_at_utc: now.to_rfc3339(),
        summary,
        providers: provider_views,
    }
}

fn build_provider_capability_summary(
    providers: &[ProviderState],
    receipts: &ProviderCapabilityReceiptsFile,
) -> ProviderCapabilitySummary {
    let now = Utc::now();
    let mut failed_tools = BTreeSet::new();
    let mut failed_structured = BTreeSet::new();
    let mut failed_streaming = BTreeSet::new();
    let mut recent_failures = 0;
    for (key, receipt) in &receipts.receipts {
        for (capability, entry) in &receipt.capabilities {
            if entry.state != "failed" || receipt_expired(entry, now) {
                continue;
            }
            if recent_failure(entry, now) {
                recent_failures += 1;
            }
            match capability.as_str() {
                "tools" => {
                    failed_tools.insert(key.clone());
                }
                "structured_output" => {
                    failed_structured.insert(key.clone());
                }
                "streaming" => {
                    failed_streaming.insert(key.clone());
                }
                _ => {}
            }
        }
    }
    let mut providers_with_evidence = BTreeSet::new();
    for receipt in receipts.receipts.values() {
        if !receipt.capabilities.is_empty() {
            providers_with_evidence.insert(receipt.provider_id.clone());
        }
    }
    let providers_with_no_capability_evidence = providers
        .iter()
        .filter(|provider| provider.enabled)
        .filter(|provider| !providers_with_evidence.contains(&provider.id))
        .count();
    ProviderCapabilitySummary {
        receipt_model_count: receipts.receipts.len(),
        models_with_failed_tool_receipts: failed_tools.len(),
        models_with_failed_structured_output_receipts: failed_structured.len(),
        models_with_failed_streaming_receipts: failed_streaming.len(),
        recent_capability_failures: recent_failures,
        providers_with_no_capability_evidence,
    }
}

fn read_provider_candidates() -> ProviderCandidatesFile {
    let path = std::env::var("ARDA_MANWE_PROVIDER_CANDIDATES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| paths::arda_root().join("config/charon.provider_candidates.toml"));
    fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn build_provider_promotion_guard_view(
    candidates: ProviderCandidatesFile,
    receipts: ProviderCapabilityReceiptsFile,
) -> ProviderPromotionGuardView {
    let mut views = Vec::new();
    for candidate in candidates.provider_candidate {
        let mut reasons = Vec::new();
        if candidate.status == "rejected" {
            reasons.push("candidate_rejected".to_string());
        }
        if candidate.requires_adapter {
            reasons.push("adapter_required".to_string());
        }
        let has_evidence = receipts
            .receipts
            .values()
            .any(|receipt| receipt.provider_id == candidate.id && !receipt.capabilities.is_empty());
        if !has_evidence && !candidates.probe_budget.active_capability_probes_enabled {
            reasons.push("needs_passive_receipt_or_operator_approved_active_probe".to_string());
        }
        if candidate.free_kind != "permanent" && candidate.access_tier_candidate == "free_cloud" {
            reasons.push("free_kind_not_permanent_for_free_cloud".to_string());
        }
        let promotion_ready = candidate.status == "candidate" && reasons.is_empty();
        views.push(ProviderPromotionCandidateView {
            id: candidate.id,
            name: candidate.name,
            status: candidate.status,
            free_kind: candidate.free_kind,
            access_tier_candidate: candidate.access_tier_candidate,
            requires_adapter: candidate.requires_adapter,
            promotion_ready,
            reasons,
        });
    }
    views.sort_by_key(|view| (!view.promotion_ready, view.id.clone()));
    ProviderPromotionGuardView {
        schema_version: candidates.schema_version,
        generated_at_utc: Utc::now().to_rfc3339(),
        active_capability_probes_enabled: candidates.probe_budget.active_capability_probes_enabled,
        candidates: views,
    }
}

fn receipt_view(
    entry: &CapabilityReceiptEntry,
    now: chrono::DateTime<Utc>,
) -> CapabilityReceiptView {
    CapabilityReceiptView {
        state: if receipt_expired(entry, now) {
            "expired".to_string()
        } else {
            entry.state.clone()
        },
        observed_at_utc: non_empty(entry.observed_at_utc.clone()),
        expires_at_utc: non_empty(entry.expires_at_utc.clone()),
        outcome_class: non_empty(entry.outcome_class.clone()),
        status_code: entry.status_code,
        expired: receipt_expired(entry, now),
    }
}

fn receipt_expired(entry: &CapabilityReceiptEntry, now: chrono::DateTime<Utc>) -> bool {
    chrono::DateTime::parse_from_rfc3339(&entry.expires_at_utc)
        .ok()
        .is_some_and(|expires| expires.with_timezone(&Utc) <= now)
}

fn recent_failure(entry: &CapabilityReceiptEntry, now: chrono::DateTime<Utc>) -> bool {
    chrono::DateTime::parse_from_rfc3339(&entry.observed_at_utc)
        .ok()
        .is_some_and(|observed| (now - observed.with_timezone(&Utc)).num_hours() <= 24)
}

fn receipt_key(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}::{model_id}")
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::types::{ModelCapabilities, ModelState};

    #[test]
    fn capability_summary_counts_failed_unexpired_receipts() {
        let providers = vec![ProviderState {
            id: "p1".to_string(),
            name: "p1".to_string(),
            base_url: None,
            api_key_env: None,
            access_tier: "mixed".to_string(),
            quality_band: "high".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 0,
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
            models: vec![ModelState {
                id: "m1".to_string(),
                aliases: vec![],
                capable_tasks: vec!["chat".to_string()],
                context_window: 8192,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: ModelCapabilities::default(),
                streaming_validated: None,
            }],
        }];
        let now = Utc::now();
        let mut receipts = ProviderCapabilityReceiptsFile::default();
        receipts.receipts.insert(
            "p1::m1".to_string(),
            ProviderModelCapabilityReceipt {
                provider_id: "p1".to_string(),
                model_id: "m1".to_string(),
                updated_at_utc: now.to_rfc3339(),
                capabilities: BTreeMap::from([(
                    "tools".to_string(),
                    CapabilityReceiptEntry {
                        state: "failed".to_string(),
                        source: "test".to_string(),
                        observed_at_utc: now.to_rfc3339(),
                        expires_at_utc: (now + chrono::Duration::hours(1)).to_rfc3339(),
                        outcome_class: "client_payload_error".to_string(),
                        status_code: Some(400),
                    },
                )]),
            },
        );
        let summary = build_provider_capability_summary(&providers, &receipts);
        assert_eq!(summary.models_with_failed_tool_receipts, 1);
        assert_eq!(summary.recent_capability_failures, 1);
        assert_eq!(summary.providers_with_no_capability_evidence, 0);
    }
}
