#![cfg(feature = "full-cli")]
use super::*;

pub(super) fn write_warden_guardhouse(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("warden_guardhouse.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let queue_path = std::env::var("ARDA_WARDEN_QUEUE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/warden/informant_queue.jsonl"));
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let queue_path = if queue_path.is_relative() {
        workspace_root.join(queue_path)
    } else {
        queue_path
    };
    let queue_events = read_recent_jsonl(&queue_path, 64);
    let metrics_heartbeats = latest_events_by_key(
        &queue_events,
        "event_type",
        "crate_health_heartbeat",
        "crate_name",
    );
    let queue_status_counts = summarize_field_counts(&queue_events, "status");
    let queue_repair_pressure = summarize_repair_pressure(&workspace_root, &queue_events);
    let queue_source_counts = summarize_field_counts(&queue_events, "source");
    let policy_permission =
        read_json_file(core_root.join("state").join("permission_profiles.json"))
            .unwrap_or_else(|| json!({"active_profile":"unknown"}));
    let policy_destructive =
        read_json_file(core_root.join("state").join("destructive_quorum.json"))
            .unwrap_or_else(|| json!({"enabled": true}));
    let policy_interrupt = read_json_file(core_root.join("state").join("interrupt_authority.json"))
        .unwrap_or_else(|| json!({"default":{"allow":[]}}));
    let health_workflow = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("health_workflow_last.json"),
    )
    .unwrap_or_else(|| json!({"status":"unknown","issues":[],"actions":[]}));
    let fleet_control = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("fleet_control_last.json"),
    )
    .unwrap_or_else(|| json!({"status":"unknown"}));
    let fleet_health = read_json_file(core_root.join("state").join("fleet_health.json"))
        .unwrap_or_else(|| {
            json!({
                "cleanup_summary": {
                    "status": "unknown",
                    "stale_candidates_total": 0,
                    "offline_recent_total": 0,
                    "safe_review_candidates_total": 0,
                    "safe_review_candidates": [],
                    "safe_action": "refresh_fleet_health_projection"
                }
            })
        });

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "warden_system_projection",
        "system_model": "distributed_guardhouse",
        "edge_role": {
            "node": "warden_pi5_guardhouse",
            "topology": "informants_to_edge_authority",
            "source_of_truth": "core_and_informant_bus"
        },
        "queue": {
            "path": queue_path.display().to_string(),
            "recent_event_count": queue_events.len(),
            "status_counts": queue_status_counts,
            "effective_status_counts": queue_repair_pressure["effective_status_counts"].clone(),
            "repair_pressure": queue_repair_pressure,
            "source_counts": queue_source_counts,
            "recent_events": queue_events,
            "crate_health_heartbeats": metrics_heartbeats
        },
        "policy": {
            "permission_profiles": policy_permission,
            "destructive_quorum": policy_destructive,
            "interrupt_authority": policy_interrupt
        },
        "health": {
            "workflow": health_workflow,
            "fleet_control": fleet_control,
            "fleet_cleanup": fleet_health.get("cleanup_summary").cloned().unwrap_or_else(|| json!({}))
        },
        "duties": [
            "informant_network",
            "quarantine_authority",
            "container_lifecycle",
            "nightly_cleanup",
            "nemesis_enforcement",
            "drift_watch"
        ]
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn summarize_repair_pressure(workspace_root: &Path, events: &[Value]) -> Value {
    let mut latest_repair_by_file = serde_json::Map::new();
    let mut duplicate_repair_events = 0u64;
    let mut non_repair_attention = 0u64;
    let mut resolved_repair_files = Vec::new();
    let mut resolved_orphan_files = Vec::new();

    for event in events {
        let source = event
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if source == "warden_repair_pressure_triage" {
            continue;
        }
        let status = event
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let event_type = event
            .get("event_type")
            .or_else(|| event.get("event"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let is_repair = source == "repair_pipeline" && event_type == "repair_detected";
        if is_repair {
            if let Some(file) = event.get("file").and_then(Value::as_str) {
                if latest_repair_by_file.contains_key(file) {
                    duplicate_repair_events += 1;
                }
                latest_repair_by_file.insert(file.to_string(), event.clone());
            }
        } else if status == "attention_required" {
            if source == "repair_pipeline" && event_type == "orphan_found" {
                if let Some(file) = event.get("file").and_then(Value::as_str) {
                    if file_has_any_sigil(workspace_root, file) {
                        resolved_orphan_files.push(file.to_string());
                        continue;
                    }
                }
            }
            non_repair_attention += 1;
        }
    }

    let mut active_repair_by_file = serde_json::Map::new();
    for (file, event) in &latest_repair_by_file {
        if event.get("status").and_then(Value::as_str) != Some("attention_required") {
            continue;
        }
        if file_is_still_repair_marked(workspace_root, file) {
            active_repair_by_file.insert(file.clone(), event.clone());
        } else {
            resolved_repair_files.push(file.clone());
        }
    }
    let effective_repair_attention = active_repair_by_file.len() as u64;
    let effective_attention_required = effective_repair_attention + non_repair_attention;
    let raw_attention_required = events
        .iter()
        .filter(|event| event.get("status").and_then(Value::as_str) == Some("attention_required"))
        .filter(|event| {
            event.get("source").and_then(Value::as_str) != Some("warden_repair_pressure_triage")
        })
        .count() as u64;
    let repeated_noise = raw_attention_required.saturating_sub(effective_attention_required);

    json!({
        "mode": "dedupe_repair_pipeline_by_file",
        "raw_attention_required": raw_attention_required,
        "effective_attention_required": effective_attention_required,
        "repeated_repair_noise": repeated_noise,
        "duplicate_repair_events": duplicate_repair_events,
        "unique_repair_files": latest_repair_by_file.len(),
        "active_repair_files": active_repair_by_file.len(),
        "resolved_repair_files": resolved_repair_files.len(),
        "resolved_orphan_files": resolved_orphan_files.len(),
        "non_repair_attention_required": non_repair_attention,
        "effective_status_counts": {
            "attention_required": effective_attention_required,
            "repeated_repair_noise": repeated_noise
        },
        "latest_repair_by_file": Value::Object(active_repair_by_file),
        "resolved_repair_file_paths": resolved_repair_files,
        "resolved_orphan_file_paths": resolved_orphan_files,
        "recommended_action": if repeated_noise > 0 {
            "Treat repeated repair detections for the same file as deduplicated pressure; inspect unique files before escalating."
        } else {
            "No repeated repair pressure detected in the recent WARDEN window."
        }
    })
}

fn file_has_any_sigil(workspace_root: &Path, file: &str) -> bool {
    let path = workspace_root.join(file);
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    content.lines().take(80).any(|line| {
        let lowered = line.trim().to_ascii_lowercase();
        lowered.contains("\"sigil\"") || lowered.contains("sigil:") || lowered.contains("sigil =")
    })
}

fn file_is_still_repair_marked(workspace_root: &Path, file: &str) -> bool {
    let path = workspace_root.join(file);
    let Ok(content) = fs::read_to_string(path) else {
        return true;
    };
    for line in content.lines().take(80) {
        let lowered = line.trim().to_ascii_lowercase();
        if lowered.is_empty() {
            continue;
        }
        if lowered.contains("\"sigil\":\"repair\"")
            || lowered.contains("\"sigil\": \"repair\"")
            || lowered.contains("sigil: repair")
            || lowered.contains("sigil = \"repair\"")
        {
            return true;
        }
        if lowered.contains("\"sigil\":\"scroll\"")
            || lowered.contains("\"sigil\": \"scroll\"")
            || lowered.contains("sigil: scroll")
            || lowered.contains("sigil = \"scroll\"")
        {
            return false;
        }
    }
    false
}

pub(super) fn write_warden_policy_authority(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("warden_policy_authority.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let permission_profiles =
        read_json_file(core_root.join("state").join("permission_profiles.json"))
            .unwrap_or_else(|| json!({"active_profile":"unknown","profiles":{}}));
    let destructive_quorum =
        read_json_file(core_root.join("state").join("destructive_quorum.json"))
            .unwrap_or_else(|| json!({"enabled": true}));
    let interrupt_authority =
        read_json_file(core_root.join("state").join("interrupt_authority.json"))
            .unwrap_or_else(|| json!({"default":{"allow":[]}}));
    let permission_audit = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("warden")
            .join("permission_profile_audit.jsonl"),
        32,
    );
    let escalations = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("prometheus")
            .join("escalations.jsonl"),
        64,
    );
    let reroute_metrics = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("reroute_metrics.jsonl"),
        32,
    );
    let reroute_acks = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("reroute_acks.jsonl"),
        32,
    );

    let active_profile_id = permission_profiles
        .get("active_profile")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let active_profile = permission_profiles
        .get("profiles")
        .and_then(Value::as_object)
        .and_then(|profiles| profiles.get(active_profile_id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let profile_expires_at = active_profile.get("expires_at_utc").and_then(Value::as_str);
    let network_scope_expires_at = active_profile
        .get("scopes")
        .and_then(|v| v.get("network"))
        .and_then(|v| v.get("expires_at_utc"))
        .and_then(Value::as_str);
    let destructive_scope_expires_at = active_profile
        .get("scopes")
        .and_then(|v| v.get("destructive"))
        .and_then(|v| v.get("expires_at_utc"))
        .and_then(Value::as_str);
    let policy_guard_denials = escalations
        .iter()
        .filter(|value| {
            value
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| reason == "policy_guard.denied")
        })
        .cloned()
        .collect::<Vec<_>>();
    let interrupt_denials = escalations
        .iter()
        .filter(|value| {
            value
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| reason == "interrupt_authority_policy.denied")
        })
        .cloned()
        .collect::<Vec<_>>();
    let quorum_escalations = escalations
        .iter()
        .filter(|value| {
            value
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| reason.contains("destructive") || reason.contains("quorum"))
        })
        .cloned()
        .collect::<Vec<_>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "warden_policy_projection",
        "permission_profile": {
            "active_profile": active_profile_id,
            "configured": active_profile != json!({}),
            "expired": is_expired_rfc3339(profile_expires_at),
            "network_scope_expired": is_expired_rfc3339(network_scope_expires_at),
            "destructive_scope_expired": is_expired_rfc3339(destructive_scope_expires_at),
            "state": active_profile,
            "recent_audit": {
                "count": permission_audit.len(),
                "allow_count": count_bool_field(&permission_audit, "allowed", true),
                "deny_count": count_bool_field(&permission_audit, "allowed", false),
                "recent_records": permission_audit
            }
        },
        "destructive_quorum": {
            "policy": destructive_quorum,
            "recent_related_escalations": quorum_escalations
        },
        "interrupt_authority": {
            "policy": interrupt_authority,
            "recent_policy_denials": interrupt_denials,
            "recent_reroute_metrics": reroute_metrics,
            "recent_reroute_acks": reroute_acks
        },
        "policy_guard": {
            "recent_denials": policy_guard_denials
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_warden_edge_contract(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("warden_edge_contract.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let queue_path = std::env::var("ARDA_WARDEN_QUEUE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/warden/informant_queue.jsonl"));
    let queue_path = if queue_path.is_relative() {
        workspace_root.join(queue_path)
    } else {
        queue_path
    };
    let queue_events = read_recent_jsonl(&queue_path, 128);
    let unsynced_events = queue_events
        .iter()
        .filter(|value| value.get("synced").and_then(Value::as_bool) == Some(false))
        .cloned()
        .collect::<Vec<_>>();
    let synced_events = queue_events
        .iter()
        .filter(|value| value.get("synced").and_then(Value::as_bool) == Some(true))
        .count();
    let edge_scan = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("fleet_control_last.json"),
    )
    .unwrap_or_else(|| json!({"status":"unknown","network":{"tailscale_ok":false}}));
    let local_informant = read_json_file(
        workspace_root
            .join("data")
            .join("fleet")
            .join("informants")
            .join("local_last.json"),
    )
    .unwrap_or_else(|| json!({"tailscale_ok":false,"ollama_ok":false}));
    let targets = read_edge_targets(
        &workspace_root
            .join("core")
            .join("edge")
            .join("targets.toml"),
    )
    .or_else(|| {
        read_edge_targets(
            &workspace_root
                .join("core")
                .join("edge")
                .join("targets.example.toml"),
        )
    })
    .unwrap_or_default();

    let tailscale_mesh_ok = edge_scan
        .get("network")
        .and_then(|value| value.get("tailscale_ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let local_probe_ok = local_informant
        .get("tailscale_ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ack_mode = if synced_events > 0 {
        "queue_with_sync_markers"
    } else {
        "queue_only_no_edge_ack"
    };

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "warden_edge_projection",
        "edge_contract": {
            "mode": "informant_queue_over_tailscale",
            "ack_mode": ack_mode,
            "queue_path": queue_path.display().to_string(),
            "recent_queue_events": queue_events.len(),
            "recent_synced_events": synced_events,
            "recent_unsynced_events": unsynced_events.len(),
            "recent_unsynced_sample": unsynced_events.into_iter().rev().take(10).collect::<Vec<_>>()
        },
        "mesh": {
            "fleet_scan": edge_scan,
            "local_informant_probe": local_informant,
            "tailscale_mesh_ok": tailscale_mesh_ok,
            "local_probe_ok": local_probe_ok,
            "edge_ready": tailscale_mesh_ok,
            "ack_gap_present": synced_events == 0
        },
        "inventory": {
            "configured_targets_count": targets.len(),
            "targets": targets
        },
        "contract_findings": [
            if tailscale_mesh_ok {
                "mesh_reachable_from_fleet_scan"
            } else {
                "mesh_not_verified"
            },
            if local_probe_ok {
                "local_informant_probe_ok"
            } else {
                "local_informant_probe_blocked_or_failed"
            },
            if synced_events == 0 {
                "no_end_to_end_edge_ack_visible"
            } else {
                "edge_ack_markers_visible"
            }
        ]
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_warden_nightly_doctrine(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("warden_nightly_doctrine.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let boot_nightly = read_boot_nightly(&core_root.join("realm").join("boot.toml"));
    let hades_status = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("hades")
            .join("status.json"),
    )
    .unwrap_or_else(|| json!({}));
    let maintenance = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("maintenance_last.json"),
    )
    .unwrap_or_else(|| json!({}));
    let drift_last = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("drift_report_last.json"),
    )
    .unwrap_or_else(|| json!({}));
    let pressure_guard = read_json_file(
        workspace_root
            .join("core")
            .join("state")
            .join("runtime_admission_pressure.json"),
    )
    .unwrap_or_else(|| json!({}));
    let audit_latest = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("audit_latest.json"),
    )
    .unwrap_or_else(|| json!({}));

    let declared = json!({
        "run_at": boot_nightly
            .as_ref()
            .and_then(|n| n.run_at.clone())
            .unwrap_or_else(|| "03:00".to_string()),
        "enabled": boot_nightly.as_ref().and_then(|n| n.enabled).unwrap_or(true),
        "archive_complete_after_days": boot_nightly
            .as_ref()
            .and_then(|n| n.archive_complete_after_days)
            .unwrap_or(7),
        "prune_low_resonance": boot_nightly
            .as_ref()
            .and_then(|n| n.prune_low_resonance)
            .unwrap_or(true),
        "min_resonance_threshold": boot_nightly
            .as_ref()
            .and_then(|n| n.min_resonance_threshold)
            .unwrap_or(25.0),
        "compact_ledger": boot_nightly.as_ref().and_then(|n| n.compact_ledger).unwrap_or(true),
        "emit_daily_summary": boot_nightly
            .as_ref()
            .and_then(|n| n.emit_daily_summary)
            .unwrap_or(true),
        "summary_path": boot_nightly
            .as_ref()
            .and_then(|n| n.summary_path.clone())
            .unwrap_or_else(|| "data/summaries/".to_string()),
        "expected_tasks": [
            "archive_old_tasks",
            "prune_soterion_index",
            "compact_ledger",
            "emit_health_report",
            "recalculate_trust_scores",
            "scan_for_drift"
        ]
    });

    let implemented = json!({
        "hades_scheduler": {
            "owner": "hades",
            "implements": ["scheduled_sweeps", "sigil_cleanup", "repair_detected_handoffs"],
            "status": hades_status
        },
        "ceo_maintenance": {
            "owner": "prometheus",
            "implements": [
                "compaction",
                "metrics_export",
                "gate_metrics_export",
                "pressure_guard",
                "handoff_slo_guard",
                "autonomy_budget_guard",
                "health_workflow_router"
            ],
            "status": maintenance
        },
        "drift_detection": {
            "owner": "prometheus",
            "implements": ["tracked_runtime_file_drift", "auto_open_reconcile_tasks"],
            "status": drift_last
        },
        "metrics_audit": {
            "owner": "core_metrics",
            "implements": ["audit_summary", "oversize_file_detection", "parse_validation"],
            "status": audit_latest
        },
        "pressure_guard": {
            "owner": "prometheus",
            "implements": ["disk_pressure", "oversize_files", "queue_pressure"],
            "status": pressure_guard
        }
    });

    let role_map = json!([
        {
            "duty": "archive_old_tasks",
            "declared_owner": "warden",
            "implemented_owner": "hades_and_prometheus_maintenance",
            "status": "distributed"
        },
        {
            "duty": "prune_soterion_index",
            "declared_owner": "warden",
            "implemented_owner": "hades",
            "status": "partial"
        },
        {
            "duty": "compact_ledger",
            "declared_owner": "warden",
            "implemented_owner": "prometheus_maintenance",
            "status": "implemented_elsewhere"
        },
        {
            "duty": "emit_health_report",
            "declared_owner": "warden",
            "implemented_owner": "metrics_export_and_health_workflow_router",
            "status": "distributed"
        },
        {
            "duty": "recalculate_trust_scores",
            "declared_owner": "warden",
            "implemented_owner": "not_materialized_as_dedicated_runtime_step",
            "status": "missing"
        },
        {
            "duty": "scan_for_drift",
            "declared_owner": "warden",
            "implemented_owner": "prometheus_drift_detector",
            "status": "implemented_elsewhere"
        }
    ]);

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "warden_nightly_projection",
        "declared_doctrine": declared,
        "implemented_runtime": implemented,
        "ownership_map": role_map,
        "conclusion": {
            "model": "warden_supervises_distributed_maintenance_plane",
            "gap": "warden_doctrine_exceeds_dedicated_runtime_implementation",
            "next_step": "promote_missing_trust_score_and_explicit_daily_health_summary_steps into first-class exported jobs or daemon behavior"
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn read_boot_nightly(path: &Path) -> Option<BootNightlyConfig> {
    let content = fs::read_to_string(path).ok()?;
    toml::from_str::<BootNightlyFile>(&content).ok()?.nightly
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir");
        }
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn warden_policy_projection_surfaces_expiry_and_denial_buckets() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state dir");

        write_file(
            &core_root.join("state/permission_profiles.json"),
            r#"{
  "active_profile": "restricted",
  "profiles": {
    "restricted": {
      "expires_at_utc": "2000-01-01T00:00:00Z",
      "scopes": {
        "network": { "expires_at_utc": "2000-01-01T00:00:00Z" },
        "destructive": { "expires_at_utc": "2099-01-01T00:00:00Z" }
      }
    }
  }
}"#,
        );
        write_file(
            &core_root.join("state/destructive_quorum.json"),
            r#"{ "enabled": true, "minimum_reviewers": 2 }"#,
        );
        write_file(
            &core_root.join("state/interrupt_authority.json"),
            r#"{ "default": { "allow": ["prometheus"] } }"#,
        );
        write_file(
            &dir.path()
                .join("data/warden/permission_profile_audit.jsonl"),
            "{\"allowed\":true}\n{\"allowed\":false}\n",
        );
        write_file(
            &dir.path().join("data/prometheus/escalations.jsonl"),
            "{\"reason\":\"policy_guard.denied\",\"task_id\":\"t1\"}\n\
             {\"reason\":\"interrupt_authority_policy.denied\",\"task_id\":\"t2\"}\n\
             {\"reason\":\"destructive.quorum_required\",\"task_id\":\"t3\"}\n",
        );
        write_file(
            &dir.path().join("data/hermes/reroute_metrics.jsonl"),
            "{\"task_id\":\"t2\",\"reroute\":1}\n",
        );
        write_file(
            &dir.path().join("data/hermes/reroute_acks.jsonl"),
            "{\"task_id\":\"t2\",\"ack\":true}\n",
        );

        write_warden_policy_authority(&core_root);

        let projection: Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_policy_authority.json"))
                .expect("read projection"),
        )
        .expect("projection json");

        assert_eq!(projection["authority"], "warden_policy_projection");
        assert_eq!(
            projection["permission_profile"]["active_profile"],
            "restricted"
        );
        assert_eq!(projection["permission_profile"]["configured"], true);
        assert_eq!(projection["permission_profile"]["expired"], true);
        assert_eq!(
            projection["permission_profile"]["network_scope_expired"],
            true
        );
        assert_eq!(
            projection["permission_profile"]["destructive_scope_expired"],
            false
        );
        assert_eq!(
            projection["permission_profile"]["recent_audit"]["allow_count"],
            1
        );
        assert_eq!(
            projection["permission_profile"]["recent_audit"]["deny_count"],
            1
        );
        assert_eq!(
            projection["policy_guard"]["recent_denials"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            projection["interrupt_authority"]["recent_policy_denials"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            projection["destructive_quorum"]["recent_related_escalations"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn warden_edge_projection_reports_ack_gap_and_unsynced_sample() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state dir");

        write_file(
            &dir.path().join("data/warden/informant_queue.jsonl"),
            "{\"synced\":false,\"event_type\":\"heartbeat\",\"id\":\"a1\"}\n\
             {\"synced\":false,\"event_type\":\"alert\",\"id\":\"a2\"}\n",
        );
        write_file(
            &dir.path().join("data/prometheus/fleet_control_last.json"),
            r#"{
  "status": "degraded",
  "network": { "tailscale_ok": false }
}"#,
        );
        write_file(
            &dir.path().join("data/fleet/informants/local_last.json"),
            r#"{ "tailscale_ok": true, "ollama_ok": true }"#,
        );
        write_file(
            &core_root.join("edge/targets.toml"),
            r#"
[[node]]
id = "node-pi5-warden"
role = "warden_guardhouse"
node_class = "edge_guardhouse"
"#,
        );

        write_warden_edge_contract(&core_root);

        let projection: Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_edge_contract.json"))
                .expect("read projection"),
        )
        .expect("projection json");

        assert_eq!(projection["authority"], "warden_edge_projection");
        assert_eq!(
            projection["edge_contract"]["ack_mode"],
            "queue_only_no_edge_ack"
        );
        assert_eq!(projection["edge_contract"]["recent_queue_events"], 2);
        assert_eq!(projection["edge_contract"]["recent_synced_events"], 0);
        assert_eq!(projection["edge_contract"]["recent_unsynced_events"], 2);
        assert_eq!(projection["mesh"]["tailscale_mesh_ok"], false);
        assert_eq!(projection["mesh"]["local_probe_ok"], true);
        assert_eq!(projection["mesh"]["ack_gap_present"], true);
        assert_eq!(projection["inventory"]["configured_targets_count"], 1);
        assert_eq!(
            projection["contract_findings"][2],
            "no_end_to_end_edge_ack_visible"
        );
    }
}
