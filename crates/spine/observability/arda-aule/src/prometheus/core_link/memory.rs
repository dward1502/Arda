#![cfg(feature = "full-cli")]
use super::{
    read_json_file, read_recent_jsonl, read_recent_mnemosyne_episodic, summarize_field_counts,
    CORE_STATE_SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn write_memory_identity_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("memory_identity.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let continuity = read_json_file(core_root.join("state").join("mnemosyne_continuity.json"))
        .unwrap_or_else(|| json!({}));
    let stats = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("mnemosyne")
            .join("stats.json"),
    )
    .unwrap_or_else(|| json!({}));

    let recent_memories = continuity
        .get("recent_activity")
        .and_then(|value| value.get("memories"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let memory_counts = stats
        .get("status")
        .and_then(|value| value.get("memory_counts"))
        .cloned()
        .or_else(|| stats.get("memory_counts").cloned())
        .unwrap_or_else(|| json!({}));

    let mission_focus = recent_memories
        .first()
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("No recent memory focus.");

    let priority_tags = recent_memories
        .iter()
        .flat_map(|value| {
            value
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|value| value.as_str().map(|s| s.to_string()))
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut acc, tag| {
                *acc.entry(tag).or_default() += 1;
                acc
            },
        )
        .into_iter()
        .collect::<Vec<_>>();

    let top_priority_tags = priority_tags
        .into_iter()
        .rev()
        .take(6)
        .map(|(tag, count)| json!({"tag": tag, "count": count}))
        .collect::<Vec<_>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "memory_identity_projection",
        "identity": {
            "current_mission_focus": mission_focus,
            "memory_counts": memory_counts,
            "top_priority_tags": top_priority_tags,
        },
        "derived_state": {
            "identity_ready": !recent_memories.is_empty(),
            "recent_focus_present": !recent_memories.is_empty(),
            "core_memory_count": memory_counts.get("core").and_then(Value::as_u64).unwrap_or(0),
            "active_memory_count": memory_counts.get("active").and_then(Value::as_u64).unwrap_or(0),
        },
        "arda_hints": {
            "primary_panel": "identity_continuity",
            "boardroom_section": "identity_and_growth",
            "highlight_focus": mission_focus != "No recent memory focus."
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_memory_activity_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("memory_activity.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mnemosyne_root = workspace_root.join("data").join("mnemosyne");
    let recent_memories = read_recent_mnemosyne_episodic(&mnemosyne_root.join("episodic"), 24);
    let noise = read_recent_jsonl(&mnemosyne_root.join("noise.jsonl"), 24);
    let obsidian = read_recent_jsonl(&mnemosyne_root.join("obsidian_index.jsonl"), 24);

    let event_type_counts = summarize_field_counts(&recent_memories, "event_type");
    let source_crate_counts = summarize_field_counts(&recent_memories, "source_crate");
    let scope_counts = summarize_field_counts(&recent_memories, "memory_scope");
    let high_significance_memories = recent_memories
        .iter()
        .filter(|entry| {
            entry
                .get("significance")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= 0.8
        })
        .count();
    let most_recent_ts = recent_memories
        .iter()
        .filter_map(|entry| entry.get("ts_utc").and_then(Value::as_str))
        .max()
        .map(|s| s.to_string());
    let most_recent_age_minutes = most_recent_ts
        .as_deref()
        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_minutes().max(0))
        .unwrap_or(9999);

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "memory_activity_projection",
        "summary": {
            "recent_memory_count": recent_memories.len(),
            "high_significance_memories": high_significance_memories,
            "noise_events": noise.len(),
            "obsidian_entries": obsidian.len(),
            "most_recent_memory_ts": most_recent_ts,
            "most_recent_memory_age_minutes": most_recent_age_minutes
        },
        "distributions": {
            "event_types": event_type_counts,
            "source_crates": source_crate_counts,
            "memory_scopes": scope_counts
        },
        "recent_activity": {
            "memories": recent_memories,
            "noise_events": noise,
            "obsidian_bridge": obsidian
        },
        "arda_hints": {
            "primary_panel": "memory_activity",
            "boardroom_section": "identity_and_growth",
            "alert_on_memory_quiet": most_recent_age_minutes >= 180
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_memory_scopes_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("memory_scopes.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mnemosyne_root = workspace_root.join("data").join("mnemosyne");
    let recent_memories = read_recent_mnemosyne_episodic(&mnemosyne_root.join("episodic"), 36);

    let scopes = [
        "system_continuity",
        "boardroom_council",
        "human_context",
        "edge_runtime",
    ]
    .into_iter()
    .map(|scope| {
        let entries = recent_memories
            .iter()
            .filter(|entry| entry.get("memory_scope").and_then(Value::as_str) == Some(scope))
            .cloned()
            .collect::<Vec<_>>();
        let most_recent_ts = entries
            .iter()
            .filter_map(|entry| entry.get("ts_utc").and_then(Value::as_str))
            .max()
            .map(|s| s.to_string());
        let most_recent_age_minutes = most_recent_ts
            .as_deref()
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_minutes().max(0))
            .unwrap_or(9999);

        (
            scope.to_string(),
            json!({
                "recent_count": entries.len(),
                "most_recent_ts": most_recent_ts,
                "most_recent_age_minutes": most_recent_age_minutes,
                "event_types": summarize_field_counts(&entries, "event_type"),
                "source_crates": summarize_field_counts(&entries, "source_crate"),
                "recent_memories": entries.into_iter().take(8).collect::<Vec<_>>()
            }),
        )
    })
    .collect::<serde_json::Map<String, Value>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "memory_scopes_projection",
        "scopes": scopes,
        "arda_hints": {
            "primary_panel": "memory_scope_map",
            "boardroom_section": "identity_and_growth",
            "scope_count": 4
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_mnemosyne_continuity_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("mnemosyne_continuity.json");
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
            .join("mnemosyne")
            .join("status.json"),
    )
    .unwrap_or_else(|| json!({}));
    let stats = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("mnemosyne")
            .join("stats.json"),
    )
    .unwrap_or_else(|| json!({}));
    let mnemosyne_root = workspace_root.join("data").join("mnemosyne");
    let chain_head = fs::read_to_string(mnemosyne_root.join("chain_head"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let last_consolidation_utc = fs::read_to_string(mnemosyne_root.join("last_consolidation_utc"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let recent_memories = read_recent_mnemosyne_episodic(&mnemosyne_root.join("episodic"), 12);
    let noise = read_recent_jsonl(&mnemosyne_root.join("noise.jsonl"), 12);
    let obsidian = read_recent_jsonl(&mnemosyne_root.join("obsidian_index.jsonl"), 12);
    let high_significance = recent_memories
        .iter()
        .filter(|entry| {
            entry
                .get("significance")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= 0.8
        })
        .count();
    let recent_memory_count = recent_memories.len();
    let noise_count = noise.len();
    let obsidian_count = obsidian.len();
    let last_consolidation_dt = if last_consolidation_utc.is_empty() {
        None
    } else {
        DateTime::parse_from_rfc3339(&last_consolidation_utc)
            .ok()
            .map(|v| v.with_timezone(&Utc))
    };
    let consolidation_age_hours = last_consolidation_dt
        .map(|dt| (Utc::now() - dt).num_hours())
        .unwrap_or(999);
    let continuity_drought = recent_memory_count == 0;
    let consolidation_stale = consolidation_age_hours >= 48;
    let noise_dominant = noise_count > 0 && recent_memory_count == 0;
    let continuity_pressure = if continuity_drought && consolidation_stale {
        "high"
    } else if continuity_drought || consolidation_stale || noise_dominant {
        "medium"
    } else {
        "low"
    };
    let recommended_action = if continuity_drought && consolidation_stale {
        "run mnemosyne consolidate and re-seed continuity checkpoints"
    } else if consolidation_stale {
        "run mnemosyne consolidate"
    } else if noise_dominant {
        "improve high-signal memory capture and reduce noise dominance"
    } else if recent_memory_count < 3 {
        "encourage checkpoint-worthy captures for active work"
    } else {
        "memory posture healthy"
    };

    let snapshot = json!({
        "schema_version": "arda.mnemosyne.continuity.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "mnemosyne_continuity_projection",
        "status": status,
        "stats": stats,
        "continuity": {
            "chain_head": chain_head,
            "chain_head_present": !chain_head.is_empty(),
            "last_consolidation_utc": if last_consolidation_utc.is_empty() { Value::Null } else { Value::String(last_consolidation_utc) },
            "consolidation_age_hours": consolidation_age_hours,
            "consolidation_stale": consolidation_stale
        },
        "recent_activity": {
            "memories": recent_memories,
            "noise_events": noise,
            "obsidian_bridge": obsidian,
            "counts": {
                "recent_memory_count": recent_memory_count,
                "high_significance_memories": high_significance,
                "noise_events": noise_count,
                "obsidian_entries": obsidian_count
            }
        },
        "health": {
            "continuity_pressure": continuity_pressure,
            "continuity_drought": continuity_drought,
            "noise_dominant": noise_dominant,
            "recommended_action": recommended_action
        },
        "arda_hints": {
            "primary_panel": "memory_continuity",
            "boardroom_section": "identity_and_growth",
            "alert_on_chain_integrity": chain_head.is_empty(),
            "alert_on_memory_drought": continuity_drought,
            "alert_on_consolidation_stale": consolidation_stale
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
