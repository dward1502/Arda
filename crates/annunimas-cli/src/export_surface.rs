use anyhow::Result;
use chrono::Utc;
use clap::Subcommand;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

mod aipkg_exports;
mod athena_exports;
mod business_exports;
mod embodiment_exports;
mod extension_exports;
mod fleet_exports;
mod hermes_exports;
mod operator_exports;
mod package_exports;
mod product_exports;
mod research_runtime_exports;
mod runtime_exports;

use aipkg_exports::{
    export_aipkg_contract_impl, export_aipkg_edge_lab_contract_impl,
    export_aipkg_marketplace_separation_contract_impl,
    export_edge_identity_remediation_contract_impl,
    export_network_native_node_onboarding_contract_impl,
};
use athena_exports::{
    export_agent_continuity_contract_impl, export_apollo_research_workflow_runtime_impl,
    export_async_user_intake_contract_impl, export_athena_digest_pipeline_impl,
    export_athena_integration_plan_impl, export_autonomy_resume_impl,
    export_community_signal_intake_impl, export_hermes_community_sources_impl,
    export_human_corpus_digest_plan_impl, export_human_corpus_extraction_registry_impl,
    export_human_corpus_registry_impl, export_intake_confidence_ladder_impl,
    export_multi_domain_routing_contract_impl, export_research_workflow_contract_impl,
    export_socratic_validator_contract_impl, export_source_absorption_executor_impl,
    export_source_absorption_pipeline_impl, export_source_absorption_portfolio_impl,
    export_source_ecosystem_registry_impl,
};
use business_exports::{
    export_client_delivery_portfolio_impl, export_client_delivery_readiness_impl,
    export_imported_corpus_plan_portfolio_impl, export_numenor_prime_merge_registry_impl,
    export_valinor_merge_registry_impl, export_valinor_project_dossiers_impl,
};
use embodiment_exports::{
    export_embodied_interface_impl, export_imported_capability_reconciliation_impl,
    export_legion_hierarchy_impl, export_soterion_joulework_enforcement_impl,
    export_task_agent_boundaries_impl, export_tauri_embodiment_impl,
};
use extension_exports::{
    export_agent_framework_alignment_impl, export_agentforge_alignment_impl,
    export_eliza_alignment_impl, export_extension_activation_backlog_impl,
    export_extension_surface_contract_impl, export_openfang_alignment_impl,
};
use fleet_exports::{
    export_edge_endpoint_verification_impl, export_fleet_bootstrap_state_impl,
    export_fleet_capability_ranking_impl, export_fleet_identity_reconciliation_impl,
    export_fleet_power_guard_impl, export_fleet_steward_actions_impl,
    export_fleet_steward_write_intents_impl,
};
use hermes_exports::{
    export_edge_enrollment_plan_impl, export_embodied_controller_runtime_impl,
    export_federated_comms_impl, export_github_repo_integration_impl,
    export_matrix_boardroom_contract_impl,
};
use operator_exports::{
    export_communication_adapter_contract_impl, export_nanoclaw_productization_contract_impl,
    export_opencode_productization_contract_impl,
    export_playwright_mcp_productization_contract_impl, export_remote_operator_contract_impl,
    export_tool_garage_contract_impl,
};
use package_exports::{
    export_package_enablement_impl, export_package_health_impl,
    export_package_runtime_activation_impl,
};
use product_exports::{
    export_crate_spawn_contract_impl, export_opencode_project_runtime_impl,
    export_source_lesson_embodiment_backlog_impl, export_source_lesson_embodiment_registry_impl,
};
use research_runtime_exports::{
    export_crawl4ai_runtime_contract_impl, export_external_absorption_brief_impl,
    export_hermes_discord_runtime_impl, export_litellm_routing_contract_impl,
    export_llmfit_routing_contract_impl, export_priority_human_contracts_impl,
    export_priority_human_crate_spawn_registry_impl,
    export_source_ecosystem_operationalization_impl,
};
use runtime_exports::{
    export_edge_model_rollout_impl, export_memory_governor_impl, export_metrics_delta_impl,
    export_runtime_admission_receipts_impl, export_runtime_admission_recovery_impl,
    export_runtime_budget_policy_impl, export_runtime_governor_contract_impl,
    export_scrapling_runtime_contract_impl, export_search_runtime_contract_impl,
};

#[derive(Subcommand, Debug, Clone)]
pub enum ExportCommands {
    /// Export fleet bootstrap state
    FleetBootstrapState,
    /// Export fleet steward actions
    FleetStewardActions,
    /// Export fleet steward write intents
    FleetStewardWriteIntents,
    /// Export fleet power guard
    FleetPowerGuard,
    /// Export fleet identity reconciliation
    FleetIdentityReconciliation,
    /// Export fleet capability ranking
    FleetCapabilityRanking,
    /// Export edge endpoint verification
    EdgeEndpointVerification,
    /// Export operator actions
    OperatorActions,
    /// Export operator legibility contract
    OperatorLegibilityContract,
    /// Export package health surfaces
    PackageHealth,
    /// Export package enablement projection
    PackageEnablement,
    /// Export governance gate matrix
    GateMatrix,
    /// Export plan index surfaces
    PlanIndex,
    /// Export project intake governance
    ProjectIntakeGovernance,
    /// Export ATHENA integration plan
    AthenaIntegrationPlan,
    /// Export Apollo research workflow runtime
    ApolloResearchWorkflowRuntime,
    /// Export AIPKG contract
    AipkgContract,
    /// Export AIPKG edge lab contract
    AipkgEdgeLabContract,
    /// Export AIPKG marketplace separation contract
    AipkgMarketplaceSeparationContract,
    /// Export async user intake contract
    AsyncUserIntakeContract,
    /// Export agent continuity contract
    AgentContinuityContract,
    /// Export ATHENA digest pipeline
    AthenaDigestPipeline,
    /// Export human corpus digest plan
    HumanCorpusDigestPlan,
    /// Export human corpus extraction registry
    HumanCorpusExtractionRegistry,
    /// Export human corpus registry
    HumanCorpusRegistry,
    /// Export intake confidence ladder
    IntakeConfidenceLadder,
    /// Export source absorption executor
    SourceAbsorptionExecutor,
    /// Export source absorption pipeline
    SourceAbsorptionPipeline,
    /// Export source absorption portfolio
    SourceAbsorptionPortfolio,
    /// Export source ecosystem registry
    SourceEcosystemRegistry,
    /// Export client delivery portfolio
    ClientDeliveryPortfolio,
    /// Export client delivery readiness
    ClientDeliveryReadiness,
    /// Export Valinor project dossiers
    ValinorProjectDossiers,
    /// Export imported corpus plan portfolio
    ImportedCorpusPlanPortfolio,
    /// Export Numenor Prime merge registry
    NumenorPrimeMergeRegistry,
    /// Export Valinor merge registry
    ValinorMergeRegistry,
    /// Export OpenFang alignment
    OpenfangAlignment,
    /// Export AgentForge alignment
    AgentforgeAlignment,
    /// Export eliza alignment
    ElizaAlignment,
    /// Export comparative agent framework alignment
    AgentFrameworkAlignment,
    /// Export extension surface contract
    ExtensionSurfaceContract,
    /// Export extension activation backlog
    ExtensionActivationBacklog,
    /// Export embodied interface contract
    EmbodiedInterface,
    /// Export Tauri embodiment stack contract
    TauriEmbodiment,
    /// Export legion hierarchy doctrine
    LegionHierarchy,
    /// Export task-agent boundaries
    TaskAgentBoundaries,
    /// Export Soterion/JouleWork enforcement
    SoterionJouleworkEnforcement,
    /// Export imported capability reconciliation roadmaps
    Rank2CapabilityReconciliation,
    /// Export source ecosystem operationalization
    SourceEcosystemOperationalization,
    /// Export Hermes Discord runtime
    HermesDiscordRuntime,
    /// Export external absorption brief
    ExternalAbsorptionBrief,
    /// Export priority human contracts
    PriorityHumanContracts,
    /// Export priority human crate-spawn registry
    PriorityHumanCrateSpawnRegistry,
    /// Export Crawl4AI runtime contract
    Crawl4aiRuntimeContract,
    /// Export LiteLLM routing contract
    LitellmRoutingContract,
    /// Export llmfit routing contract
    LlmfitRoutingContract,
    /// Export crate spawn contract
    CrateSpawnContract,
    /// Export OpenCode project runtime config/state
    OpencodeProjectRuntime,
    /// Export source lesson embodiment registry
    SourceLessonEmbodimentRegistry,
    /// Export source lesson embodiment backlog
    SourceLessonEmbodimentBacklog,
    /// Export Hermes community sources
    HermesCommunitySources,
    /// Export multi-domain routing contract
    MultiDomainRoutingContract,
    /// Export community signal intake
    CommunitySignalIntake,
    /// Export research workflow contract
    ResearchWorkflowContract,
    /// Export Socratic validator contract
    SocraticValidatorContract,
    /// Export autonomy resume capsule
    AutonomyResume,
    /// Export autonomy task/plan truth reconciliation
    AutonomyTaskTruth,
    /// Export compact active queue projection
    QueueActive,
    /// Export queue hygiene and stale raw-row inflation metrics
    QueueHygiene,
    /// Export read-only federation of subsystem queues and promotion candidates
    QueueFederation,
    /// Export governance-weighted task priority runtime
    GovernancePriorityRuntime,
    /// Export Moria repository MVP runtime contract
    MoriaRepositoryContract,
    /// Export ATHENA active-learning health runtime
    AthenaActiveLearningHealth,
    /// Export Hermes compression credential gate runtime
    HermesCompressionCredentialGate,
    /// Export matrix boardroom contract
    MatrixBoardroomContract,
    /// Export federated communications doctrine/runtime
    FederatedComms,
    /// Export GitHub repo integration surface
    GithubRepoIntegration,
    /// Export embodied controller runtime
    EmbodiedControllerRuntime,
    /// Export edge enrollment plan
    EdgeEnrollmentPlan,
    /// Export edge/package readiness reconciliation
    EdgePackageReadiness,
    /// Export remote operator contract
    RemoteOperatorContract,
    /// Export tool garage contract
    ToolGarageContract,
    /// Export communication adapter contract
    CommunicationAdapterContract,
    /// Export OpenCode productization contract
    OpencodeProductizationContract,
    /// Export Playwright MCP productization contract
    PlaywrightMcpProductizationContract,
    /// Export NanoClaw productization contract
    NanoclawProductizationContract,
    /// Export runtime admission receipts
    RuntimeAdmissionReceipts,
    /// Export runtime budget policy
    RuntimeBudgetPolicy,
    /// Export runtime admission recovery
    RuntimeAdmissionRecovery,
    /// Export memory governor
    MemoryGovernor,
    /// Export metrics delta
    MetricsDelta,
    /// Export package runtime activation
    PackageRuntimeActivation,
    /// Export edge model rollout projection
    EdgeModelRollout,
    /// Export runtime governor contract
    RuntimeGovernorContract,
    /// Export search runtime contract
    SearchRuntimeContract,
    /// Export Scrapling runtime contract
    ScraplingRuntimeContract,
    /// Export network-native node onboarding contract
    NetworkNativeNodeOnboardingContract,
    /// Export edge identity remediation contract
    EdgeIdentityRemediationContract,
    /// Export governance gap backlog
    GovernanceGapBacklog,
    /// Export human augmentation runtime surface
    HumanAugmentationRuntime,
    /// Export CEO council runtime surface
    CeoCouncilRuntime,
    /// Export task lifecycle runtime surface
    TaskLifecycleRuntime,
    /// Export L3 readiness projection for ARDA and Hermes operator surfaces
    L3ReadinessProjection,
}

#[derive(Debug)]
struct SignalRule {
    key: &'static str,
    patterns: &'static [&'static str],
}

const SIGNALS: &[SignalRule] = &[
    SignalRule {
        key: "soterion",
        patterns: &["SoterionMeta", "soterion::", "sigil:", "sigil ="],
    },
    SignalRule {
        key: "joulework",
        patterns: &["joulework", "joule_work", "JouleWork"],
    },
    SignalRule {
        key: "love_equation",
        patterns: &["love_equation", "LoveEquation", "love_eq"],
    },
    SignalRule {
        key: "bacon_lite",
        patterns: &["bacon_lite", "record_bacon_lite", "BaconLite"],
    },
    SignalRule {
        key: "triad",
        patterns: &["triad_validate", "triad", "Triad"],
    },
];

pub fn run(command: ExportCommands) -> Result<Value> {
    match command {
        ExportCommands::FleetBootstrapState => export_fleet_bootstrap_state(),
        ExportCommands::FleetStewardActions => export_fleet_steward_actions(),
        ExportCommands::FleetStewardWriteIntents => export_fleet_steward_write_intents(),
        ExportCommands::FleetPowerGuard => export_fleet_power_guard(),
        ExportCommands::FleetIdentityReconciliation => export_fleet_identity_reconciliation(),
        ExportCommands::FleetCapabilityRanking => export_fleet_capability_ranking(),
        ExportCommands::EdgeEndpointVerification => export_edge_endpoint_verification(),
        ExportCommands::OperatorActions => export_operator_actions(),
        ExportCommands::OperatorLegibilityContract => export_operator_legibility_contract(),
        ExportCommands::PackageHealth => export_package_health(),
        ExportCommands::PackageEnablement => export_package_enablement(),
        ExportCommands::GateMatrix => export_gate_matrix(),
        ExportCommands::PlanIndex => export_plan_index(),
        ExportCommands::ProjectIntakeGovernance => export_project_intake_governance(),
        ExportCommands::AthenaIntegrationPlan => export_athena_integration_plan(),
        ExportCommands::ApolloResearchWorkflowRuntime => export_apollo_research_workflow_runtime(),
        ExportCommands::AipkgContract => export_aipkg_contract(),
        ExportCommands::AipkgEdgeLabContract => export_aipkg_edge_lab_contract(),
        ExportCommands::AipkgMarketplaceSeparationContract => {
            export_aipkg_marketplace_separation_contract()
        }
        ExportCommands::AsyncUserIntakeContract => export_async_user_intake_contract(),
        ExportCommands::AgentContinuityContract => export_agent_continuity_contract(),
        ExportCommands::AthenaDigestPipeline => export_athena_digest_pipeline(),
        ExportCommands::HumanCorpusDigestPlan => export_human_corpus_digest_plan(),
        ExportCommands::HumanCorpusExtractionRegistry => export_human_corpus_extraction_registry(),
        ExportCommands::HumanCorpusRegistry => export_human_corpus_registry(),
        ExportCommands::IntakeConfidenceLadder => export_intake_confidence_ladder(),
        ExportCommands::SourceAbsorptionExecutor => export_source_absorption_executor(),
        ExportCommands::SourceAbsorptionPipeline => export_source_absorption_pipeline(),
        ExportCommands::SourceAbsorptionPortfolio => export_source_absorption_portfolio(),
        ExportCommands::SourceEcosystemRegistry => export_source_ecosystem_registry(),
        ExportCommands::ClientDeliveryPortfolio => export_client_delivery_portfolio(),
        ExportCommands::ClientDeliveryReadiness => export_client_delivery_readiness(),
        ExportCommands::ValinorProjectDossiers => export_valinor_project_dossiers(),
        ExportCommands::ImportedCorpusPlanPortfolio => export_imported_corpus_plan_portfolio(),
        ExportCommands::NumenorPrimeMergeRegistry => export_numenor_prime_merge_registry(),
        ExportCommands::ValinorMergeRegistry => export_valinor_merge_registry(),
        ExportCommands::OpenfangAlignment => export_openfang_alignment(),
        ExportCommands::AgentforgeAlignment => export_agentforge_alignment(),
        ExportCommands::ElizaAlignment => export_eliza_alignment(),
        ExportCommands::AgentFrameworkAlignment => export_agent_framework_alignment(),
        ExportCommands::ExtensionSurfaceContract => export_extension_surface_contract(),
        ExportCommands::ExtensionActivationBacklog => export_extension_activation_backlog(),
        ExportCommands::EmbodiedInterface => export_embodied_interface(),
        ExportCommands::TauriEmbodiment => export_tauri_embodiment(),
        ExportCommands::LegionHierarchy => export_legion_hierarchy(),
        ExportCommands::TaskAgentBoundaries => export_task_agent_boundaries(),
        ExportCommands::SoterionJouleworkEnforcement => export_soterion_joulework_enforcement(),
        ExportCommands::Rank2CapabilityReconciliation => export_rank2_capability_reconciliation(),
        ExportCommands::SourceEcosystemOperationalization => {
            export_source_ecosystem_operationalization()
        }
        ExportCommands::HermesDiscordRuntime => export_hermes_discord_runtime(),
        ExportCommands::ExternalAbsorptionBrief => export_external_absorption_brief(),
        ExportCommands::PriorityHumanContracts => export_priority_human_contracts(),
        ExportCommands::PriorityHumanCrateSpawnRegistry => {
            export_priority_human_crate_spawn_registry()
        }
        ExportCommands::Crawl4aiRuntimeContract => export_crawl4ai_runtime_contract(),
        ExportCommands::LitellmRoutingContract => export_litellm_routing_contract(),
        ExportCommands::LlmfitRoutingContract => export_llmfit_routing_contract(),
        ExportCommands::CrateSpawnContract => export_crate_spawn_contract(),
        ExportCommands::OpencodeProjectRuntime => export_opencode_project_runtime(),
        ExportCommands::SourceLessonEmbodimentRegistry => {
            export_source_lesson_embodiment_registry()
        }
        ExportCommands::SourceLessonEmbodimentBacklog => export_source_lesson_embodiment_backlog(),
        ExportCommands::HermesCommunitySources => export_hermes_community_sources(),
        ExportCommands::MultiDomainRoutingContract => export_multi_domain_routing_contract(),
        ExportCommands::CommunitySignalIntake => export_community_signal_intake(),
        ExportCommands::ResearchWorkflowContract => export_research_workflow_contract(),
        ExportCommands::SocraticValidatorContract => export_socratic_validator_contract(),
        ExportCommands::AutonomyResume => export_autonomy_resume(),
        ExportCommands::AutonomyTaskTruth => export_autonomy_task_truth(),
        ExportCommands::QueueActive => export_queue_active(),
        ExportCommands::QueueHygiene => export_queue_hygiene(),
        ExportCommands::QueueFederation => export_queue_federation(),
        ExportCommands::GovernancePriorityRuntime => export_governance_priority_runtime(),
        ExportCommands::MoriaRepositoryContract => export_moria_repository_contract(),
        ExportCommands::AthenaActiveLearningHealth => export_athena_active_learning_health(),
        ExportCommands::HermesCompressionCredentialGate => {
            export_hermes_compression_credential_gate()
        }
        ExportCommands::MatrixBoardroomContract => export_matrix_boardroom_contract(),
        ExportCommands::FederatedComms => export_federated_comms(),
        ExportCommands::GithubRepoIntegration => export_github_repo_integration(),
        ExportCommands::EmbodiedControllerRuntime => export_embodied_controller_runtime(),
        ExportCommands::EdgeEnrollmentPlan => export_edge_enrollment_plan(),
        ExportCommands::EdgePackageReadiness => export_edge_package_readiness(),
        ExportCommands::RemoteOperatorContract => export_remote_operator_contract(),
        ExportCommands::ToolGarageContract => export_tool_garage_contract(),
        ExportCommands::CommunicationAdapterContract => export_communication_adapter_contract(),
        ExportCommands::OpencodeProductizationContract => export_opencode_productization_contract(),
        ExportCommands::PlaywrightMcpProductizationContract => {
            export_playwright_mcp_productization_contract()
        }
        ExportCommands::NanoclawProductizationContract => export_nanoclaw_productization_contract(),
        ExportCommands::RuntimeAdmissionReceipts => export_runtime_admission_receipts(),
        ExportCommands::RuntimeBudgetPolicy => export_runtime_budget_policy(),
        ExportCommands::RuntimeAdmissionRecovery => export_runtime_admission_recovery(),
        ExportCommands::MemoryGovernor => export_memory_governor(),
        ExportCommands::MetricsDelta => export_metrics_delta(),
        ExportCommands::PackageRuntimeActivation => export_package_runtime_activation(),
        ExportCommands::EdgeModelRollout => export_edge_model_rollout(),
        ExportCommands::RuntimeGovernorContract => export_runtime_governor_contract(),
        ExportCommands::SearchRuntimeContract => export_search_runtime_contract(),
        ExportCommands::ScraplingRuntimeContract => export_scrapling_runtime_contract(),
        ExportCommands::NetworkNativeNodeOnboardingContract => {
            export_network_native_node_onboarding_contract()
        }
        ExportCommands::EdgeIdentityRemediationContract => {
            export_edge_identity_remediation_contract()
        }
        ExportCommands::GovernanceGapBacklog => export_governance_gap_backlog(),
        ExportCommands::HumanAugmentationRuntime => export_human_augmentation_runtime(),
        ExportCommands::CeoCouncilRuntime => export_ceo_council_runtime(),
        ExportCommands::TaskLifecycleRuntime => export_task_lifecycle_runtime(),
        ExportCommands::L3ReadinessProjection => export_l3_readiness_projection(),
    }
}

pub(crate) fn workspace_root() -> PathBuf {
    super::annunimas_root()
}

pub(crate) fn numenor_prime_root() -> PathBuf {
    env::var("ANNUNIMAS_NUMENOR_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| super::home_root().join("Numenor_Prime"))
}

pub(crate) fn valinor_root() -> PathBuf {
    numenor_prime_root().join("Valinor")
}

pub(crate) fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub(crate) fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    if existing_json_matches_except_generated_at(path, value) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn existing_json_matches_except_generated_at(path: &Path, value: &Value) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(existing) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };

    let mut existing_without_timestamp = existing;
    let mut next_without_timestamp = value.clone();
    strip_generated_at_utc(&mut existing_without_timestamp);
    strip_generated_at_utc(&mut next_without_timestamp);
    existing_without_timestamp == next_without_timestamp
}

fn strip_generated_at_utc(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("generated_at_utc");
            for child in map.values_mut() {
                strip_generated_at_utc(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_generated_at_utc(child);
            }
        }
        _ => {}
    }
}

fn parse_toml_document(raw: &str) -> Result<toml::Value> {
    let content = if let Some((_, tail)) = raw.split_once("```toml") {
        tail.split_once("```").map(|(body, _)| body).unwrap_or(tail)
    } else {
        raw
    };
    Ok(toml::from_str(content.trim())?)
}

pub(crate) fn read_toml_or(path: &Path, default: toml::Value) -> toml::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| parse_toml_document(&raw).ok())
        .unwrap_or(default)
}

pub(crate) fn read_json_or(path: &Path, default: Value) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(default)
}

fn export_human_augmentation_runtime() -> Result<Value> {
    let root = workspace_root();
    let approvals_path = root.join("core/state/human_augmentation_approval.json");
    let ruleset_path = root.join("core/state/active_ruleset.json");
    let out_path = root.join("core/state/human_augmentation_runtime.json");
    let approvals = read_json_or(
        &approvals_path,
        json!({
            "schema_version": "annunimas.human-augmentation-approval.v1",
            "approvals": []
        }),
    );
    let ruleset = read_json_or(&ruleset_path, json!({}));
    let policy = ruleset.get("policy").cloned().unwrap_or_else(|| json!({}));
    let human_augmentation = policy
        .get("human_augmentation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let approvals_rows = approvals
        .get("approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let pending_total = approvals_rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("pending"))
        .count();
    let approved_total = approvals_rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("approved"))
        .count();
    let snapshot = json!({
        "schema_version": "annunimas.human-augmentation-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "human_augmentation_runtime_export",
        "policy": human_augmentation,
        "approvals": approvals_rows,
        "summary": {
            "pending_total": pending_total,
            "approved_total": approved_total
        },
        "paths": {
            "approvals": rel(&approvals_path, &root),
            "active_ruleset": rel(&ruleset_path, &root)
        },
        "arda_hints": {
            "primary_panel": "human_augmentation_runtime",
            "boardroom_section": "governance_guardhouse",
            "highlight_pending_approvals": pending_total > 0
        }
    });
    write_pretty_json(&out_path, &snapshot)?;
    Ok(snapshot)
}

fn export_ceo_council_runtime() -> Result<Value> {
    let root = workspace_root();
    let sessions_path = root.join("core/state/ceo_council_sessions.json");
    let ruleset_path = root.join("core/state/active_ruleset.json");
    let out_path = root.join("core/state/ceo_council_runtime.json");
    let sessions = read_json_or(
        &sessions_path,
        json!({
            "schema_version": "annunimas.ceo-council-sessions.v1",
            "sessions": []
        }),
    );
    let ruleset = read_json_or(&ruleset_path, json!({}));
    let policy = ruleset.get("policy").cloned().unwrap_or_else(|| json!({}));
    let human_augmentation = policy
        .get("human_augmentation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let sessions_rows = sessions
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total_sessions = sessions_rows.len();
    let triad_sessions = sessions_rows
        .iter()
        .filter(|row| row.get("triad_required").and_then(Value::as_bool) == Some(true))
        .count();
    let lightweight_sessions = sessions_rows
        .iter()
        .filter(|row| row.get("loop_class").and_then(Value::as_str) == Some("lightweight"))
        .count();
    let human_escalations = sessions_rows
        .iter()
        .filter(|row| row.get("human_escalated").and_then(Value::as_bool) == Some(true))
        .count();
    let promoted_private_memory = sessions_rows
        .iter()
        .filter(|row| row.get("promoted_private_memory").and_then(Value::as_bool) == Some(true))
        .count();
    let snapshot = json!({
        "schema_version": "annunimas.ceo-council-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "ceo_council_runtime_export",
        "policy": {
            "human_augmentation": human_augmentation,
            "memory_lanes": [
                {
                    "id": "human_sovereign",
                    "retention": "durable",
                    "promotion_required": false
                },
                {
                    "id": "ceo_private_working",
                    "retention": "expiring",
                    "promotion_required": true
                },
                {
                    "id": "shared_executive",
                    "retention": "durable",
                    "promotion_required": false
                },
                {
                    "id": "institutional",
                    "retention": "durable",
                    "promotion_required": true
                },
                {
                    "id": "episodic",
                    "retention": "expiring",
                    "promotion_required": false
                }
            ]
        },
        "sessions": sessions_rows,
        "summary": {
            "total_sessions": total_sessions,
            "triad_sessions": triad_sessions,
            "lightweight_sessions": lightweight_sessions,
            "human_escalations": human_escalations,
            "promoted_private_memory_total": promoted_private_memory
        },
        "paths": {
            "sessions": rel(&sessions_path, &root),
            "active_ruleset": rel(&ruleset_path, &root)
        },
        "arda_hints": {
            "primary_panel": "ceo_council_runtime",
            "boardroom_section": "executive_council",
            "highlight_escalations": human_escalations > 0
        }
    });
    write_pretty_json(&out_path, &snapshot)?;
    Ok(snapshot)
}

fn export_task_lifecycle_runtime() -> Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let plan_map_path = root.join("core/state/plan_map.json");
    let out_path = root.join("core/state/task_lifecycle_runtime.json");

    let raw_tasks = read_jsonl_objects(&queue_path);
    let tasks = latest_project_tasks(&raw_tasks);
    let plan_map = read_json_or(&plan_map_path, json!({}));

    let mut queued_total = 0usize;
    let mut active_total = 0usize;
    let mut completed_total = 0usize;
    let mut cancelled_total = 0usize;
    let mut disposal_review_total = 0usize;
    let mut archive_ready_total = 0usize;

    let mut owner_counts = BTreeMap::<String, usize>::new();
    let mut disposal_candidates = Vec::new();

    for task in &tasks {
        let status = task
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let owner = task
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let glyphs = task
            .get("glyphs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_disposal_glyph = glyphs.iter().any(|glyph| glyph.as_str() == Some("↝"));

        match normalize_task_status(status) {
            "pending" | "queued" => {
                queued_total += 1;
                active_total += 1;
            }
            "in_progress" => {
                active_total += 1;
            }
            "completed" => {
                completed_total += 1;
                if has_disposal_glyph {
                    disposal_review_total += 1;
                    disposal_candidates.push(json!({
                        "id": task.get("id").cloned().unwrap_or(json!(null)),
                        "title": task.get("title").cloned().unwrap_or(json!(null)),
                        "owner": owner,
                        "completed_at_utc": task.get("completed_at_utc").cloned().unwrap_or(json!(null)),
                        "disposal_marker": "↝",
                        "next_phase": "hades_disposal_review"
                    }));
                } else {
                    archive_ready_total += 1;
                }
            }
            "cancelled" => {
                cancelled_total += 1;
                if has_disposal_glyph {
                    disposal_review_total += 1;
                }
            }
            _ => {}
        }

        *owner_counts.entry(owner).or_insert(0) += 1;
    }

    let phases = json!([
        {
            "id": "plan",
            "description": "Doctrine or strategy source that defines desired work."
        },
        {
            "id": "task_emission",
            "description": "Executable task emitted into the queue ledger."
        },
        {
            "id": "task_retrieval",
            "description": "Retrieval of queued work by subsystem, operator, or pipeline."
        },
        {
            "id": "bounded_execution",
            "description": "Execution by a bounded subsystem or operator workflow."
        },
        {
            "id": "completion_evidence",
            "description": "Completion result written back into queue state with evidence."
        },
        {
            "id": "hades_disposal_review",
            "description": "Completed work marked with ↝ is eligible for HADES lifecycle review, not immediate deletion."
        },
        {
            "id": "archive_or_retention",
            "description": "Final posture after HADES review: archive, retain, compact, or remove."
        }
    ]);

    let snapshot = json!({
        "schema_version": "annunimas.task-lifecycle-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "task_lifecycle_runtime_export",
        "contract": {
            "pipeline": "plan -> task_emission -> task_retrieval -> bounded_execution -> completion_evidence -> hades_disposal_review -> archive_or_retention",
            "completion_is_not_disposal": true,
            "disposal_is_not_deletion": true,
            "hades_is_lifecycle_boundary": true
        },
        "phases": phases,
        "metrics": {
            "raw_ledger_rows_total": raw_tasks.len(),
            "latest_task_ids_total": tasks.len(),
            "queued_total": queued_total,
            "active_total": active_total,
            "completed_total": completed_total,
            "cancelled_total": cancelled_total,
            "disposal_review_total": disposal_review_total,
            "archive_ready_total": archive_ready_total,
            "owner_counts": owner_counts
        },
        "disposal_candidates": disposal_candidates,
        "plan_context": {
            "core_plan_index_present": plan_map.is_object(),
            "core_plan_sections": plan_map.get("sections").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
        },
        "paths": {
            "queue": rel(&queue_path, &root),
            "core_plan_index": rel(&plan_map_path, &root)
        },
        "arda_hints": {
            "primary_panel": "task_lifecycle_runtime",
            "boardroom_section": "operations_flow",
            "highlight_disposal_review": disposal_review_total > 0
        }
    });
    write_pretty_json(&out_path, &snapshot)?;
    Ok(snapshot)
}

fn export_l3_readiness_projection() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/l3_readiness_projection.json");
    let flywheel_path = root.join("core/state/flywheel_packet_runtime.json");
    let queue_hygiene_path = root.join("core/state/queue_hygiene.json");
    let task_lifecycle_path = root.join("core/state/task_lifecycle_runtime.json");
    let queue_federation_path = root.join("core/state/queue_federation.json");
    let l3_receipt_path = root.join("data/prometheus/l3_e2e_receipt.json");
    let hades_report_path = root.join("data/hades/lifecycle_policy_automation_report.json");
    let hades_rollback_path = root.join("data/hades/lifecycle_cleanup_rollback_evidence.json");

    let flywheel = read_json_or(&flywheel_path, json!({}));
    let queue_hygiene = read_json_or(&queue_hygiene_path, json!({}));
    let task_lifecycle = read_json_or(&task_lifecycle_path, json!({}));
    let queue_federation = read_json_or(&queue_federation_path, json!({}));
    let l3_receipt = read_json_or(&l3_receipt_path, json!({}));
    let hades_report = read_json_or(&hades_report_path, json!({}));
    let hades_rollback = read_json_or(&hades_rollback_path, json!({}));

    let packets = flywheel
        .get("packets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let next_ready_packet = packets
        .iter()
        .find(|packet| packet.get("readiness").and_then(Value::as_str) == Some("ready"))
        .cloned()
        .unwrap_or(Value::Null);
    let l3c_p6_completed = packets.iter().any(|packet| {
        packet.get("packet_id").and_then(Value::as_str) == Some("L3C-P6")
            && packet.get("readiness").and_then(Value::as_str) == Some("completed")
    });
    let readiness_counts = flywheel
        .pointer("/summary/readiness_counts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let completed_packet_total = readiness_counts
        .get("completed")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let packet_total = flywheel
        .pointer("/summary/packet_total")
        .and_then(Value::as_u64)
        .unwrap_or(packets.len() as u64);
    let ready_total = flywheel
        .pointer("/summary/ready_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let blocked_total = flywheel
        .pointer("/summary/blocked_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let verify_status = l3_receipt
        .get("verify_status")
        .and_then(Value::as_str)
        .unwrap_or("missing_l3_receipt");
    let queue_latest_counts = queue_hygiene
        .pointer("/counts/latest_by_status")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let active_total = queue_federation
        .pointer("/central_backlog/active_total")
        .or_else(|| queue_hygiene.pointer("/metrics/latest_open_total"))
        .cloned()
        .unwrap_or(json!(0));
    let promotion_ready_total = queue_federation
        .pointer("/summary/promotion_ready_total")
        .cloned()
        .unwrap_or(json!(0));
    let completed_total = queue_latest_counts
        .get("completed")
        .cloned()
        .unwrap_or(json!(0));
    let pending_total = queue_latest_counts
        .get("pending")
        .cloned()
        .unwrap_or(json!(0));
    let queued_total = queue_latest_counts
        .get("queued")
        .cloned()
        .unwrap_or(json!(0));
    let hades_executed = hades_rollback
        .get("executed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let hades_no_deletes = hades_rollback
        .get("no_file_moves_or_deletes_performed")
        .or_else(|| hades_report.get("no_file_moves_or_deletes_performed"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let payload = json!({
        "schema_version": "annunimas.l3-readiness-projection.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export l3-readiness-projection",
        "projection_policy": {
            "read_only": true,
            "operator_surface_only": true,
            "grants_mutation_authority": false,
            "approval_authority": "none_projection_only"
        },
        "status": {
            "level": if l3c_p6_completed && verify_status == "passed" {
                "l3_safe_local_harness_proven_projection_only"
            } else {
                "l3_not_proven"
            },
            "bounded_mutation_ready": l3c_p6_completed && verify_status == "passed",
            "broad_mutation_authorized": false,
            "external_side_effects_authorized": false,
            "destructive_actions_authorized": false
        },
        "allowed_action_classes": [
            "l3_local_doc_fixture_patch",
            "local_refactor_with_tests",
            "documentation_indexing"
        ],
        "blocked_action_classes": [
            "destructive_delete",
            "archive_or_retention_mutation_without_hades_approval",
            "credential_change",
            "external_message_send",
            "service_restart",
            "provider_reload_or_reroute",
            "fleet_reimage_or_remote_mutation",
            "funds_movement",
            "legal_or_customer_commitment"
        ],
        "flywheel": {
            "packet_total": packet_total,
            "ready_total": ready_total,
            "blocked_total": blocked_total,
            "readiness_counts": readiness_counts,
            "next_ready_packet": next_ready_packet
        },
        "harness": {
            "task_id": l3_receipt.get("task_id").cloned().unwrap_or(json!("tsk_plan_l3c_p6_ded5b63c05")),
            "class_id": l3_receipt.get("class_id").cloned().unwrap_or(json!("l3_local_doc_fixture_patch")),
            "verify_status": verify_status,
            "policy_mode": l3_receipt.get("policy_mode").cloned().unwrap_or(json!("unknown")),
            "append_only_guard": l3_receipt.get("append_only_guard").cloned().unwrap_or(json!("unknown"))
        },
        "hades_lifecycle": {
            "review_completed": hades_report.get("source_findings_total").is_some(),
            "cleanup_authorized": hades_report.get("cleanup_authorized").cloned().unwrap_or(json!(false)),
            "source_findings_total": hades_report.get("source_findings_total").cloned().unwrap_or(json!(0)),
            "planned_actions_total": hades_rollback.get("planned_actions_total").cloned().unwrap_or(json!(0)),
            "executed": hades_executed,
            "no_file_moves_or_deletes_performed": hades_no_deletes,
            "requires_operator_approval_for_mutation": hades_report.get("requires_operator_approval_for_mutation").cloned().unwrap_or(json!(true))
        },
        "queue": {
            "canonical_path": "core/projects/tasks/queue.jsonl",
            "active_total": active_total,
            "completed_total": completed_total,
            "pending_total": pending_total,
            "queued_total": queued_total,
            "latest_task_ids_total": queue_hygiene.pointer("/metrics/latest_task_ids_total").cloned().unwrap_or(json!(0)),
            "raw_ledger_rows_total": queue_hygiene.pointer("/metrics/raw_ledger_rows_total").cloned().unwrap_or(json!(0))
        },
        "federation": {
            "sources_total": queue_federation.pointer("/summary/sources_total").cloned().unwrap_or(json!(0)),
            "promotion_candidates_total": queue_federation.pointer("/summary/promotion_candidates_total").cloned().unwrap_or(json!(0)),
            "promotion_ready_total": promotion_ready_total,
            "blocked_total": queue_federation.pointer("/summary/blocked_total").cloned().unwrap_or(json!(0)),
            "human_lane_included": true
        },
        "task_lifecycle": {
            "contract": task_lifecycle.get("contract").cloned().unwrap_or_else(|| json!({})),
            "disposal_review_total": task_lifecycle.pointer("/metrics/disposal_review_total").cloned().unwrap_or(json!(0)),
            "archive_ready_total": task_lifecycle.pointer("/metrics/archive_ready_total").cloned().unwrap_or(json!(0))
        },
        "operator_surfaces": {
            "arda": {
                "path": "apps/arda-hud/src/lib/ardaSource.ts",
                "bundle_field": "l3ReadinessProjection",
                "mode": "read_only_projection"
            },
            "hermes": {
                "path": "crates/annunimas-hermes/src/service/runtime.rs",
                "ipc_command": "l3_readiness",
                "mode": "read_only_projection"
            }
        },
        "source_paths": {
            "flywheel": rel(&flywheel_path, &root),
            "queue_hygiene": rel(&queue_hygiene_path, &root),
            "task_lifecycle": rel(&task_lifecycle_path, &root),
            "queue_federation": rel(&queue_federation_path, &root),
            "l3_receipt": rel(&l3_receipt_path, &root),
            "hades_lifecycle_report": rel(&hades_report_path, &root),
            "hades_rollback_evidence": rel(&hades_rollback_path, &root)
        },
        "recommendation": if ready_total > 0 {
            "execute_next_ready_flywheel_packet_after_projection_refresh"
        } else if packet_total > 0 && completed_packet_total == packet_total {
            "l3_readiness_closure_plan_complete_select_next_non_l3_backlog_or_hades_lifecycle_review"
        } else if blocked_total > 0 {
            "resolve_blocked_flywheel_packet_dependencies"
        } else {
            "refresh_flywheel_packet_runtime"
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(payload)
}

fn latest_project_tasks(rows: &[Value]) -> Vec<Value> {
    let mut latest = BTreeMap::<String, Value>::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        latest.insert(id.to_string(), row.clone());
    }
    latest.into_values().collect()
}

fn task_status_counts(tasks: &[Value]) -> serde_json::Map<String, Value> {
    let mut counts = serde_json::Map::new();
    for task in tasks {
        let status = normalize_task_status(
            task.get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        );
        let current = counts.get(status).and_then(Value::as_u64).unwrap_or(0);
        counts.insert(status.to_string(), json!(current + 1));
    }
    counts
}

fn is_open_task(task: &Value) -> bool {
    matches!(
        normalize_task_status(task.get("status").and_then(Value::as_str).unwrap_or("")),
        "pending" | "queued" | "in_progress"
    )
}

fn compact_project_task(task: &Value) -> Value {
    json!({
        "id": task.get("id").cloned().unwrap_or(Value::Null),
        "title": task.get("title").cloned().unwrap_or(Value::Null),
        "owner": task.get("owner").cloned().unwrap_or(Value::Null),
        "priority": task.get("priority").cloned().unwrap_or(Value::Null),
        "status": task.get("status").cloned().unwrap_or(Value::Null),
        "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
        "origin": task.get("meta").and_then(|meta| meta.get("origin")).cloned().unwrap_or(Value::Null),
        "scope": task.get("meta").and_then(|meta| meta.get("scope")).cloned().unwrap_or(Value::Null),
    })
}

fn normalize_task_status(status: &str) -> &str {
    match status {
        "complete" | "done" => "completed",
        "active" | "running" => "in_progress",
        other => other,
    }
}

pub(crate) fn read_jsonl_objects(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
                .filter(|value| value.is_object())
                .collect()
        })
        .unwrap_or_default()
}

fn rg_files(patterns: &[&str], base: &Path, cwd: &Path) -> Vec<String> {
    if !base.exists() {
        return Vec::new();
    }
    let mut cmd = Command::new("rg");
    cmd.arg("-l").arg("-i");
    for pattern in patterns {
        cmd.arg("-e").arg(pattern);
    }
    cmd.arg("--").arg(base);
    let output = match cmd.current_dir(cwd).output() {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };
    if !output.status.success() && output.status.code() != Some(1) {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn export_fleet_bootstrap_state() -> Result<Value> {
    export_fleet_bootstrap_state_impl()
}

fn export_edge_endpoint_verification() -> Result<Value> {
    export_edge_endpoint_verification_impl()
}

fn export_fleet_steward_actions() -> Result<Value> {
    export_fleet_steward_actions_impl()
}

fn export_fleet_steward_write_intents() -> Result<Value> {
    export_fleet_steward_write_intents_impl()
}

fn export_fleet_power_guard() -> Result<Value> {
    export_fleet_power_guard_impl()
}

fn export_fleet_identity_reconciliation() -> Result<Value> {
    export_fleet_identity_reconciliation_impl()
}

fn export_fleet_capability_ranking() -> Result<Value> {
    export_fleet_capability_ranking_impl()
}

fn export_project_intake_governance() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/project_intake_governance.json");
    let dossier_standard = read_json_or(
        &root.join("core/state/project_dossier_standard.json"),
        json!({}),
    );
    let intake_contract = read_json_or(
        &root.join("core/state/imported_memory_intake_contract.json"),
        json!({}),
    );
    let classification = read_json_or(
        &root.join("core/state/portfolio_classification_posture.json"),
        json!({}),
    );
    let lifecycle = read_json_or(
        &root.join("core/state/project_intake_lifecycle.json"),
        json!({}),
    );
    let valinor_dossiers = read_json_or(
        &root.join("core/state/valinor_project_dossiers.json"),
        json!({}),
    );
    let plan_portfolio = read_json_or(
        &root.join("core/state/imported_corpus_plan_portfolio.json"),
        json!({}),
    );
    let review_digest = read_json_or(
        &root.join("core/state/imported_corpus_review_digest.json"),
        json!({}),
    );

    let dossiers = valinor_dossiers
        .get("dossiers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let portfolio_entries = plan_portfolio
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let compliant_total = dossiers
        .iter()
        .filter(|row| {
            row.get("dossier_standard_compliance")
                .and_then(|value| value.get("compliant"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let build_now_total = dossiers
        .iter()
        .filter(|row| row.get("review_posture").and_then(Value::as_str) == Some("build_now"))
        .count();
    let execution_eligible_total = dossiers
        .iter()
        .filter(|row| row.get("review_posture").and_then(Value::as_str) == Some("build_now"))
        .filter(|row| {
            row.get("readiness").and_then(Value::as_str) == Some("ready_for_content_and_code")
        })
        .count();
    let dossier_overview = dossiers
        .iter()
        .map(|row| {
            json!({
                "slug": row.get("slug").cloned().unwrap_or(Value::Null),
                "review_posture": row.get("review_posture").cloned().unwrap_or(Value::Null),
                "readiness": row.get("readiness").cloned().unwrap_or(Value::Null),
                "compliant": row
                    .get("dossier_standard_compliance")
                    .and_then(|value| value.get("compliant"))
                    .cloned()
                    .unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "annunimas.project-intake-governance.v1",
        "generated_at_utc": now_utc(),
        "authority": "project_dossier_standard + imported_memory_intake_contract + portfolio_classification_posture + project_intake_lifecycle",
        "status": "active_governance_surface",
        "summary": {
            "required_dossier_fields_total": dossier_standard
                .get("required_fields")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "intake_states_total": intake_contract
                .get("states")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "classification_labels_total": classification
                .get("labels")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "lifecycle_stages_total": lifecycle
                .get("stages")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "tracked_dossiers_total": dossiers.len(),
            "dossier_standard_compliant_total": compliant_total,
            "build_now_total": build_now_total,
            "execution_eligible_total": execution_eligible_total,
            "portfolio_entries_total": portfolio_entries.len(),
        },
        "binding": {
            "dossier_workflow": "core/state/valinor_project_dossiers.json",
            "queue_workflow": "core/state/task_agent_boundaries.json",
            "restart_resume_workflow": "core/state/autonomy_resume.json",
            "review_digest": "core/state/imported_corpus_review_digest.json",
        },
        "first_execution_tranche": review_digest
            .get("first_execution_tranche_after_approval")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "second_execution_tranche": review_digest
            .get("second_execution_tranche_after_approval")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "dossier_overview": dossier_overview,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "tracked_dossiers_total": dossiers.len()
    }))
}

fn export_athena_integration_plan() -> Result<Value> {
    export_athena_integration_plan_impl()
}

fn export_apollo_research_workflow_runtime() -> Result<Value> {
    export_apollo_research_workflow_runtime_impl()
}

fn export_aipkg_contract() -> Result<Value> {
    export_aipkg_contract_impl()
}

fn export_aipkg_edge_lab_contract() -> Result<Value> {
    export_aipkg_edge_lab_contract_impl()
}

fn export_aipkg_marketplace_separation_contract() -> Result<Value> {
    export_aipkg_marketplace_separation_contract_impl()
}

fn export_async_user_intake_contract() -> Result<Value> {
    export_async_user_intake_contract_impl()
}

fn export_agent_continuity_contract() -> Result<Value> {
    export_agent_continuity_contract_impl()
}

fn export_athena_digest_pipeline() -> Result<Value> {
    export_athena_digest_pipeline_impl()
}

fn export_human_corpus_digest_plan() -> Result<Value> {
    export_human_corpus_digest_plan_impl()
}

fn export_human_corpus_extraction_registry() -> Result<Value> {
    export_human_corpus_extraction_registry_impl()
}

fn export_human_corpus_registry() -> Result<Value> {
    export_human_corpus_registry_impl()
}

fn export_intake_confidence_ladder() -> Result<Value> {
    export_intake_confidence_ladder_impl()
}

fn export_source_absorption_executor() -> Result<Value> {
    export_source_absorption_executor_impl()
}

fn export_source_absorption_pipeline() -> Result<Value> {
    export_source_absorption_pipeline_impl()
}

fn export_source_absorption_portfolio() -> Result<Value> {
    export_source_absorption_portfolio_impl()
}

fn export_source_ecosystem_registry() -> Result<Value> {
    export_source_ecosystem_registry_impl()
}

fn export_client_delivery_portfolio() -> Result<Value> {
    export_client_delivery_portfolio_impl()
}

fn export_client_delivery_readiness() -> Result<Value> {
    export_client_delivery_readiness_impl()
}

fn export_valinor_project_dossiers() -> Result<Value> {
    export_valinor_project_dossiers_impl()
}

fn export_imported_corpus_plan_portfolio() -> Result<Value> {
    export_imported_corpus_plan_portfolio_impl()
}

fn export_numenor_prime_merge_registry() -> Result<Value> {
    export_numenor_prime_merge_registry_impl()
}

fn export_valinor_merge_registry() -> Result<Value> {
    export_valinor_merge_registry_impl()
}

fn export_openfang_alignment() -> Result<Value> {
    export_openfang_alignment_impl()
}

fn export_agentforge_alignment() -> Result<Value> {
    export_agentforge_alignment_impl()
}

fn export_eliza_alignment() -> Result<Value> {
    export_eliza_alignment_impl()
}

fn export_agent_framework_alignment() -> Result<Value> {
    export_agent_framework_alignment_impl()
}

fn export_extension_surface_contract() -> Result<Value> {
    export_extension_surface_contract_impl()
}

fn export_extension_activation_backlog() -> Result<Value> {
    export_extension_activation_backlog_impl()
}

fn export_embodied_interface() -> Result<Value> {
    export_embodied_interface_impl()
}

fn export_tauri_embodiment() -> Result<Value> {
    export_tauri_embodiment_impl()
}

fn export_legion_hierarchy() -> Result<Value> {
    export_legion_hierarchy_impl()
}

fn export_task_agent_boundaries() -> Result<Value> {
    export_task_agent_boundaries_impl()
}

fn export_soterion_joulework_enforcement() -> Result<Value> {
    export_soterion_joulework_enforcement_impl()
}

fn export_rank2_capability_reconciliation() -> Result<Value> {
    export_imported_capability_reconciliation_impl()
}

fn export_source_ecosystem_operationalization() -> Result<Value> {
    export_source_ecosystem_operationalization_impl()
}

fn export_hermes_discord_runtime() -> Result<Value> {
    export_hermes_discord_runtime_impl()
}

fn export_external_absorption_brief() -> Result<Value> {
    export_external_absorption_brief_impl()
}

fn export_priority_human_contracts() -> Result<Value> {
    export_priority_human_contracts_impl()
}

fn export_priority_human_crate_spawn_registry() -> Result<Value> {
    export_priority_human_crate_spawn_registry_impl()
}

fn export_crawl4ai_runtime_contract() -> Result<Value> {
    export_crawl4ai_runtime_contract_impl()
}

fn export_litellm_routing_contract() -> Result<Value> {
    export_litellm_routing_contract_impl()
}

fn export_llmfit_routing_contract() -> Result<Value> {
    export_llmfit_routing_contract_impl()
}

fn export_crate_spawn_contract() -> Result<Value> {
    export_crate_spawn_contract_impl()
}

fn export_opencode_project_runtime() -> Result<Value> {
    export_opencode_project_runtime_impl()
}

fn export_source_lesson_embodiment_registry() -> Result<Value> {
    export_source_lesson_embodiment_registry_impl()
}

fn export_source_lesson_embodiment_backlog() -> Result<Value> {
    export_source_lesson_embodiment_backlog_impl()
}

fn export_hermes_community_sources() -> Result<Value> {
    export_hermes_community_sources_impl()
}

fn export_multi_domain_routing_contract() -> Result<Value> {
    export_multi_domain_routing_contract_impl()
}

fn export_community_signal_intake() -> Result<Value> {
    export_community_signal_intake_impl()
}

fn export_research_workflow_contract() -> Result<Value> {
    export_research_workflow_contract_impl()
}

fn export_socratic_validator_contract() -> Result<Value> {
    export_socratic_validator_contract_impl()
}

fn export_autonomy_resume() -> Result<Value> {
    export_autonomy_resume_impl()
}

fn export_autonomy_task_truth() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/autonomy_task_truth.json");
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let queue_summary_path = root.join("core/state/queue_summary.json");
    let queue_hygiene_path = root.join("core/state/queue_hygiene.json");
    let autonomy_resume_path = root.join("core/state/autonomy_resume.json");
    let flywheel_path = root.join("core/state/flywheel_packet_runtime.json");
    let queue_federation_path = root.join("core/state/queue_federation.json");
    let preflight_path = root.join("data/prometheus/autonomy_operating_loop_preflight.json");
    let plan_map_path = root.join("core/state/plan_map.json");

    let raw_tasks = read_jsonl_objects(&queue_path);
    let tasks = latest_project_tasks(&raw_tasks);
    let queue_hygiene = write_queue_hygiene_projection(&root, &raw_tasks, &tasks)?;
    write_compact_queue_summary_projection(&root, &raw_tasks, &tasks)?;
    write_queue_active_projection(&root, &raw_tasks, &tasks)?;
    let queue_federation = write_queue_federation_projection(&root)?;
    let queue_summary = read_json_or(&queue_summary_path, json!({}));
    let autonomy_resume = read_json_or(&autonomy_resume_path, json!({}));
    let flywheel = read_json_or(&flywheel_path, json!({}));
    let preflight = read_json_or(&preflight_path, json!({}));
    let plan_map = read_json_or(&plan_map_path, json!({}));

    let latest_counts = task_status_counts(&tasks);
    let open_tasks = tasks
        .iter()
        .filter(|task| is_open_task(task))
        .cloned()
        .collect::<Vec<_>>();
    let compact_open = open_tasks
        .iter()
        .take(16)
        .map(compact_project_task)
        .collect::<Vec<_>>();

    let summary_counts = queue_summary
        .get("project_tasks")
        .and_then(|value| value.get("counts_by_status"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let summary_has_reading_policy = queue_summary.get("agent_reading_policy").is_some();
    let summary_has_compact = queue_summary
        .get("project_tasks")
        .is_some_and(|project_tasks| project_tasks.get("open_compact").is_some());

    let flywheel_packet_total = flywheel
        .get("summary")
        .and_then(|summary| summary.get("packet_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let preflight_ready_for_mutation = preflight
        .get("summary")
        .and_then(|summary| summary.get("ready_for_mutation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let preflight_safe_local_ready = preflight
        .get("summary")
        .and_then(|summary| summary.get("safe_local_ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mutation_hold_reason = preflight
        .get("summary")
        .and_then(|summary| summary.get("mutation_hold_reason"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let broken_markdown_link_count = preflight
        .get("summary")
        .and_then(|summary| summary.get("broken_markdown_link_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let review_candidate_total = preflight
        .get("summary")
        .map(|summary| {
            summary
                .get("review_archive_candidate_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + summary
                    .get("review_unreferenced_candidate_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
        })
        .unwrap_or(0);

    let mut warnings = Vec::new();
    if !summary_has_reading_policy {
        warnings.push(json!({
            "id": "queue_summary_missing_agent_reading_policy",
            "severity": "warn",
            "message": "queue_summary.json is still in the older projection shape; agents can fall back to this reconciliation surface."
        }));
    }
    if !summary_has_compact {
        warnings.push(json!({
            "id": "queue_summary_missing_compact_views",
            "severity": "warn",
            "message": "queue_summary.json does not expose open_compact/recent_compact yet."
        }));
    }
    if summary_counts != Value::Object(latest_counts.clone()) {
        warnings.push(json!({
            "id": "queue_summary_count_mismatch",
            "severity": "warn",
            "message": "queue_summary counts do not match latest-by-id queue ledger counts."
        }));
    }
    if flywheel_packet_total == 0 {
        warnings.push(json!({
            "id": "flywheel_packet_runtime_empty",
            "severity": "info",
            "message": "No active Flywheel work packets are projected."
        }));
    }
    if !preflight_safe_local_ready {
        warnings.push(json!({
            "id": "autonomy_preflight_not_ready_for_safe_local",
            "severity": "hold",
            "message": "Autonomy operating loop preflight is not ready for safe-local packet selection."
        }));
    }
    if !preflight_ready_for_mutation {
        warnings.push(json!({
            "id": "autonomy_preflight_mutation_hold",
            "severity": "info",
            "message": format!("Mutation remains held: {mutation_hold_reason}.")
        }));
    }

    let recommendation =
        if !preflight_safe_local_ready || !summary_has_reading_policy || !summary_has_compact {
            "hold_expansion_and_reconcile_task_truth"
        } else if open_tasks.is_empty() {
            "safe_local_packet_selection_can_resume"
        } else {
            "safe_local_packet_selection_has_open_backlog"
        };

    let payload = json!({
        "schema_version": "annunimas.autonomy-task-truth.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export autonomy-task-truth",
        "mutation_policy": "read_only_reconciliation",
        "recommendation": recommendation,
        "sources": {
            "queue_ledger": rel(&queue_path, &root),
            "queue_summary": rel(&queue_summary_path, &root),
            "queue_hygiene": rel(&queue_hygiene_path, &root),
            "queue_federation": rel(&queue_federation_path, &root),
            "autonomy_resume": rel(&autonomy_resume_path, &root),
            "flywheel_packet_runtime": rel(&flywheel_path, &root),
            "preflight": rel(&preflight_path, &root),
            "plan_map": rel(&plan_map_path, &root)
        },
        "queue_ledger_latest_by_id": {
            "total": tasks.len(),
            "counts_by_status": Value::Object(latest_counts),
            "open_total": open_tasks.len(),
            "open_compact": compact_open
        },
        "queue_summary_projection": {
            "generated_at_utc": queue_summary.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "has_agent_reading_policy": summary_has_reading_policy,
            "has_compact_views": summary_has_compact,
            "counts_by_status": summary_counts
        },
        "queue_hygiene_projection": {
            "generated_at_utc": queue_hygiene.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "raw_ledger_rows_total": queue_hygiene.get("metrics").and_then(|metrics| metrics.get("raw_ledger_rows_total")).cloned().unwrap_or(Value::Null),
            "latest_task_ids_total": queue_hygiene.get("metrics").and_then(|metrics| metrics.get("latest_task_ids_total")).cloned().unwrap_or(Value::Null),
            "latest_open_total": queue_hygiene.get("metrics").and_then(|metrics| metrics.get("latest_open_total")).cloned().unwrap_or(Value::Null),
            "stale_raw_queued_rows_total": queue_hygiene.get("metrics").and_then(|metrics| metrics.get("stale_raw_queued_rows_total")).cloned().unwrap_or(Value::Null)
        },
        "queue_federation_projection": {
            "generated_at_utc": queue_federation.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "summary": queue_federation.get("summary").cloned().unwrap_or(Value::Null),
            "promotion_policy": queue_federation.get("promotion_policy").cloned().unwrap_or(Value::Null),
            "control": queue_federation.get("control").cloned().unwrap_or_else(|| {
                queue_federation_control_projection(&queue_federation, &flywheel)
            }),
            "next_promotion_candidate": queue_federation
                .get("promotion_candidates")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|candidate| candidate.get("promotion_ready").and_then(Value::as_bool) == Some(true))
                .cloned()
                .unwrap_or(Value::Null)
        },
        "autonomy_resume_projection": {
            "generated_at_utc": autonomy_resume.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "queued_tasks_top_total": autonomy_resume
                .get("machine_truth")
                .and_then(|truth| truth.get("queued_tasks_top"))
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "flywheel_packet_summary": autonomy_resume
                .get("machine_truth")
                .and_then(|truth| truth.get("flywheel_packet_summary"))
                .cloned()
                .unwrap_or(Value::Null)
        },
        "flywheel_packet_runtime": {
            "generated_at_utc": flywheel.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "summary": flywheel.get("summary").cloned().unwrap_or(Value::Null),
            "next_ready_packet": flywheel
                .get("packets")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|packet| packet.get("readiness").and_then(Value::as_str) == Some("ready"))
                .cloned()
                .unwrap_or(Value::Null)
        },
        "preflight": {
            "generated_at_utc": preflight.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "safe_local_ready": preflight_safe_local_ready,
            "ready_for_mutation": preflight_ready_for_mutation,
            "mutation_hold_reason": mutation_hold_reason,
            "broken_markdown_link_count": broken_markdown_link_count,
            "review_candidate_total": review_candidate_total
        },
        "plan_map": {
            "generated_at_utc": plan_map.get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "summary": plan_map.get("summary").cloned().unwrap_or(Value::Null)
        },
        "warnings": warnings,
        "next_actions": [
            "keep autonomy expansion held until task truth surfaces agree",
            "refresh queue_summary projection through the updated source path",
            "regenerate only active Flywheel work packets from the canonical autonomy reconciliation plan",
            "route HADES link/archive findings into review packets before any cleanup mutation"
        ]
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root), "recommendation": recommendation }))
}

fn export_queue_hygiene() -> Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let raw_tasks = read_jsonl_objects(&queue_path);
    let tasks = latest_project_tasks(&raw_tasks);
    let payload = write_queue_hygiene_projection(&root, &raw_tasks, &tasks)?;
    write_compact_queue_summary_projection(&root, &raw_tasks, &tasks)?;
    let queue_active = write_queue_active_projection(&root, &raw_tasks, &tasks)?;
    let queue_federation = write_queue_federation_projection(&root)?;
    Ok(json!({
        "out": "core/state/queue_hygiene.json",
        "queue_active_out": "core/state/queue_active.json",
        "queue_federation_out": "core/state/queue_federation.json",
        "latest_open_total": payload.get("metrics").and_then(|metrics| metrics.get("latest_open_total")).cloned().unwrap_or(Value::Null),
        "queue_active_total": queue_active.get("active_task_count").cloned().unwrap_or(Value::Null),
        "promotion_candidates_total": queue_federation.pointer("/summary/promotion_candidates_total").cloned().unwrap_or(Value::Null),
        "stale_raw_queued_rows_total": payload.get("metrics").and_then(|metrics| metrics.get("stale_raw_queued_rows_total")).cloned().unwrap_or(Value::Null)
    }))
}

fn export_queue_active() -> Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let raw_tasks = read_jsonl_objects(&queue_path);
    let tasks = latest_project_tasks(&raw_tasks);
    let payload = write_queue_active_projection(&root, &raw_tasks, &tasks)?;
    Ok(json!({
        "out": "core/state/queue_active.json",
        "active_task_count": payload.get("active_task_count").cloned().unwrap_or(Value::Null)
    }))
}

fn export_queue_federation() -> Result<Value> {
    let root = workspace_root();
    let payload = write_queue_federation_projection(&root)?;
    Ok(json!({
        "out": "core/state/queue_federation.json",
        "sources_total": payload.pointer("/summary/sources_total").cloned().unwrap_or(Value::Null),
        "promotion_candidates_total": payload.pointer("/summary/promotion_candidates_total").cloned().unwrap_or(Value::Null),
        "blocked_total": payload.pointer("/summary/blocked_total").cloned().unwrap_or(Value::Null)
    }))
}

fn write_queue_federation_projection(root: &Path) -> Result<Value> {
    let out_path = root.join("core/state/queue_federation.json");
    let canonical_queue_path = root.join("core/projects/tasks/queue.jsonl");
    let raw_tasks = read_jsonl_objects(&canonical_queue_path);
    let latest_tasks = latest_project_tasks(&raw_tasks);
    let active_tasks = latest_tasks
        .iter()
        .filter(|task| is_open_task(task))
        .map(compact_project_task)
        .collect::<Vec<_>>();

    let queue_sources = vec![
        queue_source_spec(
            "canonical_project_tasks",
            "prometheus",
            "canonical_task_queue",
            "core/projects/tasks/queue.jsonl",
            "canonical_task",
            "latest_by_id_execution_backlog",
            true,
        ),
        queue_source_spec(
            "runtime_queue",
            "prometheus",
            "runtime_queue",
            "core/queue/queue.jsonl",
            "runtime_message",
            "legacy_or_runtime_messages_not_canonical_tasks",
            false,
        ),
        queue_source_spec(
            "knowledge_actionable_review",
            "athena",
            "review_queue",
            "core/knowledge/actionable_review_queue.jsonl",
            "review_signal",
            "human_or_triad_review_required_before_task_mutation",
            false,
        ),
        queue_source_spec(
            "athena_policy_tasks",
            "athena",
            "subsystem_task_queue",
            "crates/annunimas-athena/core/projects/tasks/queue.jsonl",
            "promotion_candidate",
            "requires_prometheus_safe_local_task_promotion_receipt",
            false,
        ),
        queue_source_spec(
            "hades_action_queue",
            "hades",
            "action_queue",
            "data/hades/action_queue.jsonl",
            "review_signal",
            "lifecycle_action_queue_closeout_controls_completion",
            false,
        ),
        queue_source_spec(
            "hades_action_closeouts",
            "hades",
            "closeout_ledger",
            "data/hades/action_queue_closeouts.jsonl",
            "closeout",
            "append_only_closeouts_for_hades_action_queue",
            false,
        ),
        queue_source_spec(
            "warden_informant",
            "warden",
            "telemetry_queue",
            "data/warden/informant_queue.jsonl",
            "telemetry",
            "attention_required_records_may_become_review_packets",
            false,
        ),
        queue_source_spec(
            "hades_warden_handoff",
            "hades",
            "handoff_queue",
            "data/hades/warden_queue.jsonl",
            "review_signal",
            "warden_hades_review_packet_source_not_direct_task_mutation",
            false,
        ),
        queue_source_spec(
            "hades_athena_handoff",
            "hades",
            "handoff_queue",
            "data/hades/athena_handoff_queue.jsonl",
            "review_signal",
            "athena_handoff_evidence_not_direct_task_mutation",
            false,
        ),
        queue_source_spec(
            "hermes_outbound",
            "hermes",
            "delivery_queue",
            "data/hermes/outbound_queue.jsonl",
            "delivery",
            "outbound_delivery_status_not_execution_backlog",
            false,
        ),
        queue_source_spec(
            "arandur_queue_proposals",
            "arandur",
            "proposal_queue",
            "data/arandur/mission_queue_proposals.jsonl",
            "proposal",
            "requires_explicit_arandur_queue_operation_approval",
            false,
        ),
        queue_source_spec(
            "arandur_queue_write_requests",
            "arandur",
            "write_request_queue",
            "data/arandur/mission_queue_write_requests.jsonl",
            "promotion_candidate",
            "requires_annunimas_arandur_queue_operation_v1",
            false,
        ),
        queue_source_spec(
            "human_workspace",
            "human",
            "human_knowledge_lane",
            "human/",
            "human_signal",
            "human_notes_are_signal_sources_and_require_explicit_promotion",
            false,
        ),
        queue_source_spec(
            "learning_loop_v1_state",
            "prometheus",
            "learning_loop_state",
            "core/state/learning_loop_v1.json",
            "learning_loop_proposal",
            "oracle_warden_gate_before_prometheus_promotion_or_hades_packet",
            false,
        ),
        queue_source_spec(
            "flywheel_packet_runtime",
            "prometheus",
            "flywheel_packet_projection",
            "core/state/flywheel_packet_runtime.json",
            "flywheel_plan_packet",
            "readiness_projection_only_requires_flywheel_packet_receipt_before_queue_append",
            false,
        ),
        queue_source_spec(
            "ceo_autopilot_task_truth",
            "prometheus",
            "ceo_autopilot_reconciliation",
            "core/state/autonomy_task_truth.json",
            "ceo_autopilot_reconciliation",
            "read_only_truth_reconciliation_not_execution_authority",
            false,
        ),
        queue_source_spec(
            "ceo_autopilot_preflight",
            "prometheus",
            "ceo_autopilot_preflight",
            "data/prometheus/autonomy_operating_loop_preflight.json",
            "autonomy_preflight",
            "preflight_readiness_requires_action_class_receipt_before_mutation",
            false,
        ),
    ];

    let mut source_summaries = Vec::new();
    for source in &queue_sources {
        source_summaries.push(queue_source_summary(root, source));
    }

    let mut promotion_candidates = Vec::new();
    let promoted_source_record_ids = promoted_source_record_ids(&raw_tasks);
    promotion_candidates.extend(athena_policy_promotion_candidates(
        root,
        &promoted_source_record_ids,
    ));
    promotion_candidates.extend(arandur_write_promotion_candidates(root));
    promotion_candidates.extend(human_workspace_promotion_candidates(root));

    let blocked = promotion_candidates
        .iter()
        .filter(|candidate| candidate.get("promotion_ready").and_then(Value::as_bool) != Some(true))
        .count();
    let ready = promotion_candidates.len().saturating_sub(blocked);
    let validation = queue_federation_stage_validation(
        &raw_tasks,
        &latest_tasks,
        &source_summaries,
        &promotion_candidates,
    );
    let flywheel = read_json_or(
        &root.join("core/state/flywheel_packet_runtime.json"),
        json!({}),
    );
    let projected_promotion_candidates = promotion_candidates
        .iter()
        .take(64)
        .cloned()
        .collect::<Vec<_>>();
    let control_source = json!({
        "schema_version": "annunimas.queue-federation.v1",
        "promotion_candidates": projected_promotion_candidates
    });
    let control = queue_federation_control_projection(&control_source, &flywheel);

    let payload = json!({
        "schema_version": "annunimas.queue-federation.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export queue-federation",
        "mutation_policy": "read_only_projection_no_task_queue_mutation",
        "central_backlog": {
            "path": "core/projects/tasks/queue.jsonl",
            "role": "canonical latest-by-id execution backlog",
            "active_total": active_tasks.len(),
            "latest_task_ids_total": latest_tasks.len(),
            "raw_rows_total": raw_tasks.len()
        },
        "promotion_policy": {
            "default": "passive_projection_only",
            "safe_local_initial_scope": true,
            "allowed_contracts": [
                "flywheel_plan_packet",
                "prometheus_safe_local_task_promotion",
                "annunimas.arandur.queue_operation.v1"
            ],
            "blocked_without_explicit_human_or_triad_gate": [
                "credential_sensitive",
                "external_side_effect",
                "destructive_delete",
                "archive_or_retention",
                "funds_movement",
                "legal_commitment",
                "service_restart",
                "provider_reload",
                "fleet_reimage",
                "customer_commitment"
            ],
            "human_lane": {
                "path": "human/",
                "rule": "human notes, decisions, and task templates are visible as source signals but never auto-promoted without explicit human/promotion receipt"
            }
        },
        "summary": {
            "sources_total": source_summaries.len(),
            "promotion_candidates_total": promotion_candidates.len(),
            "promotion_ready_total": ready,
            "blocked_total": blocked
        },
        "validation": validation,
        "control": control,
        "sources": source_summaries,
        "active_canonical_tasks": active_tasks.into_iter().take(32).collect::<Vec<_>>(),
        "promotion_candidates": projected_promotion_candidates
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(payload)
}

fn queue_source_spec(
    id: &str,
    owner: &str,
    source_type: &str,
    path: &str,
    default_record_class: &str,
    feed_policy: &str,
    canonical: bool,
) -> Value {
    let lane_contract = queue_source_lane_contract(id, default_record_class);
    json!({
        "id": id,
        "owner": owner,
        "source_type": source_type,
        "path": path,
        "default_record_class": lane_contract.get("default_record_class").cloned().unwrap_or(json!("evidence")),
        "lane_subclass": lane_contract.get("lane_subclass").cloned().unwrap_or(json!(default_record_class)),
        "allowed_emits": lane_contract.get("allowed_emits").cloned().unwrap_or(json!(["evidence"])),
        "allowed_mutations": lane_contract.get("allowed_mutations").cloned().unwrap_or(json!([])),
        "promotion_receipt_required": lane_contract.get("promotion_receipt_required").cloned().unwrap_or(json!("explicit_promotion_receipt")),
        "human_gated": lane_contract.get("human_gated").cloned().unwrap_or(json!(false)),
        "feed_policy": feed_policy,
        "canonical": canonical
    })
}

fn queue_source_lane_contract(id: &str, lane_subclass: &str) -> Value {
    let (
        default_record_class,
        allowed_emits,
        allowed_mutations,
        promotion_receipt_required,
        human_gated,
    ) = match id {
        "canonical_project_tasks" => (
            "execution_attempt",
            vec!["execution_attempt", "terminal_result"],
            vec!["core/projects/tasks/queue.jsonl:append_only"],
            "none_already_canonical",
            false,
        ),
        "runtime_queue" => (
            "evidence",
            vec!["evidence"],
            vec![],
            "flywheel_plan_packet_or_prometheus_safe_local_task_promotion",
            false,
        ),
        "knowledge_actionable_review" => (
            "evidence",
            vec!["proposal", "evidence"],
            vec![],
            "human_or_triad_review_receipt",
            true,
        ),
        "athena_policy_tasks" => (
            "proposal",
            vec!["proposal", "evidence"],
            vec![],
            "prometheus_safe_local_task_promotion",
            false,
        ),
        "hades_action_queue" => (
            "approval",
            vec!["approval", "execution_attempt", "terminal_result"],
            vec!["data/hades/action_queue_closeouts.jsonl:append_only"],
            "hades_lifecycle_action_receipt",
            true,
        ),
        "hades_action_closeouts" => (
            "terminal_result",
            vec!["evidence", "terminal_result"],
            vec![],
            "result_evidence_receipt",
            false,
        ),
        "warden_informant" => (
            "evidence",
            vec!["evidence", "proposal"],
            vec![],
            "warden_review_packet_receipt",
            false,
        ),
        "hades_warden_handoff" => (
            "evidence",
            vec!["evidence", "proposal"],
            vec![],
            "warden_hades_review_packet_receipt",
            false,
        ),
        "hades_athena_handoff" => (
            "evidence",
            vec!["evidence", "proposal"],
            vec![],
            "athena_handoff_review_receipt",
            false,
        ),
        "hermes_outbound" => (
            "execution_attempt",
            vec!["evidence", "execution_attempt", "terminal_result"],
            vec!["data/hermes/outbound_queue.jsonl:delivery_attempts_only"],
            "external_side_effect_approval_receipt_for_non_delivery_promotion",
            true,
        ),
        "arandur_queue_proposals" => (
            "proposal",
            vec!["proposal", "approval"],
            vec![],
            "explicit_arandur_queue_operation_approval",
            true,
        ),
        "arandur_queue_write_requests" => (
            "approval",
            vec!["approval", "execution_attempt"],
            vec![],
            "annunimas.arandur.queue_operation.v1",
            true,
        ),
        "human_workspace" => (
            "evidence",
            vec!["proposal", "evidence", "approval"],
            vec![],
            "explicit_human_promotion_receipt",
            true,
        ),
        "learning_loop_v1_state" => (
            "proposal",
            vec!["proposal", "evidence"],
            vec![],
            "oracle_warden_gate_verdict_plus_prometheus_safe_local_task_promotion_or_hades_lifecycle_packet",
            true,
        ),
        "flywheel_packet_runtime" => (
            "proposal",
            vec!["proposal", "evidence", "approval"],
            vec![],
            "flywheel_plan_packet_readiness_receipt",
            false,
        ),
        "ceo_autopilot_task_truth" => (
            "evidence",
            vec!["evidence", "proposal"],
            vec![],
            "autonomy_task_truth_reconciliation_receipt",
            false,
        ),
        "ceo_autopilot_preflight" => (
            "evidence",
            vec!["evidence", "approval"],
            vec![],
            "bounded_action_class_preflight_receipt_and_human_gate_for_mutation",
            true,
        ),
        _ => (
            "evidence",
            vec!["evidence"],
            vec![],
            "explicit_promotion_receipt",
            true,
        ),
    };

    json!({
        "default_record_class": default_record_class,
        "lane_subclass": lane_subclass,
        "allowed_emits": allowed_emits,
        "allowed_mutations": allowed_mutations,
        "promotion_receipt_required": promotion_receipt_required,
        "human_gated": human_gated
    })
}

fn canonical_lane_record_class(lane_subclass: &str) -> &'static str {
    match lane_subclass {
        "proposal" | "promotion_candidate" => "proposal",
        "evidence" | "canonical_task" | "runtime_message" | "review_signal" | "telemetry"
        | "human_signal" | "already_promoted" => "evidence",
        "approval" | "write_request" => "approval",
        "execution_attempt" | "delivery" => "execution_attempt",
        "terminal_result" | "closeout" => "terminal_result",
        _ => "evidence",
    }
}

fn is_canonical_lane_record_class(record_class: &str) -> bool {
    matches!(
        record_class,
        "proposal" | "evidence" | "approval" | "execution_attempt" | "terminal_result"
    )
}

fn is_terminal_lane_status(status: &str) -> bool {
    matches!(
        normalize_task_status(status),
        "completed" | "blocked" | "cancelled" | "failed"
    )
}

fn has_receipt_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Bool(value) => *value,
        Value::Number(_) => true,
    }
}

fn has_any_receipt_path(record: &Value, paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|path| record.pointer(path).is_some_and(has_receipt_value))
}

fn queue_federation_control_projection(queue_federation: &Value, flywheel: &Value) -> Value {
    let promotion_candidates = queue_federation
        .get("promotion_candidates")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let flywheel_packets = flywheel
        .get("packets")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut control_packets = Vec::new();
    let mut rejected = Vec::new();

    for candidate in promotion_candidates {
        let promotion_ready =
            candidate.get("promotion_ready").and_then(Value::as_bool) == Some(true);
        let risk_lane = candidate
            .get("risk_lane")
            .and_then(Value::as_str)
            .unwrap_or("");
        let human_gated = risk_lane == "human_review_required"
            || candidate.get("human_gated").and_then(Value::as_bool) == Some(true);
        let has_promotion_receipt = has_any_receipt_path(
            candidate,
            &[
                "/promotion_receipt",
                "/receipt",
                "/receipt_surface",
                "/provenance/promotion_receipt",
                "/provenance/receipt",
                "/provenance/receipt_surface",
            ],
        );
        let source_record_id = candidate
            .get("source_record_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let matching_flywheel_packet = flywheel_packets.iter().find(|packet| {
            packet
                .get("packet_id")
                .or_else(|| packet.get("task_id"))
                .and_then(Value::as_str)
                == Some(source_record_id)
        });
        let flywheel_ready = matching_flywheel_packet
            .and_then(|packet| packet.get("readiness").and_then(Value::as_str))
            == Some("ready");
        let safe_local = risk_lane == "safe-local_candidate_unverified"
            || matching_flywheel_packet
                .and_then(|packet| packet.get("risk").and_then(Value::as_str))
                == Some("safe-local");

        if promotion_ready && safe_local && !human_gated && has_promotion_receipt && flywheel_ready
        {
            control_packets.push(json!({
                "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                "record_class": candidate.get("record_class").cloned().unwrap_or(json!("proposal")),
                "lane_subclass": candidate.get("lane_subclass").cloned().unwrap_or(json!("promotion_candidate")),
                "promotion_receipt_required": candidate.get("required_contract").cloned().unwrap_or(json!("explicit_promotion_receipt")),
                "promotion_receipt": candidate.get("promotion_receipt").cloned().unwrap_or(Value::Null),
                "risk_lane": candidate.get("risk_lane").cloned().unwrap_or(Value::Null),
                "flywheel_packet": matching_flywheel_packet.cloned().unwrap_or(Value::Null)
            }));
        } else if promotion_ready {
            rejected.push(json!({
                "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                "record_class": candidate.get("record_class").cloned().unwrap_or(Value::Null),
                "lane_subclass": candidate.get("lane_subclass").cloned().unwrap_or(Value::Null),
                "reason": if human_gated {
                    "human_gated"
                } else if !has_promotion_receipt {
                    "missing_promotion_receipt"
                } else if !safe_local {
                    "not_safe_local"
                } else if !flywheel_ready {
                    "flywheel_packet_not_ready"
                } else {
                    "selector_guard"
                }
            }));
        }
    }

    let next_safe_local_candidate = control_packets.first().cloned().unwrap_or(Value::Null);

    json!({
        "schema_version": "annunimas.queue-federation-control.v1",
        "projection_cache": {
            "source_paths": [
                "core/state/queue_federation.json",
                "core/state/flywheel_packet_runtime.json"
            ],
            "source_schema_versions": {
                "queue_federation": queue_federation.get("schema_version").cloned().unwrap_or(Value::Null),
                "flywheel_packet_runtime": flywheel.get("schema_version").cloned().unwrap_or(Value::Null)
            }
        },
        "selector": {
            "mode": "safe_local_receipt_valid_only",
            "next_safe_local_candidate": next_safe_local_candidate,
            "selected_total": control_packets.len(),
            "rejected_total": rejected.len(),
            "rejected": rejected.into_iter().take(16).collect::<Vec<_>>()
        },
        "control_packets": control_packets.into_iter().take(16).collect::<Vec<_>>()
    })
}

fn has_terminal_result_evidence(record: &Value) -> bool {
    has_any_receipt_path(
        record,
        &[
            "/result",
            "/blocked_reason",
            "/error",
            "/verification",
            "/evidence",
            "/receipts",
            "/artifacts",
            "/meta/result",
            "/meta/result_evidence",
            "/meta/blocker",
            "/meta/receipt",
            "/meta/receipt_surface",
            "/meta/verification",
            "/meta/verify",
            "/meta/artifact",
            "/meta/artifacts",
            "/meta/log",
            "/meta/tests",
        ],
    )
}

fn queue_federation_stage_validation(
    raw_tasks: &[Value],
    latest_tasks: &[Value],
    source_summaries: &[Value],
    promotion_candidates: &[Value],
) -> Value {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    for source in source_summaries {
        let source_id = source.get("id").cloned().unwrap_or(Value::Null);
        if let Some(record_class) = source.get("default_record_class").and_then(Value::as_str) {
            if !is_canonical_lane_record_class(record_class) {
                errors.push(json!({
                    "id": "invalid_source_record_class",
                    "severity": "error",
                    "source_id": source_id,
                    "record_class": record_class,
                    "message": "source default_record_class is outside the lane-stage contract vocabulary"
                }));
            }
        } else {
            errors.push(json!({
                "id": "source_missing_record_class",
                "severity": "error",
                "source_id": source_id,
                "message": "source summary is missing default_record_class"
            }));
        }

        if let Some(allowed_emits) = source.get("allowed_emits").and_then(Value::as_array) {
            for allowed in allowed_emits {
                let Some(record_class) = allowed.as_str() else {
                    continue;
                };
                if !is_canonical_lane_record_class(record_class) {
                    errors.push(json!({
                        "id": "invalid_allowed_emit_record_class",
                        "severity": "error",
                        "source_id": source_id,
                        "record_class": record_class,
                        "message": "allowed_emits entry is outside the lane-stage contract vocabulary"
                    }));
                }
            }
        }
    }

    for task in latest_tasks {
        let status = task.get("status").and_then(Value::as_str).unwrap_or("");
        if is_terminal_lane_status(status) && !has_terminal_result_evidence(task) {
            errors.push(json!({
                "id": "central_terminal_result_missing_evidence",
                "severity": "error",
                "task_id": task.get("id").cloned().unwrap_or(Value::Null),
                "status": status,
                "message": "canonical queue terminal record is missing result, blocker, rollback/no-op, artifact, log, or test receipt evidence"
            }));
        }
    }

    for record in raw_tasks {
        let status = record.get("status").and_then(Value::as_str).unwrap_or("");
        let origin = record
            .pointer("/meta/origin")
            .and_then(Value::as_str)
            .unwrap_or("");
        if origin == "queue_federation_promotion"
            && is_terminal_lane_status(status)
            && !has_any_receipt_path(
                record,
                &[
                    "/meta/source_record_id",
                    "/meta/source_queue",
                    "/meta/promotion_receipt",
                    "/meta/receipt",
                    "/meta/receipt_surface",
                ],
            )
        {
            warnings.push(json!({
                "id": "federation_promotion_missing_source_receipt",
                "severity": "warn",
                "task_id": record.get("id").cloned().unwrap_or(Value::Null),
                "message": "queue_federation_promotion record should bind source queue, source record id, and promotion receipt before terminal closeout"
            }));
        }
    }

    for candidate in promotion_candidates {
        let record_class = candidate
            .get("record_class")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !is_canonical_lane_record_class(record_class) {
            errors.push(json!({
                "id": "invalid_candidate_record_class",
                "severity": "error",
                "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                "record_class": record_class,
                "message": "promotion candidate record_class is outside the lane-stage contract vocabulary"
            }));
        }

        let promotion_ready =
            candidate.get("promotion_ready").and_then(Value::as_bool) == Some(true);
        let has_required_contract = candidate
            .get("required_contract")
            .is_some_and(has_receipt_value);
        let has_source_record_id = candidate
            .get("source_record_id")
            .is_some_and(has_receipt_value);
        let has_risk_lane = candidate.get("risk_lane").is_some_and(has_receipt_value);

        if record_class == "proposal"
            && promotion_ready
            && !(has_required_contract && has_source_record_id && has_risk_lane)
        {
            warnings.push(json!({
                "id": "proposal_ready_missing_promotion_receipt_fields",
                "severity": "warn",
                "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                "message": "proposal is promotion_ready without source id, risk lane, and required contract receipt fields"
            }));
        }

        if record_class == "evidence" && promotion_ready {
            warnings.push(json!({
                "id": "evidence_promoted_without_receipt",
                "severity": "warn",
                "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                "message": "evidence records may support promotion but must not become execution work without an explicit binding receipt"
            }));
        }

        if record_class == "approval" {
            let provenance = candidate.get("provenance").unwrap_or(&Value::Null);
            let has_authority = candidate.get("owner").is_some_and(has_receipt_value)
                || has_any_receipt_path(provenance, &["/authority", "/approved_by"]);
            let has_scope = candidate.get("scope").is_some_and(has_receipt_value)
                || has_any_receipt_path(provenance, &["/scope", "/requested_queue_entry/scope"]);
            let has_target = has_source_record_id
                || has_any_receipt_path(provenance, &["/target", "/requested_queue_entry/id"]);
            let has_action_class = candidate
                .get("bounded_action_class")
                .is_some_and(has_receipt_value)
                || has_any_receipt_path(
                    provenance,
                    &["/bounded_action_class", "/action_class", "/phase"],
                );
            if !(has_authority && has_scope && has_target && has_action_class) {
                warnings.push(json!({
                    "id": "approval_missing_scope_receipt_fields",
                    "severity": "warn",
                    "source_queue": candidate.get("source_queue").cloned().unwrap_or(Value::Null),
                    "source_record_id": candidate.get("source_record_id").cloned().unwrap_or(Value::Null),
                    "message": "approval records should name authority, target, scope, and bounded action class before execution"
                }));
            }
        }
    }

    json!({
        "status": if errors.is_empty() { "clean" } else { "error" },
        "mode": "lightweight_stage_transition_warnings",
        "errors_total": errors.len(),
        "warnings_total": warnings.len(),
        "checks": [
            "canonical_record_class_vocabulary",
            "proposal_ready_requires_source_risk_contract_receipt",
            "evidence_not_promotion_ready_without_binding_receipt",
            "approval_requires_authority_target_scope_action_class",
            "central_terminal_result_requires_result_evidence"
        ],
        "errors": errors.into_iter().take(32).collect::<Vec<_>>(),
        "warnings": warnings.into_iter().take(64).collect::<Vec<_>>()
    })
}

fn queue_source_summary(root: &Path, source: &Value) -> Value {
    let rel_path = source.get("path").and_then(Value::as_str).unwrap_or("");
    let path = root.join(rel_path);
    let is_dir = path.is_dir();
    let records = if path.is_file() {
        read_jsonl_objects(&path)
    } else {
        Vec::new()
    };
    let status_counts = if records.is_empty() {
        json!({})
    } else {
        count_status_like(&records)
    };
    let newest = records
        .iter()
        .filter_map(record_timestamp)
        .max()
        .unwrap_or_else(|| {
            fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .map(|mtime| chrono::DateTime::<Utc>::from(mtime).to_rfc3339())
                .unwrap_or_default()
        });
    json!({
        "id": source.get("id").cloned().unwrap_or(Value::Null),
        "owner": source.get("owner").cloned().unwrap_or(Value::Null),
        "source_type": source.get("source_type").cloned().unwrap_or(Value::Null),
        "path": rel_path,
        "exists": path.exists(),
        "is_directory": is_dir,
        "record_count": records.len(),
        "newest_observed_at_utc": if newest.is_empty() { Value::Null } else { json!(newest) },
        "status_counts": status_counts,
        "default_record_class": source.get("default_record_class").cloned().unwrap_or(Value::Null),
        "lane_subclass": source.get("lane_subclass").cloned().unwrap_or(Value::Null),
        "allowed_emits": source.get("allowed_emits").cloned().unwrap_or(json!([])),
        "allowed_mutations": source.get("allowed_mutations").cloned().unwrap_or(json!([])),
        "promotion_receipt_required": source.get("promotion_receipt_required").cloned().unwrap_or(Value::Null),
        "human_gated": source.get("human_gated").cloned().unwrap_or(json!(false)),
        "feed_policy": source.get("feed_policy").cloned().unwrap_or(Value::Null),
        "canonical": source.get("canonical").cloned().unwrap_or(json!(false))
    })
}

fn count_status_like(records: &[Value]) -> Value {
    let mut counts = serde_json::Map::new();
    for record in records {
        let key = record
            .get("status")
            .or_else(|| record.get("state"))
            .or_else(|| record.get("decision"))
            .or_else(|| record.get("event_type"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        *counts.entry(key.to_string()).or_insert(json!(0)) =
            json!(counts.get(key).and_then(Value::as_u64).unwrap_or(0) + 1);
    }
    Value::Object(counts)
}

fn record_timestamp(record: &Value) -> Option<String> {
    [
        "queued_at_utc",
        "created_at_utc",
        "triaged_at_utc",
        "generated_at_utc",
        "completed_at_utc",
        "closed_at_utc",
        "ts_utc",
        "ts",
        "timestamp",
    ]
    .iter()
    .find_map(|field| {
        record
            .get(*field)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn promoted_source_record_ids(raw_tasks: &[Value]) -> BTreeSet<String> {
    raw_tasks
        .iter()
        .filter(|record| {
            record
                .pointer("/meta/origin")
                .and_then(Value::as_str)
                .is_some_and(|origin| origin == "queue_federation_promotion")
        })
        .filter_map(|record| {
            record
                .pointer("/meta/source_record_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

fn athena_policy_promotion_candidates(
    root: &Path,
    promoted_source_record_ids: &BTreeSet<String>,
) -> Vec<Value> {
    let path = root.join("crates/annunimas-athena/core/projects/tasks/queue.jsonl");
    read_jsonl_objects(&path)
        .into_iter()
        .filter(|record| normalize_task_status(record.get("status").and_then(Value::as_str).unwrap_or("")) == "queued")
        .take(16)
        .map(|record| {
            let source_record_id = record.get("id").and_then(Value::as_str).unwrap_or("");
            let source_id = record.pointer("/meta/source_id").and_then(Value::as_str).unwrap_or("unknown");
            let blocker = record.pointer("/meta/blocker").and_then(Value::as_str);
            let already_promoted = promoted_source_record_ids.contains(source_record_id);
            let lane_subclass = if already_promoted {
                "already_promoted"
            } else {
                "promotion_candidate"
            };
            json!({
                "source_queue": "crates/annunimas-athena/core/projects/tasks/queue.jsonl",
                "source_record_id": record.get("id").cloned().unwrap_or(Value::Null),
                "record_class": canonical_lane_record_class(lane_subclass),
                "lane_subclass": lane_subclass,
                "promotion_ready": blocker.is_none() && !already_promoted,
                "blocked_reason": if already_promoted { "already_promoted_to_canonical_queue" } else { blocker.unwrap_or("requires_prometheus_safe_local_task_promotion_receipt") },
                "required_contract": "prometheus_safe_local_task_promotion",
                "risk_lane": "safe-local_candidate_unverified",
                "owner": record.get("owner").cloned().unwrap_or(json!("athena")),
                "title": record.get("title").cloned().unwrap_or(json!("ATHENA policy task")),
                "source_id": source_id,
                "provenance": record.get("meta").cloned().unwrap_or(json!({}))
            })
        })
        .collect()
}

fn arandur_write_promotion_candidates(root: &Path) -> Vec<Value> {
    let path = root.join("data/arandur/mission_queue_write_requests.jsonl");
    read_jsonl_objects(&path)
        .into_iter()
        .take(16)
        .map(|record| {
            json!({
                "source_queue": "data/arandur/mission_queue_write_requests.jsonl",
                "source_record_id": record.get("queue_write_request_id").cloned().unwrap_or(Value::Null),
                "record_class": "approval",
                "lane_subclass": "promotion_candidate",
                "promotion_ready": false,
                "blocked_reason": "requires_explicit_arandur_queue_operation_approval",
                "required_contract": "annunimas.arandur.queue_operation.v1",
                "risk_lane": "human_review_required",
                "owner": "arandur",
                "title": record.pointer("/requested_queue_entry/title").cloned().unwrap_or(json!("ARANDUR queue write request")),
                "provenance": {
                    "phase": record.get("phase").cloned().unwrap_or(Value::Null),
                    "source_queue_proposal_id": record.get("source_queue_proposal_id").cloned().unwrap_or(Value::Null),
                    "review_required": record.get("review_required").cloned().unwrap_or(json!(true))
                }
            })
        })
        .collect()
}

fn human_workspace_promotion_candidates(root: &Path) -> Vec<Value> {
    let candidates = [
        ("human/00-Templates/agent_task.md", "human_task_template"),
        (
            "human/05-Projects/summaries/task-notes.md",
            "human_task_notes",
        ),
    ];
    candidates
        .iter()
        .filter_map(|(path, kind)| {
            let full_path = root.join(path);
            if !full_path.exists() {
                return None;
            }
            Some(json!({
                "source_queue": "human/",
                "source_record_id": path,
                "record_class": "evidence",
                "lane_subclass": "human_signal",
                "promotion_ready": false,
                "blocked_reason": "requires_explicit_human_promotion_receipt",
                "required_contract": "prometheus_safe_local_task_promotion",
                "risk_lane": "human_review_required",
                "owner": "human",
                "title": format!("Human workspace signal: {kind}"),
                "provenance": {
                    "path": path,
                    "kind": kind
                }
            }))
        })
        .collect()
}

fn export_moria_repository_contract() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/moria_repository_contract.json");
    let contract_path = root.join("docs/contracts/moria-repository-mvp-contract.md");
    let moria_root =
        env::var("ANNUNIMAS_MORIA_ROOT").unwrap_or_else(|_| "/var/home/mythos/Moria".to_string());
    let payload = json!({
        "schema_version": "annunimas.moria-repository-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export moria-repository-contract",
        "contract": rel(&contract_path, &root),
        "status": "contract_defined_no_mutation",
        "root": moria_root,
        "allowed_source_classes": [
            "public_git_repository_with_license_metadata",
            "public_article_or_documentation_with_stable_url",
            "operator_provided_ingestion_note",
            "annunimas_receipt_referencing_external_source"
        ],
        "gates": {
            "license_gate_required": true,
            "prompt_injection_gate_required": true,
            "provenance_gate_required": true,
            "first_pass_writes_index_receipts_only": true
        },
        "planned_outputs": [
            "data/moria/index.jsonl",
            "data/moria/review_packets.jsonl"
        ],
        "mutation_policy": "no_source_rewrite_or_autonomous_clone"
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({"out": rel(&out_path, &root), "status": payload["status"]}))
}

fn export_athena_active_learning_health() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/athena_active_learning_health.json");
    let contract_path = root.join("docs/contracts/athena-active-learning-mvp-contract.md");
    let source_lane =
        read_jsonl_objects(&root.join("data/athena/external_source_lane_ledger.jsonl"));
    let uncertainty_selections_path = root.join("data/athena/uncertainty_selections.jsonl");
    let uncertainty_selection_receipts = read_jsonl_objects(&uncertainty_selections_path);
    let reviewed_total = source_lane.len();
    let promotion_allowed_total = source_lane
        .iter()
        .filter(|row| {
            row.get("task_promotion_allowed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let human_review_total = source_lane
        .iter()
        .filter(|row| {
            row.get("decision")
                .and_then(Value::as_str)
                .is_some_and(|decision| decision.contains("review") || decision.contains("gated"))
        })
        .count();
    let payload = json!({
        "schema_version": "annunimas.athena-active-learning-health.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export athena-active-learning-health",
        "contract": rel(&contract_path, &root),
        "status": "mvp_health_surface_defined",
        "metrics": {
            "reviewed_sources_total": reviewed_total,
            "promotion_allowed_total": promotion_allowed_total,
            "human_review_total": human_review_total,
            "coverage_gap_total": 0,
            "stale_or_low_confidence_total": 0,
            "uncertainty_selection_receipts_total": uncertainty_selection_receipts.len()
        },
        "receipt_surfaces": {
            "uncertainty_selections": rel(&uncertainty_selections_path, &root)
        },
        "metric_contract": [
            "confidence_score",
            "promotion_readiness",
            "coverage_gap",
            "staleness_score",
            "quality_flags"
        ],
        "safe_promotion_boundary": {
            "provenance_required": true,
            "quality_flags_must_not_be_high_severity": true,
            "receipt_required": true,
            "silent_memory_promotion_allowed": false
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({"out": rel(&out_path, &root), "status": payload["status"]}))
}

fn export_hermes_compression_credential_gate() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/hermes_compression_credential_gate.json");
    let contract_path = root.join("docs/contracts/hermes-compression-credential-freshness-gate.md");
    let queue_summary = read_json_or(&root.join("core/state/queue_summary.json"), json!({}));
    let open_tasks = queue_summary
        .get("project_tasks")
        .and_then(|tasks| tasks.get("open_compact"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let payload = json!({
        "schema_version": "annunimas.hermes-compression-credential-gate.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export hermes-compression-credential-gate",
        "contract": rel(&contract_path, &root),
        "status": "gate_contract_defined_no_secret_material",
        "gate_rule": {
            "fresh_provider_health_receipt_allows_route": true,
            "recent_charon_heartbeat_allows_route": true,
            "local_no_credential_route_allows_route": true,
            "otherwise_skip_provider": true
        },
        "receipt_fields": [
            "provider_id",
            "model_id",
            "credential_freshness_status",
            "last_successful_heartbeat_timestamp",
            "decision",
            "task_or_request_id"
        ],
        "decisions": [
            "allowed",
            "skipped_stale_credential",
            "local_no_credential"
        ],
        "constraints": {
            "write_secret_material": false,
            "automatic_secret_refresh": false,
            "bypass_charon_provider_health": false
        },
        "queue_context": {
            "open_tasks": open_tasks
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({"out": rel(&out_path, &root), "status": payload["status"]}))
}

fn write_queue_hygiene_projection(
    root: &Path,
    raw_tasks: &[Value],
    tasks: &[Value],
) -> Result<Value> {
    let out_path = root.join("core/state/queue_hygiene.json");
    let latest_by_id = tasks
        .iter()
        .filter_map(|task| {
            task.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), task))
        })
        .collect::<BTreeMap<_, _>>();
    let latest_open = tasks
        .iter()
        .filter(|task| is_open_task(task))
        .map(compact_project_task)
        .collect::<Vec<_>>();
    let stale_raw_queued = raw_tasks
        .iter()
        .filter(|task| {
            normalize_task_status(task.get("status").and_then(Value::as_str).unwrap_or(""))
                == "queued"
                && task
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| latest_by_id.get(id))
                    .is_some_and(|latest| !is_open_task(latest))
        })
        .map(compact_project_task)
        .collect::<Vec<_>>();
    let raw_queued_total = raw_tasks
        .iter()
        .filter(|task| {
            normalize_task_status(task.get("status").and_then(Value::as_str).unwrap_or(""))
                == "queued"
        })
        .count();
    let payload = json!({
        "schema_version": "annunimas.queue-hygiene.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export queue-hygiene",
        "doctrine": {
            "raw_queue_is_append_only_evidence": true,
            "operational_queue_counts_use_latest_state_by_id": true,
            "stale_raw_queued_rows_are_not_active_backlog": true,
            "closeouts_append_same_id_terminal_records": true
        },
        "metrics": {
            "raw_ledger_rows_total": raw_tasks.len(),
            "latest_task_ids_total": tasks.len(),
            "history_rows_total": raw_tasks.len().saturating_sub(tasks.len()),
            "raw_queued_rows_total": raw_queued_total,
            "latest_open_total": latest_open.len(),
            "stale_raw_queued_rows_total": stale_raw_queued.len(),
            "inflation_ratio_raw_queued_to_latest_open": if latest_open.is_empty() {
                Value::Null
            } else {
                json!(raw_queued_total as f64 / latest_open.len() as f64)
            }
        },
        "counts": {
            "raw_by_status": Value::Object(task_status_counts(raw_tasks)),
            "latest_by_status": Value::Object(task_status_counts(tasks))
        },
        "latest_open_compact": latest_open,
        "stale_raw_queued_compact": stale_raw_queued.into_iter().take(32).collect::<Vec<_>>(),
        "workflow": {
            "agent_default_read": "core/state/queue_summary.json",
            "hygiene_receipt": "core/state/queue_hygiene.json",
            "truth_receipt": "core/state/autonomy_task_truth.json",
            "mutation_target": "core/projects/tasks/queue.jsonl",
            "rules": [
                "Read queue_summary or queue_hygiene before raw queue.jsonl.",
                "Treat raw queued rows with later terminal same-id rows as historical evidence, not backlog.",
                "Use task-pivot or append a same-id terminal record for closeouts.",
                "Run export queue-hygiene and export autonomy-task-truth after backlog mutations."
            ]
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(payload)
}

fn write_queue_active_projection(
    root: &Path,
    raw_tasks: &[Value],
    tasks: &[Value],
) -> Result<Value> {
    let out_path = root.join("core/state/queue_active.json");
    let mut active_tasks = tasks
        .iter()
        .filter(|task| is_open_task(task))
        .map(compact_project_task)
        .collect::<Vec<_>>();
    active_tasks.sort_by(|a, b| {
        let ap = priority_rank(a.get("priority").and_then(Value::as_str).unwrap_or(""));
        let bp = priority_rank(b.get("priority").and_then(Value::as_str).unwrap_or(""));
        bp.cmp(&ap).then_with(|| {
            a.get("queued_at_utc")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(b.get("queued_at_utc").and_then(Value::as_str).unwrap_or(""))
        })
    });

    let payload = json!({
        "schema_version": "annunimas.queue_active.v1",
        "generated_at_utc": now_utc(),
        "authority": "queue_active_projection",
        "source": "core/projects/tasks/queue.jsonl",
        "mutation_policy": "read_only_latest_by_id_projection_no_queue_compaction",
        "raw_ledger_rows_total": raw_tasks.len(),
        "latest_task_ids_total": tasks.len(),
        "active_task_count": active_tasks.len(),
        "agent_reading_policy": {
            "default_surface": "core/state/queue_active.json",
            "fallback_surface": "core/state/queue_summary.json",
            "hygiene_surface": "core/state/queue_hygiene.json",
            "raw_queue_policy": "Do not read core/projects/tasks/queue.jsonl for task discovery. Use this compact active projection first; open the raw queue only for exact id evidence or appending a task record."
        },
        "tasks": active_tasks
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(payload)
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn export_governance_priority_runtime() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/governance_priority_runtime.json");
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let queue_hygiene_path = root.join("core/state/queue_hygiene.json");
    let pressure_guard_path = root.join("data/prometheus/pressure_guard_last.json");
    let governance_runtime_path = root.join("core/state/governance_runtime.json");
    let repeated_audit_path = root.join("core/state/repeated_audit_status.json");

    let raw_tasks = read_jsonl_objects(&queue_path);
    let tasks = latest_project_tasks(&raw_tasks);
    let queue_hygiene = write_queue_hygiene_projection(&root, &raw_tasks, &tasks)?;
    write_compact_queue_summary_projection(&root, &raw_tasks, &tasks)?;
    write_queue_active_projection(&root, &raw_tasks, &tasks)?;

    let pressure_guard = read_json_or(&pressure_guard_path, json!({}));
    let governance_runtime = read_json_or(&governance_runtime_path, json!({}));
    let repeated_audit = read_json_or(&repeated_audit_path, json!({}));
    let governance_signals = governance_runtime
        .get("signals")
        .or_else(|| governance_runtime.pointer("/contracts/active_ruleset/policy/validators/core"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let latest_open_total = queue_hygiene
        .pointer("/metrics/latest_open_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale_raw_queued_rows_total = queue_hygiene
        .pointer("/metrics/stale_raw_queued_rows_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let raw_queued_rows_total = queue_hygiene
        .pointer("/metrics/raw_queued_rows_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let pressure_status = pressure_guard
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let pressure_violation_total = pressure_guard
        .get("violations")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let oversize_total = pressure_guard
        .pointer("/observed/oversize_files_gte_100mb")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let audit_status = repeated_audit
        .get("gate_status")
        .or_else(|| repeated_audit.get("status"))
        .or_else(|| repeated_audit.pointer("/summary/status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let triad_signal = governance_runtime
        .pointer("/signals/triad_pass_rate")
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let love_signal = governance_runtime
        .pointer("/signals/avg_love_eq")
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let joule_signal = governance_runtime
        .pointer("/signals/avg_joulework")
        .and_then(Value::as_f64)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let governance_score =
        ((triad_signal * 0.40) + (love_signal * 0.30) + (joule_signal * 0.30)).clamp(0.0, 1.0);

    let mut ranked = tasks
        .iter()
        .filter(|task| is_open_task(task))
        .map(|task| {
            let priority = task
                .get("priority")
                .and_then(Value::as_str)
                .unwrap_or("medium");
            let title = task
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let scope = task
                .get("meta")
                .and_then(|meta| meta.get("scope"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let owner = task
                .get("owner")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let base = match priority {
                "critical" => 100.0,
                "high" => 80.0,
                "medium" => 55.0,
                "low" => 30.0,
                _ => 45.0,
            };
            let backlog_boost = if title.contains("queue")
                || title.contains("backlog")
                || scope.contains("queue")
            {
                12.0
            } else {
                0.0
            };
            let audit_boost =
                if title.contains("audit") || scope.contains("audit") || owner == "chronos" {
                    10.0
                } else {
                    0.0
                };
            let governance_boost = if title.contains("governance")
                || title.contains("triad")
                || scope.contains("governance")
            {
                10.0
            } else {
                0.0
            };
            let pressure_boost = if pressure_status == "alert"
                && (title.contains("pressure")
                    || title.contains("storage")
                    || title.contains("oversize")
                    || scope.contains("pressure"))
            {
                14.0
            } else {
                0.0
            };
            let score = (base + backlog_boost + audit_boost + governance_boost + pressure_boost)
                * governance_score.max(0.45);
            let mut compact = compact_project_task(task);
            compact["governance_priority"] = json!({
                "score": (score * 100.0).round() / 100.0,
                "base_priority": priority,
                "triad_signal": triad_signal,
                "love_equation_signal": love_signal,
                "joulework_signal": joule_signal,
                "boosts": {
                    "backlog": backlog_boost,
                    "audit": audit_boost,
                    "governance": governance_boost,
                    "pressure": pressure_boost
                }
            });
            compact
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        let a_score = a
            .pointer("/governance_priority/score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let b_score = b
            .pointer("/governance_priority/score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let backlog_alert = latest_open_total > 30;
    let pressure_alert = matches!(pressure_status, "alert" | "error");
    let audit_alert = matches!(audit_status, "alert" | "error" | "fail");
    let payload = json!({
        "schema_version": "annunimas.governance-priority-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export governance-priority-runtime",
        "mutation_policy": "read_only_priority_projection_no_queue_reorder_no_task_mutation",
        "sources": {
            "queue": rel(&queue_path, &root),
            "queue_hygiene": rel(&queue_hygiene_path, &root),
            "pressure_guard": rel(&pressure_guard_path, &root),
            "governance_runtime": rel(&governance_runtime_path, &root),
            "repeated_audit": rel(&repeated_audit_path, &root)
        },
        "thresholds": {
            "backlog_latest_open_alert": 30,
            "pressure_guard_alert_statuses": ["alert", "error"],
            "audit_alert_statuses": ["alert", "error", "fail"]
        },
        "summary": {
            "latest_open_total": latest_open_total,
            "raw_queued_rows_total": raw_queued_rows_total,
            "stale_raw_queued_rows_total": stale_raw_queued_rows_total,
            "ranked_open_total": ranked.len(),
            "backlog_alert": backlog_alert,
            "pressure_guard_status": pressure_status,
            "pressure_violation_total": pressure_violation_total,
            "oversize_files_gte_100mb": oversize_total,
            "repeated_audit_status": audit_status,
            "audit_alert": audit_alert,
            "pressure_alert": pressure_alert,
            "governance_score": governance_score,
            "triad_signal": triad_signal,
            "love_equation_signal": love_signal,
            "joulework_signal": joule_signal
        },
        "governance_signals": governance_signals,
        "ranked_open_tasks": ranked.into_iter().take(32).collect::<Vec<_>>(),
        "monitoring": {
            "prometheus_metrics": [
                "annunimas_queue_latest_open_total",
                "annunimas_queue_stale_raw_queued_rows_total",
                "annunimas_pressure_guard_status",
                "annunimas_pressure_guard_violations_total",
                "annunimas_audit_health_status"
            ],
            "health_endpoint": "/health/audit",
            "dashboard_panel": "audit_governance_runtime"
        },
        "arda_hints": {
            "primary_panel": "governance_priority_runtime",
            "boardroom_section": "triad_joule_love",
            "highlight_backlog_alert": backlog_alert,
            "highlight_pressure_alert": pressure_alert,
            "highlight_audit_alert": audit_alert
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "latest_open_total": payload["summary"]["latest_open_total"],
        "backlog_alert": payload["summary"]["backlog_alert"],
        "pressure_guard_status": payload["summary"]["pressure_guard_status"],
        "ranked_open_total": payload["summary"]["ranked_open_total"]
    }))
}

fn write_compact_queue_summary_projection(
    root: &Path,
    raw_tasks: &[Value],
    tasks: &[Value],
) -> Result<()> {
    let out_path = root.join("core/state/queue_summary.json");
    let runtime_queue = read_jsonl_objects(&root.join("core/queue/queue.jsonl"));
    let open_total = tasks.iter().filter(|task| is_open_task(task)).count();
    let open_tasks = tasks
        .iter()
        .filter(|task| is_open_task(task))
        .take(32)
        .map(compact_project_task)
        .collect::<Vec<_>>();
    let mut recent_tasks = tasks.to_vec();
    recent_tasks.sort_by(|a, b| {
        a.get("queued_at_utc")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("queued_at_utc").and_then(Value::as_str).unwrap_or(""))
    });
    if recent_tasks.len() > 32 {
        recent_tasks = recent_tasks.split_off(recent_tasks.len() - 32);
    }
    let mut recent_runtime_queue = runtime_queue
        .iter()
        .rev()
        .take(32)
        .cloned()
        .collect::<Vec<_>>();
    recent_runtime_queue.reverse();

    let payload = json!({
        "schema_version": "annunimas.core.state.v1",
        "generated_at_utc": now_utc(),
        "authority": "queue_summary_projection",
        "agent_reading_policy": {
            "default_surface": "core/state/queue_active.json",
            "summary_surface": "core/state/queue_summary.json",
            "raw_ledger": "core/projects/tasks/queue.jsonl",
            "raw_ledger_role": "compacted_active_ledger_and_append_target",
            "guidance": "Agents should read queue_active.json for active task selection, then queue_summary.json for counts. Do not bulk-read queue.jsonl; open it only for exact id evidence, append validation, or targeted append."
        },
        "project_tasks": {
            "total_effective": tasks.len(),
            "raw_ledger_rows_total": raw_tasks.len(),
            "history_rows_total": raw_tasks.len().saturating_sub(tasks.len()),
            "counts_by_status": Value::Object(task_status_counts(tasks)),
            "counts_by_owner": count_task_field(tasks, "owner"),
            "counts_by_priority": count_task_field(tasks, "priority"),
            "open_total": open_total,
            "open_compact_limit": 32,
            "open_compact": open_tasks,
            "recent_compact": recent_tasks.iter().map(compact_project_task).collect::<Vec<_>>()
        },
        "runtime_queue": {
            "counts_by_status": count_task_field(&runtime_queue, "status"),
            "counts_by_owner": count_task_field(&runtime_queue, "owner"),
            "recent_compact": recent_runtime_queue.iter().map(compact_project_task).collect::<Vec<_>>()
        },
        "arda_hints": {
            "primary_panel": "task_board",
            "boardroom_section": "execution_queue",
            "alert_on_queued_tasks": tasks.iter().any(|task| task.get("status").and_then(Value::as_str) == Some("queued")),
            "alert_on_failed_tasks": tasks.iter().any(|task| task.get("result").and_then(Value::as_str) == Some("failed"))
        }
    });
    write_pretty_json(&out_path, &payload)
}

fn count_task_field(tasks: &[Value], field: &str) -> Value {
    let mut counts = serde_json::Map::new();
    for task in tasks {
        let key = task
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let current = counts.get(&key).and_then(Value::as_u64).unwrap_or(0);
        counts.insert(key, json!(current + 1));
    }
    Value::Object(counts)
}

fn export_matrix_boardroom_contract() -> Result<Value> {
    export_matrix_boardroom_contract_impl()
}

fn export_federated_comms() -> Result<Value> {
    export_federated_comms_impl()
}

fn export_github_repo_integration() -> Result<Value> {
    export_github_repo_integration_impl()
}

fn export_embodied_controller_runtime() -> Result<Value> {
    export_embodied_controller_runtime_impl()
}

fn export_edge_enrollment_plan() -> Result<Value> {
    export_edge_enrollment_plan_impl()
}

fn export_edge_package_readiness() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/edge_package_readiness.json");
    let edge = read_json_or(
        &root.join("core/state/edge_enrollment_plan.json"),
        json!({}),
    );
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let actions = operator_actions
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let surfaces = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut safe_local_ready = Vec::new();
    let mut observed_not_running = Vec::new();
    let mut human_gated = Vec::new();

    for (tool, surface) in surfaces {
        let status = surface
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let row = json!({
            "tool": tool,
            "status": status,
            "ok": surface.get("ok").cloned().unwrap_or(Value::Null),
            "ready": surface.get("ready").cloned().unwrap_or(Value::Null),
            "runtime_ready": surface.get("runtime_ready").cloned().unwrap_or(Value::Null),
        });
        match status {
            "ready" | "running" | "contract_ready" | "shim_only" | "optional_signal_absent" => {
                safe_local_ready.push(row)
            }
            "not_running" | "not_configured" => observed_not_running.push(row),
            _ => human_gated.push(row),
        }
    }

    let identity_binding_actions = actions
        .iter()
        .filter(|action| action.get("kind").and_then(Value::as_str) == Some("identity_binding"))
        .cloned()
        .collect::<Vec<_>>();
    let fleet_recovery_actions = actions
        .iter()
        .filter(|action| {
            action.get("kind").and_then(Value::as_str) == Some("fleet_recovery_failed")
        })
        .cloned()
        .collect::<Vec<_>>();
    let package_summary = package_enablement
        .get("summary")
        .cloned()
        .unwrap_or(Value::Null);
    let edge_summary = edge.get("summary").cloned().unwrap_or(Value::Null);
    let operator_summary = operator_actions
        .get("summary")
        .cloned()
        .unwrap_or(Value::Null);

    let payload = json!({
        "schema_version": "annunimas.edge-package-readiness.v1",
        "generated_at_utc": now_utc(),
        "authority": "annunimas-cli export edge-package-readiness",
        "mutation_policy": "read_only_reconciliation_no_service_start_no_device_mutation",
        "sources": {
            "edge_enrollment_plan": "core/state/edge_enrollment_plan.json",
            "package_runtime_activation": "core/state/package_runtime_activation.json",
            "package_enablement": "core/state/package_enablement.json",
            "operator_actions": "core/state/operator_actions.json"
        },
        "summary": {
            "edge_identity_binding_required_total": edge_summary.get("identity_binding_required_total").cloned().unwrap_or(Value::Null),
            "operator_human_needed_total": operator_summary.get("human_needed_total").cloned().unwrap_or(Value::Null),
            "package_ready_for_activation_total": package_summary.get("ready_for_activation_total").cloned().unwrap_or(Value::Null),
            "package_policy_ready_total": package_summary.get("policy_ready_total").cloned().unwrap_or(Value::Null),
            "safe_local_ready_surface_total": safe_local_ready.len(),
            "observed_not_running_surface_total": observed_not_running.len(),
            "human_gated_action_total": actions.len(),
            "fleet_recovery_failed_total": fleet_recovery_actions.len()
        },
        "safe_local_ready_surfaces": safe_local_ready,
        "observed_not_running_surfaces": observed_not_running,
        "human_gated_actions": {
            "identity_binding": identity_binding_actions,
            "fleet_recovery": fleet_recovery_actions,
            "other": actions
                .iter()
                .filter(|action| !matches!(action.get("kind").and_then(Value::as_str), Some("identity_binding" | "fleet_recovery_failed")))
                .cloned()
                .collect::<Vec<_>>()
        },
        "decision": {
            "next_safe_local_work": "keep package readiness and edge identity state visible; do not start services or mutate devices",
            "operator_required": actions.len() > 0,
            "activation_allowed_without_operator": false,
            "recommended_followup": "operator confirms canonical edge identity for node-pi5-citadel-avatar before embodied bootstrap; fleet recovery failures remain human-gated"
        }
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "safe_local_ready_surface_total": payload["summary"]["safe_local_ready_surface_total"],
        "human_gated_action_total": payload["summary"]["human_gated_action_total"]
    }))
}

fn export_remote_operator_contract() -> Result<Value> {
    export_remote_operator_contract_impl()
}

fn export_tool_garage_contract() -> Result<Value> {
    export_tool_garage_contract_impl()
}

fn export_communication_adapter_contract() -> Result<Value> {
    export_communication_adapter_contract_impl()
}

fn export_opencode_productization_contract() -> Result<Value> {
    export_opencode_productization_contract_impl()
}

fn export_playwright_mcp_productization_contract() -> Result<Value> {
    export_playwright_mcp_productization_contract_impl()
}

fn export_nanoclaw_productization_contract() -> Result<Value> {
    export_nanoclaw_productization_contract_impl()
}

fn export_runtime_admission_receipts() -> Result<Value> {
    export_runtime_admission_receipts_impl()
}

fn export_operator_actions() -> Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let package_runtime_path = root.join("core/state/package_runtime_activation.json");
    let fleet_recon_path = root.join("core/state/fleet_identity_reconciliation.json");
    let out_path = root.join("core/state/operator_actions.json");

    let mut latest_rows = BTreeMap::new();
    for row in read_jsonl_objects(&queue_path) {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        latest_rows.insert(id.to_string(), row);
    }

    let package_runtime = read_json_or(&package_runtime_path, json!({}));
    let fleet_recon = read_json_or(&fleet_recon_path, json!({}));
    let mut actions = Vec::new();

    for task in latest_rows.values() {
        let status = task.get("status").and_then(Value::as_str).unwrap_or("");
        if status != "blocked" {
            continue;
        }
        let origin = task
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|meta| meta.get("origin"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        actions.push(json!({
            "title": task.get("title").cloned().unwrap_or_else(|| json!("Untitled action")),
            "owner": task.get("owner").cloned().unwrap_or_else(|| json!("unknown")),
            "status": status,
            "kind": if origin == "external_blocker" { "external_blocker" } else { "task_blocker" },
            "note": task.get("notes").cloned().unwrap_or_else(|| json!("Human action required.")),
        }));
    }

    for (tool, state) in package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let status = state
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if !matches!(status, "auth_required" | "configuration_required") {
            continue;
        }
        let prefix = state
            .get("project_root")
            .and_then(Value::as_str)
            .unwrap_or("External package surface");
        let suffix = if status == "auth_required" {
            "Authentication or account linking is required before autonomous activation can continue"
        } else {
            "Configuration is required before autonomous activation can continue"
        };
        actions.push(json!({
            "title": format!("{tool} requires human setup"),
            "owner": "prometheus",
            "status": status,
            "kind": status,
            "note": format!("{prefix}. {suffix}"),
        }));
    }

    for candidate in fleet_recon
        .get("canonical_binding_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(candidate) = candidate.as_object() else {
            continue;
        };
        let target_id = candidate
            .get("target_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-target");
        let names = candidate
            .get("candidate_tailscale_names")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|joined| !joined.is_empty())
            .unwrap_or_else(|| "none".to_string());
        let hostname = candidate
            .get("expected_hostname")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        actions.push(json!({
            "title": format!("{target_id} requires canonical edge identity binding"),
            "owner": "warden",
            "status": "operator_confirmation_required",
            "kind": "identity_binding",
            "note": format!(
                "Bind `{target_id}` to expected hostname `{hostname}` before enrollment. Candidate Tailscale names: {names}."
            ),
        }));
    }

    for cluster in fleet_recon
        .get("stale_hostname_clusters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(cluster) = cluster.as_object() else {
            continue;
        };
        if cluster.get("count").and_then(Value::as_i64).unwrap_or(0) <= 1 {
            continue;
        }
        let hostname = cluster
            .get("hostname")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let ids = cluster
            .get("tailscale_node_ids")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        actions.push(json!({
            "title": format!("Retire stale duplicate identities for {hostname}"),
            "owner": "warden",
            "status": "operator_confirmation_required",
            "kind": "identity_cleanup",
            "note": format!(
                "Multiple stale nodes still share hostname `{hostname}`. Review and retire duplicates: {ids}."
            ),
        }));
    }

    let payload = json!({
        "schema_version": "annunimas.core.state.v1",
        "generated_at_utc": now_utc(),
        "authority": "operator_actions_projection",
        "summary": {
            "human_needed_total": actions.len(),
            "external_blockers_total": actions.iter().filter(|item| item.get("kind").and_then(Value::as_str) == Some("external_blocker")).count(),
            "auth_required_total": actions.iter().filter(|item| item.get("kind").and_then(Value::as_str) == Some("auth_required")).count(),
            "configuration_required_total": actions.iter().filter(|item| item.get("kind").and_then(Value::as_str) == Some("configuration_required")).count(),
            "identity_binding_total": actions.iter().filter(|item| item.get("kind").and_then(Value::as_str) == Some("identity_binding")).count(),
            "identity_cleanup_total": actions.iter().filter(|item| item.get("kind").and_then(Value::as_str) == Some("identity_cleanup")).count(),
        },
        "actions": actions,
        "arda_hints": {
            "primary_panel": "operations_and_packages",
            "boardroom_section": "human_needed",
            "alert_on_human_needed": !actions.is_empty(),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn export_operator_legibility_contract() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/operator_legibility_contract.json");
    let external_brief = read_json_or(
        &root.join("core/state/external_absorption_brief.json"),
        json!({}),
    );
    let model_control = read_json_or(
        &root.join("core/state/model_control_surface.json"),
        json!({}),
    );
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let async_intake = read_json_or(
        &root.join("core/state/async_user_intake_contract.json"),
        json!({}),
    );

    let payload = json!({
        "schema_version": "annunimas.operator-legibility-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "external_absorption_brief + model_control_surface + runtime_governor",
        "mission": {
            "goal": "Make Annunimas readable as a product and operator surface without creating a second authority plane.",
            "driver": "Mission Control comparison signal",
        },
        "operator_promises": [
            "One control plane backed by sovereign state, not hidden side channels.",
            "Health, routing, cost, and queue posture must be readable without code spelunking.",
            "Foreground chat and background work must appear as one system, not unrelated subsystems.",
        ],
        "productization_lanes": [
            {
                "lane": "dashboard_legibility",
                "goal": "Compress core system truth into a smaller set of obvious operator panels.",
                "source_signals": [
                    "core/state/model_control_surface.json",
                    "core/state/runtime_governor_contract.json",
                    "core/state/async_user_intake_runtime.json",
                ],
            },
            {
                "lane": "continuity_legibility",
                "goal": "Expose background intake, memory continuity, and routed execution as one lifecycle.",
                "source_signals": [
                    "core/state/async_user_intake_contract.json",
                    "core/state/agent_continuity_contract.json",
                    "core/state/project_task_executor.json",
                ],
            },
            {
                "lane": "governance_legibility",
                "goal": "Show Soterion, JouleWork, and route pressure without making the operator read raw ledgers.",
                "source_signals": [
                    "core/state/soterion_joulework_enforcement.json",
                    "core/state/runtime_budget_policy.json",
                    "core/state/opencode_route_governor.json",
                ],
            },
        ],
        "non_goals": [
            "Do not fork a second orchestration runtime outside Annunimas state authority.",
            "Do not rebuild another generic dashboard for parity theater.",
            "Do not privilege UI preferences over sovereign routing/governor truth.",
        ],
        "summary": {
            "productization_lanes_total": 3,
            "provider_catalog_total": model_control
                .get("charon_providers")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "governor_capability_lanes_total": runtime_governor
                .get("capability_lanes")
                .and_then(Value::as_object)
                .map(serde_json::Map::len)
                .unwrap_or(0),
            "async_handoff_steps_total": async_intake
                .get("summary")
                .and_then(Value::as_object)
                .and_then(|summary| summary.get("handoff_steps_total"))
                .cloned()
                .unwrap_or(Value::Null),
            "comparison_sources_total": external_brief
                .get("comparison_set")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn export_runtime_budget_policy() -> Result<Value> {
    export_runtime_budget_policy_impl()
}

fn export_runtime_admission_recovery() -> Result<Value> {
    export_runtime_admission_recovery_impl()
}

fn export_memory_governor() -> Result<Value> {
    export_memory_governor_impl()
}

fn export_metrics_delta() -> Result<Value> {
    export_metrics_delta_impl()
}

fn command_probe(cmd: &[&str], cwd: &Path) -> (bool, String) {
    let Some((program, args)) = cmd.split_first() else {
        return (false, "empty command".to_string());
    };
    match Command::new(program).args(args).current_dir(cwd).output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(if output.stdout.is_empty() {
                &output.stderr
            } else {
                &output.stdout
            })
            .trim()
            .chars()
            .take(1200)
            .collect::<String>();
            (output.status.success(), text)
        }
        Err(err) => (false, err.to_string()),
    }
}

fn shell_surface(script: &str, action: &str, root: &Path) -> Value {
    let (ok, output) = command_probe(&["bash", script, action], root);
    if output.is_empty() {
        return json!({"ok": ok, "status": "no_output"});
    }
    match serde_json::from_str::<Value>(&output) {
        Ok(mut payload) if payload.is_object() => {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("ok".to_string(), Value::from(ok));
            }
            payload
        }
        _ => json!({"ok": ok, "status": "invalid_json", "raw_output": output}),
    }
}

fn export_package_runtime_activation() -> Result<Value> {
    export_package_runtime_activation_impl()
}

fn export_package_health() -> Result<Value> {
    export_package_health_impl()
}

fn export_package_enablement() -> Result<Value> {
    export_package_enablement_impl()
}

fn export_edge_model_rollout() -> Result<Value> {
    export_edge_model_rollout_impl()
}

fn export_runtime_governor_contract() -> Result<Value> {
    export_runtime_governor_contract_impl()
}

fn export_search_runtime_contract() -> Result<Value> {
    export_search_runtime_contract_impl()
}

fn export_scrapling_runtime_contract() -> Result<Value> {
    export_scrapling_runtime_contract_impl()
}

fn export_network_native_node_onboarding_contract() -> Result<Value> {
    export_network_native_node_onboarding_contract_impl()
}

fn export_edge_identity_remediation_contract() -> Result<Value> {
    export_edge_identity_remediation_contract_impl()
}

fn export_governance_gap_backlog() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/governance_gap_backlog.json");
    let matrix = read_json_or(
        &root.join("core/metrics/by_crate/governance/gate_matrix.json"),
        json!({}),
    );
    let mut gaps = matrix
        .get("crates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|crate_row| {
            let mut missing = Vec::new();
            let mut weak = Vec::new();
            let signals = crate_row.get("signals").and_then(Value::as_object)?;
            for (signal, info) in signals {
                match info.get("state").and_then(Value::as_str) {
                    Some("absent") => missing.push(Value::String(signal.clone())),
                    Some("emitted_or_symbolic") => weak.push(Value::String(signal.clone())),
                    _ => {}
                }
            }
            if missing.is_empty() && weak.is_empty() {
                return None;
            }
            let crate_name = crate_row.get("crate").and_then(Value::as_str).unwrap_or("");
            let priority = if matches!(
                crate_name,
                "annunimas-apollo" | "annunimas-oracle" | "annunimas-plutus"
            ) {
                "high"
            } else {
                "medium"
            };
            Some(json!({
                "crate": crate_name,
                "priority": priority,
                "missing": missing,
                "weak": weak,
                "recommended_move": if priority == "high" {
                    "embed runtime governance at execution boundary"
                } else {
                    "raise symbolic coverage into enforced runtime gates"
                },
            }))
        })
        .collect::<Vec<_>>();
    gaps.sort_by(|a, b| {
        let a_rank = if a.get("priority").and_then(Value::as_str) == Some("high") {
            0
        } else {
            1
        };
        let b_rank = if b.get("priority").and_then(Value::as_str) == Some("high") {
            0
        } else {
            1
        };
        (a_rank, a.get("crate").and_then(Value::as_str).unwrap_or(""))
            .cmp(&(b_rank, b.get("crate").and_then(Value::as_str).unwrap_or("")))
    });
    let payload = json!({
        "schema_version": "annunimas.governance-gap-backlog.v1",
        "generated_at_utc": now_utc(),
        "authority": "governance_gate_matrix",
        "summary": {
            "crates_with_gaps_total": gaps.len(),
            "high_priority_total": gaps.iter().filter(|gap| gap.get("priority").and_then(Value::as_str) == Some("high")).count(),
        },
        "gaps": gaps,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn crate_summary(crate_dir: &Path, root: &Path) -> Value {
    let crate_name = crate_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| rel(crate_dir, root));
    let src_dir = crate_dir.join("src");
    let readme = crate_dir.join("README.md");
    let cargo_toml = crate_dir.join("Cargo.toml");
    let mut signals = serde_json::Map::new();

    for signal in SIGNALS {
        let src_hits = rg_files(signal.patterns, &src_dir, root);
        let mut all_hits = src_hits.clone();
        all_hits.extend(rg_files(signal.patterns, &crate_dir.join("data"), root));
        all_hits.extend(rg_files(signal.patterns, &readme, root));
        all_hits.sort();
        all_hits.dedup();
        let state = if crate_name == "annunimas-ceo" {
            "shim"
        } else if !src_hits.is_empty() {
            "implemented"
        } else if !all_hits.is_empty() {
            "emitted_or_symbolic"
        } else {
            "absent"
        };
        signals.insert(
            signal.key.to_string(),
            json!({
                "state": state,
                "src_hits": src_hits.iter().take(12).map(|hit| rel(Path::new(hit), root)).collect::<Vec<_>>(),
                "sample_hits": all_hits.iter().take(12).map(|hit| rel(Path::new(hit), root)).collect::<Vec<_>>(),
                "src_hit_count": src_hits.len(),
                "hit_count": all_hits.len(),
            }),
        );
    }

    json!({
        "crate": crate_name,
        "cargo_toml": if cargo_toml.exists() { Some(rel(&cargo_toml, root)) } else { None::<String> },
        "kind": if crate_name == "annunimas-ceo" { "shim" } else { "workspace_crate" },
        "signals": signals,
    })
}

fn aggregate_gate_matrix(rows: &[Value]) -> Value {
    let mut totals = serde_json::Map::new();
    for signal in SIGNALS {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::from([
            ("implemented", 0),
            ("emitted_or_symbolic", 0),
            ("absent", 0),
            ("shim", 0),
        ]);
        for row in rows {
            if let Some(state) = row
                .get("signals")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get(signal.key))
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("state"))
                .and_then(Value::as_str)
            {
                if let Some(count) = counts.get_mut(state) {
                    *count += 1;
                }
            }
        }
        totals.insert(signal.key.to_string(), json!(counts));
    }
    Value::Object(totals)
}

fn export_gate_matrix() -> Result<Value> {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    let out_json = root.join("core/metrics/by_crate/governance/gate_matrix.json");
    let out_last = root.join("data/prometheus/gate_matrix_last.json");
    let out_history = root.join("data/prometheus/gate_matrix_history.jsonl");

    let mut rows = Vec::new();
    for entry in fs::read_dir(&crates_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            rows.push(crate_summary(&path, &root));
        }
    }
    rows.sort_by(|a, b| a["crate"].as_str().cmp(&b["crate"].as_str()));

    let payload = json!({
        "generated_at_utc": now_utc(),
        "authority": "governance_gate_matrix",
        "schema_version": "annunimas.governance.gate-matrix.v1",
        "crates_total": rows.len(),
        "signals": SIGNALS.iter().map(|signal| signal.key).collect::<Vec<_>>(),
        "totals": aggregate_gate_matrix(&rows),
        "crates": rows,
    });

    write_pretty_json(&out_json, &payload)?;
    write_pretty_json(&out_last, &payload)?;
    if let Some(parent) = out_history.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut history = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out_history)?;
    history.write_all((serde_json::to_string(&payload)? + "\n").as_bytes())?;

    Ok(json!({
        "json": rel(&out_json, &root),
        "last": rel(&out_last, &root),
        "history": rel(&out_history, &root),
        "payload": payload,
    }))
}

fn parse_title(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    for line in raw.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            if !title.contains("Quick Reference") {
                return Some(title.trim().to_string());
            }
        }
    }
    None
}

fn read_queue_counts(path: &Path) -> Result<HashMap<String, usize>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return Ok(HashMap::new()),
    };
    let mut latest = HashMap::<String, Value>::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Ok(record) = serde_json::from_str::<Value>(line) {
            let Some(id) = record.get("id").and_then(Value::as_str) else {
                continue;
            };
            latest.insert(id.to_string(), record);
        }
    }

    let mut counts = HashMap::new();
    for record in latest.values() {
        let status = record
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(status.as_str(), "completed" | "cancelled") {
            continue;
        }
        let owner = record
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        *counts.entry(owner).or_insert(0) += 1;
    }
    Ok(counts)
}

fn export_plan_index() -> Result<Value> {
    let root = workspace_root();
    let human_plan_root = root.join("human/plans");
    let core_plan_root = root.join("core/projects/Plans");
    let task_queue_path = root.join("core/projects/tasks/queue.jsonl");
    let human_index_path = human_plan_root.join("index.json");
    let core_index_path = root.join("core/state/plan_map.json");

    let open_counts = read_queue_counts(&task_queue_path)?;
    let mut entries = Vec::new();

    if human_plan_root.exists() {
        for entry in fs::read_dir(&human_plan_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
                continue;
            }
            let slug = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let core_path = core_plan_root.join(file_name);
            entries.push(json!({
                "id": slug,
                "title": parse_title(&path).unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().to_string()),
                "human_plan_path": rel(&path, &root),
                "core_quick_ref_path": rel(&core_path, &root),
                "owner": slug,
                "open_task_count": open_counts.get(&slug).copied().unwrap_or(0),
                "present": {
                    "human_plan": path.exists(),
                    "core_quick_ref": core_path.exists(),
                },
            }));
        }
    }

    entries.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let payload = json!({
        "schema_version": "annunimas.plan.index.v1",
        "generated_at_utc": now_utc(),
        "authority": "plan_index_export",
        "human_plan_root": "human/plans",
        "core_plan_root": "core/projects/Plans",
        "task_queue_path": "core/projects/tasks/queue.jsonl",
        "summary": {
            "plans_total": entries.len(),
            "open_task_total": entries.iter().filter_map(|entry| entry.get("open_task_count").and_then(Value::as_u64)).sum::<u64>(),
            "plans_with_open_tasks_total": entries.iter().filter(|entry| entry.get("open_task_count").and_then(Value::as_u64).unwrap_or(0) > 0).count(),
        },
        "plans": entries,
    });

    write_pretty_json(&human_index_path, &payload)?;
    write_pretty_json(&core_index_path, &payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_json_path(name: &str) -> Result<PathBuf> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!("annunimas-cli-{name}-{stamp}.json")))
    }

    fn temp_root(name: &str) -> Result<PathBuf> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("annunimas-cli-{name}-{stamp}"));
        fs::create_dir_all(root.join("core/state"))?;
        fs::create_dir_all(root.join("core/projects/tasks"))?;
        Ok(root)
    }

    #[test]
    fn write_pretty_json_skips_timestamp_only_changes() -> Result<()> {
        let path = temp_json_path("timestamp-only")?;
        let original = json!({
            "schema_version": "test.v1",
            "generated_at_utc": "2026-01-01T00:00:00Z",
            "nested": {
                "generated_at_utc": "2026-01-01T00:00:00Z",
                "value": 42
            }
        });
        write_pretty_json(&path, &original)?;
        let before = fs::read_to_string(&path)?;

        let timestamp_only = json!({
            "schema_version": "test.v1",
            "generated_at_utc": "2026-01-02T00:00:00Z",
            "nested": {
                "generated_at_utc": "2026-01-02T00:00:00Z",
                "value": 42
            }
        });
        write_pretty_json(&path, &timestamp_only)?;
        let after_timestamp_only = fs::read_to_string(&path)?;
        assert_eq!(before, after_timestamp_only);

        let changed = json!({
            "schema_version": "test.v1",
            "generated_at_utc": "2026-01-03T00:00:00Z",
            "nested": {
                "generated_at_utc": "2026-01-03T00:00:00Z",
                "value": 43
            }
        });
        write_pretty_json(&path, &changed)?;
        let after_changed = fs::read_to_string(&path)?;
        assert_ne!(after_timestamp_only, after_changed);

        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn lane_record_class_normalization_preserves_lane_subclass_semantics() {
        let cases = [
            ("promotion_candidate", "proposal"),
            ("already_promoted", "evidence"),
            ("human_signal", "evidence"),
            ("delivery", "execution_attempt"),
            ("closeout", "terminal_result"),
            ("write_request", "approval"),
        ];

        for (lane_subclass, expected_record_class) in cases {
            assert_eq!(
                canonical_lane_record_class(lane_subclass),
                expected_record_class
            );
        }
    }

    #[test]
    fn queue_federation_stage_validation_flags_bad_transitions() {
        let raw_tasks = vec![json!({
            "id": "promoted_without_receipt",
            "status": "completed",
            "result": "completed",
            "meta": {"origin": "queue_federation_promotion"}
        })];
        let latest_tasks = vec![json!({
            "id": "terminal_without_evidence",
            "status": "completed",
            "meta": {"origin": "test"}
        })];
        let source_summaries = vec![json!({
            "id": "bad_lane",
            "default_record_class": "telemetry",
            "allowed_emits": ["proposal", "closeout"]
        })];
        let promotion_candidates = vec![
            json!({
                "source_queue": "source/proposals.jsonl",
                "source_record_id": "proposal_a",
                "record_class": "proposal",
                "promotion_ready": true,
                "risk_lane": "safe-local_candidate_unverified"
            }),
            json!({
                "source_queue": "source/evidence.jsonl",
                "source_record_id": "evidence_a",
                "record_class": "evidence",
                "promotion_ready": true
            }),
            json!({
                "source_queue": "source/approvals.jsonl",
                "source_record_id": "approval_a",
                "record_class": "approval",
                "promotion_ready": false,
                "owner": "arandur",
                "provenance": {"phase": "requested"}
            }),
        ];

        let validation = queue_federation_stage_validation(
            &raw_tasks,
            &latest_tasks,
            &source_summaries,
            &promotion_candidates,
        );

        assert_eq!(
            validation.get("status").and_then(Value::as_str),
            Some("error")
        );
        assert_eq!(validation.get("errors_total"), Some(&json!(3)));
        assert_eq!(validation.get("warnings_total"), Some(&json!(4)));
        let errors = validation
            .get("errors")
            .and_then(Value::as_array)
            .expect("errors array");
        assert!(errors
            .iter()
            .any(|warning| warning.get("id")
                == Some(&json!("central_terminal_result_missing_evidence"))));
        let warnings = validation
            .get("warnings")
            .and_then(Value::as_array)
            .expect("warnings array");
        assert!(warnings.iter().any(|warning| warning.get("id")
            == Some(&json!("federation_promotion_missing_source_receipt"))));
        assert!(warnings
            .iter()
            .any(|warning| warning.get("id")
                == Some(&json!("approval_missing_scope_receipt_fields"))));
    }

    #[test]
    fn queue_federation_control_projection_selects_only_receipt_valid_safe_local_candidates() {
        let queue_federation = json!({
            "schema_version": "annunimas.queue-federation.v1",
            "sources": [
                {
                    "id": "flywheel_packet_runtime",
                    "default_record_class": "proposal",
                    "lane_subclass": "flywheel_plan_packet",
                    "allowed_emits": ["proposal", "evidence", "approval"],
                    "allowed_mutations": [],
                    "promotion_receipt_required": "flywheel_plan_packet_readiness_receipt",
                    "human_gated": false
                },
                {
                    "id": "human_workspace",
                    "default_record_class": "evidence",
                    "lane_subclass": "human_signal",
                    "allowed_emits": ["proposal", "evidence", "approval"],
                    "allowed_mutations": [],
                    "promotion_receipt_required": "explicit_human_promotion_receipt",
                    "human_gated": true
                }
            ],
            "promotion_candidates": [
                {
                    "source_queue": "core/state/flywheel_packet_runtime.json",
                    "source_record_id": "packet-safe",
                    "record_class": "proposal",
                    "lane_subclass": "flywheel_plan_packet",
                    "promotion_ready": true,
                    "required_contract": "flywheel_plan_packet_readiness_receipt",
                    "risk_lane": "safe-local_candidate_unverified",
                    "promotion_receipt": "receipt-safe"
                },
                {
                    "source_queue": "crates/annunimas-athena/core/projects/tasks/queue.jsonl",
                    "source_record_id": "packet-missing-receipt",
                    "record_class": "proposal",
                    "lane_subclass": "promotion_candidate",
                    "promotion_ready": true,
                    "required_contract": "prometheus_safe_local_task_promotion",
                    "risk_lane": "safe-local_candidate_unverified"
                },
                {
                    "source_queue": "human/",
                    "source_record_id": "human-note",
                    "record_class": "evidence",
                    "lane_subclass": "human_signal",
                    "promotion_ready": true,
                    "required_contract": "explicit_human_promotion_receipt",
                    "risk_lane": "human_review_required",
                    "promotion_receipt": "receipt-human"
                }
            ]
        });
        let flywheel = json!({
            "schema_version": "annunimas.flywheel.packet_runtime.v1",
            "packets": [
                {
                    "packet_id": "packet-safe",
                    "task_id": "task-safe",
                    "title": "Safe local packet",
                    "readiness": "ready",
                    "status": "queued",
                    "risk": "safe-local",
                    "receipt_surface": "core/state/flywheel_packet_runtime.json"
                }
            ],
            "summary": {"packet_total": 1, "ready_total": 1, "blocked_total": 0}
        });

        let control = queue_federation_control_projection(&queue_federation, &flywheel);

        assert_eq!(
            control.get("schema_version").and_then(Value::as_str),
            Some("annunimas.queue-federation-control.v1")
        );
        assert_eq!(
            control.pointer("/selector/mode").and_then(Value::as_str),
            Some("safe_local_receipt_valid_only")
        );
        assert_eq!(
            control
                .pointer("/selector/next_safe_local_candidate/source_record_id")
                .and_then(Value::as_str),
            Some("packet-safe")
        );
        assert_eq!(control.pointer("/selector/rejected_total"), Some(&json!(2)));
        assert_eq!(
            control.pointer("/projection_cache/source_paths"),
            Some(&json!([
                "core/state/queue_federation.json",
                "core/state/flywheel_packet_runtime.json"
            ]))
        );
        assert_eq!(
            control
                .pointer("/control_packets/0/record_class")
                .and_then(Value::as_str),
            Some("proposal")
        );
        assert_eq!(
            control
                .pointer("/control_packets/0/lane_subclass")
                .and_then(Value::as_str),
            Some("flywheel_plan_packet")
        );
        assert_eq!(
            control
                .pointer("/control_packets/0/promotion_receipt_required")
                .and_then(Value::as_str),
            Some("flywheel_plan_packet_readiness_receipt")
        );
    }

    #[test]
    fn queue_projection_reconciler_uses_latest_state_by_id() -> Result<()> {
        let root = temp_root("queue-reconcile")?;
        let raw_tasks = vec![
            json!({
                "id": "task_a",
                "title": "Task A",
                "owner": "hades",
                "priority": "high",
                "status": "queued",
                "queued_at_utc": "2026-06-07T00:00:00Z",
                "meta": {"origin": "test", "scope": "queue"}
            }),
            json!({
                "id": "task_b",
                "title": "Task B",
                "owner": "prometheus",
                "priority": "medium",
                "status": "queued",
                "queued_at_utc": "2026-06-07T00:01:00Z",
                "meta": {"origin": "test", "scope": "queue"}
            }),
            json!({
                "id": "task_a",
                "title": "Task A",
                "owner": "hades",
                "priority": "high",
                "status": "completed",
                "result": "completed",
                "queued_at_utc": "2026-06-07T00:02:00Z",
                "completed_at_utc": "2026-06-07T00:02:00Z",
                "meta": {"origin": "test", "scope": "queue"}
            }),
        ];
        let tasks = latest_project_tasks(&raw_tasks);

        let hygiene = write_queue_hygiene_projection(&root, &raw_tasks, &tasks)?;
        write_compact_queue_summary_projection(&root, &raw_tasks, &tasks)?;
        let active = write_queue_active_projection(&root, &raw_tasks, &tasks)?;

        let summary = read_json_or(&root.join("core/state/queue_summary.json"), json!({}));
        let active_file = read_json_or(&root.join("core/state/queue_active.json"), json!({}));

        assert_eq!(
            hygiene.pointer("/metrics/latest_open_total"),
            Some(&json!(1))
        );
        assert_eq!(active.get("active_task_count"), Some(&json!(1)));
        assert_eq!(active_file.get("active_task_count"), Some(&json!(1)));
        assert_eq!(
            summary.pointer("/project_tasks/open_total"),
            Some(&json!(1))
        );
        assert_eq!(
            active_file.pointer("/tasks/0/id").and_then(Value::as_str),
            Some("task_b")
        );
        assert_eq!(
            active_file.get("authority").and_then(Value::as_str),
            Some("queue_active_projection")
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }
}
