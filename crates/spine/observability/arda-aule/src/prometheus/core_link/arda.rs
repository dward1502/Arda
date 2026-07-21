#![cfg(feature = "full-cli")]
use super::{
    collect_file_paths, read_json_file, read_recent_jsonl, rel_path, CORE_STATE_SCHEMA_VERSION,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn write_arda_source_map(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("arda_source_map.json");
    let generic_snapshot_path = core_root.join("state").join("system_source_map.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let crate_embedded_state = collect_crate_embedded_state(&workspace_root.join("crates"));
    let sections = vec![
        json!({
            "id": "sovereign_world",
            "title": "Sovereign World",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/world.json"), &workspace_root),
                rel_path(core_root.join("state/system_manifest.json"), &workspace_root),
                rel_path(core_root.join("state/system_control.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/active_ruleset.json"), &workspace_root),
                rel_path(core_root.join("state/autonomy_runtime.json"), &workspace_root)
            ],
            "arda_panels": ["3d_world", "executive_overview"]
        }),
        json!({
            "id": "governance_guardhouse",
            "title": "Governance And Guardhouse",
            "owner": "warden",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/warden_guardhouse.json"), &workspace_root),
                rel_path(core_root.join("state/warden_policy_authority.json"), &workspace_root),
                rel_path(core_root.join("state/warden_edge_contract.json"), &workspace_root),
                rel_path(core_root.join("state/warden_nightly_doctrine.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/prometheus/gate_matrix_last.json"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/gate_metrics_last.json"), &workspace_root)
            ],
            "arda_panels": ["security_posture", "edge_guardhouse", "policy_authority"]
        }),
        json!({
            "id": "knowledge_and_reasoning",
            "title": "Knowledge And Reasoning",
            "owner": "athena_oracle",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/athena_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/oracle_runtime.json"), &workspace_root),
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/athena/digest.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/athena/deep_graph.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/knowledge/athena/index/sources.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/athena/policy_readiness.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/athena/deep_queue.jsonl"), &workspace_root)
            ],
            "missing_projections": ["human_library_projection_for_boardroom_consumption"],
            "arda_panels": ["knowledge_corpus", "verdict_stream", "source_graph"]
        }),
        json!({
            "id": "routing_and_comms",
            "title": "Routing And Communications",
            "owner": "manwe_hermes",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/manwe_router.json"), &workspace_root),
                rel_path(core_root.join("state/hermes_command.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/hermes/boardroom.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/hermes/interruptions.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/arda_presence_events.jsonl"), &workspace_root)
            ],
            "arda_panels": ["boardroom", "inference_router", "interrupts"]
        }),
        json!({
            "id": "lifecycle_execution_economics",
            "title": "Lifecycle Execution Economics",
            "owner": "hades_apollo_plutus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/hades_lifecycle.json"), &workspace_root),
                rel_path(core_root.join("state/apollo_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/plutus_runtime.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/prometheus/health_workflow_last.json"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/pressure_guard_last.json"), &workspace_root),
                rel_path(workspace_root.join("data/hades/joulework.jsonl"), &workspace_root)
            ],
            "arda_panels": ["maintenance", "task_execution", "joulework_and_budget"]
        }),
        json!({
            "id": "memory_and_continuity",
            "title": "Memory And Continuity",
            "owner": "mnemosyne",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/mnemosyne_continuity.json"), &workspace_root),
                rel_path(core_root.join("state/memory_identity.json"), &workspace_root),
                rel_path(core_root.join("state/memory_activity.json"), &workspace_root),
                rel_path(core_root.join("state/memory_scopes.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/mnemosyne/obsidian_index.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/mnemosyne/noise.jsonl"), &workspace_root)
            ],
            "arda_panels": ["memory", "identity_continuity", "memory_activity", "memory_scope_map"]
        }),
        json!({
            "id": "planning_and_queue",
            "title": "Planning And Queue",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/queue_summary.json"), &workspace_root),
                rel_path(core_root.join("state/escalation_runtime.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("core/projects/tasks/queue.jsonl"), &workspace_root),
                rel_path(workspace_root.join("core/queue/queue.jsonl"), &workspace_root),
                rel_path(workspace_root.join("core/projects/Plans"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/orders.jsonl"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/escalations.jsonl"), &workspace_root)
            ],
            "arda_panels": ["task_board", "plan_progress", "escalation_queue"]
        }),
        json!({
            "id": "soterion_language",
            "title": "Soterion Language",
            "owner": "prometheus_core",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/soterion_render_contract.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("meta/soterion_sigils.yaml"), &workspace_root)
            ],
            "arda_panels": ["soterion_language", "operations_flow", "executive_overview"]
        }),
        json!({
            "id": "governance_and_operations",
            "title": "Governance And Operations",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/governance_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/operations_flow.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("core/metrics/by_crate/governance/signals.json"), &workspace_root),
                rel_path(workspace_root.join("core/metrics/by_crate/prometheus/ops_dashboard.json"), &workspace_root)
            ],
            "arda_panels": ["governance_runtime", "operations_flow", "executive_overview"]
        }),
        json!({
            "id": "paperclip_alignment",
            "title": "Paperclip Alignment",
            "owner": "prometheus_manwe_hades_athena",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/paperclip_alignment.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/governance_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/manwe_router.json"), &workspace_root),
                rel_path(core_root.join("state/control_plane_lockdown.json"), &workspace_root),
                rel_path(core_root.join("state/athena_runtime.json"), &workspace_root),
                rel_path(workspace_root.join("data/athena/policy_readiness.jsonl"), &workspace_root),
                rel_path(workspace_root.join("core/projects/tasks/queue.jsonl"), &workspace_root)
            ],
            "arda_panels": ["paperclip_alignment", "governance_runtime", "operations_flow"]
        }),
        json!({
            "id": "operations_health",
            "title": "Operations Health",
            "owner": "prometheus_hades",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/storage_pressure.json"), &workspace_root),
                rel_path(core_root.join("state/package_health.json"), &workspace_root),
                rel_path(core_root.join("state/package_enablement.json"), &workspace_root),
                rel_path(core_root.join("state/package_runtime_activation.json"), &workspace_root),
                rel_path(core_root.join("state/extension_surface_contract.json"), &workspace_root),
                rel_path(core_root.join("state/extension_activation_backlog.json"), &workspace_root),
                rel_path(core_root.join("state/aipkg_marketplace_separation_contract.json"), &workspace_root),
                rel_path(core_root.join("state/network_native_node_onboarding_contract.json"), &workspace_root),
                rel_path(core_root.join("state/operator_actions.json"), &workspace_root),
                rel_path(core_root.join("state/task_agent_boundaries.json"), &workspace_root),
                rel_path(core_root.join("state/runtime_settings.json"), &workspace_root),
                rel_path(core_root.join("state/control_plane_lockdown.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("data/prometheus/compaction_last.json"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/package_health_last.json"), &workspace_root),
                rel_path(workspace_root.join("core/metrics/audit_latest.json"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/control_plane/policy_snapshot.json"), &workspace_root),
                rel_path(workspace_root.join("config/runtime.generated.env"), &workspace_root)
            ],
            "arda_panels": ["storage_pressure", "package_health", "package_enablement", "runtime_settings", "control_plane_lockdown"]
        }),
        json!({
            "id": "framework_extensions",
            "title": "Framework Extensions",
            "owner": "prometheus_athena",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/agent_framework_alignment.json"), &workspace_root),
                rel_path(core_root.join("state/agentforge_alignment.json"), &workspace_root),
                rel_path(core_root.join("state/eliza_alignment.json"), &workspace_root),
                rel_path(core_root.join("state/extension_surface_contract.json"), &workspace_root),
                rel_path(core_root.join("state/extension_activation_backlog.json"), &workspace_root),
                rel_path(core_root.join("state/aipkg_marketplace_separation_contract.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/aipkg_contract.json"), &workspace_root),
                rel_path(core_root.join("state/crate_spawn_contract.json"), &workspace_root),
                rel_path(core_root.join("state/package_enablement.json"), &workspace_root)
            ],
            "arda_panels": ["package_enablement", "operations_flow", "executive_overview"]
        }),
        json!({
            "id": "package_marketplace_doctrine",
            "title": "Package Marketplace Doctrine",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/aipkg_contract.json"), &workspace_root),
                rel_path(core_root.join("state/aipkg_marketplace_separation_contract.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/openfang_alignment.json"), &workspace_root),
                rel_path(core_root.join("state/extension_surface_contract.json"), &workspace_root),
                rel_path(core_root.join("state/extension_activation_backlog.json"), &workspace_root),
                rel_path(core_root.join("state/package_runtime_activation.json"), &workspace_root)
            ],
            "arda_panels": ["package_enablement", "operations_flow", "executive_overview"]
        }),
        json!({
            "id": "communication_adapters",
            "title": "Communication Adapters",
            "owner": "hermes",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/federated_comms_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/hermes_command.json"), &workspace_root),
                rel_path(core_root.join("state/communication_adapter_contract.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/extension_surface_contract.json"), &workspace_root),
                rel_path(core_root.join("state/extension_activation_backlog.json"), &workspace_root),
                rel_path(core_root.join("state/package_runtime_activation.json"), &workspace_root)
            ],
            "arda_panels": ["boardroom", "operations_flow", "package_enablement"]
        }),
        json!({
            "id": "edge_enrollment",
            "title": "Edge Enrollment",
            "owner": "warden_arda",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/fleet_identity_reconciliation.json"), &workspace_root),
                rel_path(core_root.join("state/edge_enrollment_plan.json"), &workspace_root),
                rel_path(core_root.join("state/operator_actions.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/task_agent_boundaries.json"), &workspace_root),
                rel_path(workspace_root.join("core/edge/targets.toml"), &workspace_root),
                rel_path(workspace_root.join("data/fleet/informants"), &workspace_root)
            ],
            "arda_panels": ["fleet_health", "edge_guardhouse", "operations_flow"]
        }),
        json!({
            "id": "network_native_onboarding",
            "title": "Network Native Onboarding",
            "owner": "warden_prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/openfang_alignment.json"), &workspace_root),
                rel_path(core_root.join("state/network_native_node_onboarding_contract.json"), &workspace_root),
                rel_path(core_root.join("state/edge_enrollment_plan.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/fleet_identity_reconciliation.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/warden_edge_contract.json"), &workspace_root),
                rel_path(core_root.join("state/operator_actions.json"), &workspace_root)
            ],
            "arda_panels": ["fleet_health", "edge_guardhouse", "operations_flow"]
        }),
        json!({
            "id": "output_topology",
            "title": "Output Topology",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/output_topology.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("core/metrics/history"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/autopilot"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/supervisor"), &workspace_root),
                rel_path(workspace_root.join("target"), &workspace_root)
            ],
            "arda_panels": ["output_topology", "storage_pressure", "ops_hygiene"]
        }),
        json!({
            "id": "output_accounting",
            "title": "Output Accounting",
            "owner": "prometheus",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/output_accounting.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("state/output_topology.json"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/output_accounting_runs.jsonl"), &workspace_root)
            ],
            "arda_panels": ["output_topology", "ops_hygiene", "storage_pressure"]
        }),
        json!({
            "id": "fleet_and_backbone",
            "title": "Fleet And Backbone",
            "owner": "fleet_observation",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/fleet_runtime.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_nodes.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_models.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_health.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_hardware.json"), &workspace_root),
                rel_path(core_root.join("state/fleet_backbone.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("config/fleet.toml"), &workspace_root),
                rel_path(workspace_root.join("core/edge/targets.toml"), &workspace_root),
                rel_path(workspace_root.join("data/prometheus/fleet_control_last.json"), &workspace_root),
                rel_path(workspace_root.join("data/fleet/informants/local_last.json"), &workspace_root)
            ],
            "arda_panels": ["fleet_health", "node_inventory", "model_inventory", "hardware_inventory", "backbone_topology"]
        }),
        json!({
            "id": "human_business_personal",
            "title": "Human Business Personal",
            "owner": "human_operator",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/human_context.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("docs/operator/index.md"), &workspace_root),
                rel_path(workspace_root.join("docs/operator/onboard.md"), &workspace_root),
                rel_path(workspace_root.join("docs/operator/company-view.md"), &workspace_root),
                rel_path(workspace_root.join("config/business.toml"), &workspace_root),
                rel_path(workspace_root.join("data/business/soterion-business.json"), &workspace_root),
                rel_path(workspace_root.join("docs/operator/notes"), &workspace_root),
                rel_path(workspace_root.join("data/personal"), &workspace_root),
                rel_path(workspace_root.join("core/personal"), &workspace_root)
            ],
            "arda_panels": ["human_notes", "business_ops", "personal_growth"]
        }),
        json!({
            "id": "business_ops",
            "title": "Business Operations",
            "owner": "human_operator",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/business_runtime.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(workspace_root.join("docs/operator/company-view.md"), &workspace_root),
                rel_path(workspace_root.join("config/business.toml"), &workspace_root),
                rel_path(workspace_root.join("data/business/soterion-business.json"), &workspace_root),
                rel_path(workspace_root.join("data/business/clients"), &workspace_root)
            ],
            "arda_panels": ["business_ops", "boardroom", "settings"]
        }),
        json!({
            "id": "personal_growth",
            "title": "Personal Growth",
            "owner": "human_operator",
            "status": "ready",
            "primary_sources": [
                rel_path(core_root.join("state/personal_runtime.json"), &workspace_root)
            ],
            "supplemental_sources": [
                rel_path(core_root.join("personal/personal-identity.toml"), &workspace_root),
                rel_path(workspace_root.join("data/personal/soterion-personal.json"), &workspace_root),
                rel_path(workspace_root.join("data/personal"), &workspace_root),
                rel_path(workspace_root.join("docs/operator/onboard.md"), &workspace_root),
                rel_path(workspace_root.join("docs/operator/notes"), &workspace_root)
            ],
            "arda_panels": ["personal_growth", "human_notes", "boardroom"]
        }),
    ];
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "arda_source_map_projection",
        "purpose": "backend source map for system consumers including ARDA_HUD and external agents",
        "arda_primary_entrypoint_recommended": "core/state/arda_snapshot.json",
        "arda_primary_entrypoint_status": "implemented",
        "system_primary_entrypoint_recommended": "core/state/system_snapshot.json",
        "system_primary_entrypoint_status": "implemented",
        "sections": sections,
        "repo_reorganization": {
            "status": "phase_1_completed",
            "pressure_points": {
                "crate_embedded_state": crate_embedded_state,
                "scattered_human_notes": [
                    "docs/operator/notes",
                    "docs/operator/library",
                    "docs/operator/summaries"
                ],
                "mixed_runtime_and_reference_outputs": [
                    "core/state",
                    "core/metrics",
                    "data/*"
                ]
            },
            "recommended_targets": {
                "machine_read_models": "core/state",
                "time_series_and_ledgers": "data",
                "human_reference": "docs/operator/library",
                "human_working_notes": "docs/operator/notes",
                "integration_contracts": "docs/integrations"
            },
            "notes": [
                "Do not move user notes blindly; project them first.",
                "Remove crate-local data/human mirrors after confirming they are generated artifacts or stale copies.",
                "ARDA should consume core/state projections first and raw ledgers second."
            ]
        }
    });
    write_semantic_source_map(&snapshot_path, &snapshot);
    write_semantic_source_map(&generic_snapshot_path, &snapshot);
}

fn write_semantic_source_map(path: &Path, snapshot: &Value) {
    if let Some(existing) = fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        if source_map_semantically_equal(&existing, snapshot) {
            return;
        }
    }

    let Ok(body) = serde_json::to_string_pretty(snapshot) else {
        return;
    };
    let _ = fs::write(path, body + "\n");
}

fn source_map_semantically_equal(left: &Value, right: &Value) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    if let Some(obj) = left.as_object_mut() {
        obj.remove("generated_at_utc");
    }
    if let Some(obj) = right.as_object_mut() {
        obj.remove("generated_at_utc");
    }
    left == right
}

pub(super) fn write_athena_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("athena_runtime.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let status = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("athena")
            .join("status.json"),
    )
    .unwrap_or_else(|| json!({}));
    let recent_digest = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("athena")
            .join("digest.jsonl"),
        16,
    );
    let recent_policy = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("athena")
            .join("policy_readiness.jsonl"),
        16,
    );
    let recent_deep_queue = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("athena")
            .join("deep_queue.jsonl"),
        16,
    );
    let recent_deep_graph = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("athena")
            .join("deep_graph.jsonl"),
        16,
    );
    let recent_planning_task_receipts = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("athena")
            .join("planning_task_receipts.jsonl"),
        16,
    );
    let recent_sources = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("knowledge")
            .join("athena")
            .join("index")
            .join("sources.jsonl"),
        16,
    );
    let recent_policy_ready = recent_policy
        .iter()
        .filter(|value| {
            value
                .get("policy_readiness")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "policy_ready")
        })
        .count();
    let recent_reference_only = recent_policy
        .iter()
        .filter(|value| {
            value
                .get("policy_readiness")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "reference_only")
        })
        .count();
    let policy_ready = status
        .get("policy_ready_count")
        .and_then(Value::as_u64)
        .unwrap_or(recent_policy_ready as u64);
    let reference_only = status
        .get("reference_only_count")
        .and_then(Value::as_u64)
        .unwrap_or(recent_reference_only as u64);
    let task_emission_receipts_total = status
        .get("task_emission_receipts_total")
        .and_then(Value::as_u64)
        .unwrap_or(recent_planning_task_receipts.len() as u64);
    let task_emission_success_total = status
        .get("task_emission_success_total")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            recent_planning_task_receipts
                .iter()
                .filter(|value| {
                    value
                        .get("disposition")
                        .and_then(Value::as_str)
                        .is_some_and(|status| status == "queued")
                })
                .count() as u64
        });
    let task_emission_skipped_total = status
        .get("task_emission_skipped_total")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            recent_planning_task_receipts
                .iter()
                .filter(|value| {
                    value
                        .get("disposition")
                        .and_then(Value::as_str)
                        .is_some_and(|status| status == "skipped")
                })
                .count() as u64
        });
    let task_emission_last_run_at_utc = status
        .get("task_emission_last_run_at_utc")
        .cloned()
        .unwrap_or_else(|| {
            recent_planning_task_receipts
                .iter()
                .rev()
                .find_map(|value| value.get("ts_utc").and_then(Value::as_str))
                .map(|ts| json!(ts))
                .unwrap_or(Value::Null)
        });
    let policy_readiness_total = policy_ready + reference_only;
    let policy_ready_ratio = if policy_readiness_total > 0 {
        policy_ready as f64 / policy_readiness_total as f64
    } else {
        0.0
    };
    let policy_pressure_status = if policy_ready == 0 && reference_only > 0 {
        "blocked_review_pressure"
    } else if reference_only > policy_ready {
        "review_pressure"
    } else if policy_ready > 0 {
        "promotion_ready"
    } else {
        "no_recent_policy_records"
    };
    let next_operator_action = if policy_ready > 0 {
        "preview_policy_ready_promotion"
    } else if reference_only > 0 {
        "review_reference_only_blockers"
    } else {
        "refresh_athena_digest"
    };

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "athena_runtime_projection",
        "status": status,
        "knowledge": {
            "recent_digest": recent_digest,
            "recent_policy_readiness": recent_policy,
            "recent_deep_queue": recent_deep_queue,
            "recent_deep_graph": recent_deep_graph,
            "recent_sources": recent_sources,
            "counts": {
                "policy_ready_recent": policy_ready,
                "reference_only_recent": reference_only,
                "digest_recent": recent_digest.len(),
                "deep_queue_recent": recent_deep_queue.len(),
                "deep_graph_recent": recent_deep_graph.len(),
                "sources_recent": recent_sources.len(),
                "planning_task_receipts_recent": recent_planning_task_receipts.len()
            }
        },
        "policy_readiness_summary": {
            "source": "data/athena/policy_readiness.jsonl",
            "status": policy_pressure_status,
            "policy_ready_total": policy_ready,
            "reference_only_total": reference_only,
            "review_pressure_total": reference_only.saturating_sub(policy_ready),
            "records_total": policy_readiness_total,
            "policy_ready_ratio": policy_ready_ratio,
            "promotion_preview_available": policy_ready > 0,
            "reference_review_needed": reference_only > 0,
            "governance_gate": "human_review_required",
            "safe_action": "athena.refresh_digest",
            "governed_actions": ["athena.ingest_knowledge", "athena.promote_policy_ready"],
            "next_operator_action": next_operator_action
        },
        "task_emission": {
            "receipts_total": task_emission_receipts_total,
            "success_total": task_emission_success_total,
            "skipped_total": task_emission_skipped_total,
            "last_run_at_utc": task_emission_last_run_at_utc,
            "recent_receipts": recent_planning_task_receipts,
        },
        "arda_hints": {
            "primary_panel": "knowledge",
            "boardroom_section": "research_and_policy",
            "alert_on_deep_queue_failed": status
                .get("deep_queue_failed")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0,
            "alert_on_reference_only_pressure": reference_only > policy_ready
                || policy_pressure_status == "blocked_review_pressure",
            "policy_readiness_status": policy_pressure_status,
            "next_operator_action": next_operator_action
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_apollo_runtime_projection(core_root: &Path) {
    write_runtime_projection(
        core_root,
        "apollo_runtime.json",
        "apollo_runtime_projection",
        Path::new("data/apollo/runtime_status.json"),
        json!({
            "authority": "apollo_runtime_projection",
            "runtime_ready": false,
            "reason": "apollo_runtime_status_missing",
        }),
    );
}

pub(super) fn write_plutus_runtime_projection(core_root: &Path) {
    write_runtime_projection(
        core_root,
        "plutus_runtime.json",
        "plutus_runtime_projection",
        Path::new("data/plutus/runtime_status.json"),
        json!({
            "authority": "plutus_runtime_projection",
            "runtime_ready": false,
            "reason": "plutus_runtime_status_missing",
        }),
    );
}

pub(super) fn write_oracle_runtime_projection(core_root: &Path) {
    write_runtime_projection(
        core_root,
        "oracle_runtime.json",
        "oracle_runtime_projection",
        Path::new("data/oracle/runtime_status.json"),
        json!({
            "authority": "oracle_runtime_projection",
            "runtime_ready": false,
            "reason": "oracle_runtime_status_missing",
        }),
    );
}

fn write_runtime_projection(
    core_root: &Path,
    file_name: &str,
    authority: &str,
    source_path: &Path,
    fallback_runtime: Value,
) {
    let snapshot_path = core_root.join("state").join(file_name);
    let runtime = read_json_file(source_path.to_path_buf()).unwrap_or(fallback_runtime);
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": authority,
        "runtime": runtime,
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_repo_reorganization_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("repo_reorganization.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let crate_embedded_state = collect_crate_embedded_state(&workspace_root.join("crates"));
    let docs_integrations = collect_file_paths(&workspace_root.join("docs/integrations"), "md");

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "repo_reorganization_projection",
        "phase": "phase_1_audit_and_canonicalization",
        "status": "completed",
        "actions_taken": [
            {
                "action": "generated_arda_source_map",
                "path": "core/state/arda_source_map.json"
            },
            {
                "action": "documented_arda_integration_map",
                "path": "docs/integrations/ARDA_INTEGRATION_MAP.md"
            },
            {
                "action": "documented_repo_reorganization_plan",
                "path": "docs/REPO_REORGANIZATION_PLAN.md"
            }
        ],
        "pressure_points": {
            "crate_embedded_state": crate_embedded_state,
            "human_layout": {
                "working_notes": "docs/operator/notes",
                "reference_library": "docs/operator/library",
                "summaries": "docs/operator/summaries"
            },
            "mixed_machine_outputs": [
                "core/state",
                "core/metrics",
                "data"
            ]
        },
        "canonical_targets": {
            "system_projections": "core/state",
            "time_series_ledgers": "data",
            "integration_contracts": "docs/integrations",
            "human_reference": "docs/operator/library",
            "human_working_notes": "docs/operator/notes"
        },
        "artifacts": {
            "integration_docs": docs_integrations
        },
        "arda_hints": {
            "primary_panel": "ops_hygiene",
            "alert_on_crate_mirrors": !crate_embedded_state.is_empty()
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn collect_crate_embedded_state(crates_root: &Path) -> Vec<Value> {
    let entries = match fs::read_dir(crates_root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut findings = Vec::new();
    for entry in entries.flatten() {
        let crate_path = entry.path();
        if !crate_path.is_dir() {
            continue;
        }
        let mut embedded = Vec::new();
        for name in ["data", "human"] {
            if crate_path.join(name).exists() {
                embedded.push(name);
            }
        }
        if embedded.is_empty() {
            continue;
        }
        findings.push(json!({
            "crate": entry.file_name().to_string_lossy().to_string(),
            "path": rel_path(crate_path, crates_root.parent().unwrap_or(crates_root)),
            "embedded_dirs": embedded
        }));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_semantic_equality_ignores_generated_at_only() {
        let left = json!({
            "schema_version": "annunimas.core.state.v1",
            "generated_at_utc": "2026-05-20T01:00:00Z",
            "sections": [{"id": "routing", "status": "ready"}]
        });
        let right = json!({
            "schema_version": "annunimas.core.state.v1",
            "generated_at_utc": "2026-05-20T02:00:00Z",
            "sections": [{"id": "routing", "status": "ready"}]
        });

        assert!(source_map_semantically_equal(&left, &right));
    }

    #[test]
    fn source_map_semantic_equality_detects_source_changes() {
        let left = json!({
            "schema_version": "annunimas.core.state.v1",
            "generated_at_utc": "2026-05-20T01:00:00Z",
            "sections": [{"id": "routing", "supplemental_sources": ["data/hermes/boardroom.jsonl"]}]
        });
        let right = json!({
            "schema_version": "annunimas.core.state.v1",
            "generated_at_utc": "2026-05-20T02:00:00Z",
            "sections": [{"id": "routing", "supplemental_sources": ["data/hermes/interruptions.jsonl"]}]
        });

        assert!(!source_map_semantically_equal(&left, &right));
    }

    #[test]
    fn athena_runtime_projection_prefers_status_totals_over_recent_windows() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp.path();
        let core_root = workspace_root.join("core");
        let athena_metrics = core_root.join("metrics/by_crate/athena");
        let athena_data = workspace_root.join("data/athena");
        fs::create_dir_all(&athena_metrics).expect("create athena metrics dir");
        fs::create_dir_all(&athena_data).expect("create athena data dir");

        fs::write(
            athena_metrics.join("status.json"),
            serde_json::to_string_pretty(&json!({
                "policy_ready_count": 123,
                "reference_only_count": 285,
                "task_emission_receipts_total": 42,
                "task_emission_success_total": 7,
                "task_emission_skipped_total": 35,
                "task_emission_last_run_at_utc": "2026-05-30T12:00:00Z"
            }))
            .expect("serialize status"),
        )
        .expect("write athena status");
        fs::write(
            athena_data.join("policy_readiness.jsonl"),
            "{\"policy_readiness\":\"reference_only\"}\n{\"policy_readiness\":\"reference_only\"}\n",
        )
        .expect("write policy readiness window");
        fs::write(
            athena_data.join("planning_task_receipts.jsonl"),
            "{\"disposition\":\"queued\",\"ts_utc\":\"2026-05-30T11:00:00Z\"}\n",
        )
        .expect("write planning task receipt window");

        write_athena_runtime_projection(&core_root);

        let projected = fs::read_to_string(core_root.join("state/athena_runtime.json"))
            .expect("read projected athena runtime");
        let projected: Value = serde_json::from_str(&projected).expect("parse projected runtime");
        assert_eq!(
            projected["knowledge"]["counts"]["policy_ready_recent"],
            json!(123)
        );
        assert_eq!(
            projected["knowledge"]["counts"]["reference_only_recent"],
            json!(285)
        );
        assert_eq!(projected["task_emission"]["receipts_total"], json!(42));
        assert_eq!(projected["task_emission"]["success_total"], json!(7));
        assert_eq!(projected["task_emission"]["skipped_total"], json!(35));
        assert_eq!(
            projected["task_emission"]["last_run_at_utc"],
            json!("2026-05-30T12:00:00Z")
        );
    }
}
