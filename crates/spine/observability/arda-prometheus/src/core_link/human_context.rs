use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    collect_json_file_summaries_recursive, collect_markdown_file_summaries,
    collect_markdown_file_summaries_recursive, count_files_with_extension, read_json_file,
    read_toml_as_json, summarize_markdown_file, CORE_STATE_SCHEMA_VERSION,
};

pub(super) fn write_human_context_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("human_context.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let human_root = workspace_root.join("human");
    let notes_root = human_root.join("Notes");
    let docs_root = human_root.join("docs");
    let arandur_root = human_root.join("arandur");
    let summaries_root = human_root.join("summaries");
    let library_root = human_root.join("library");
    let business_config = read_toml_as_json(workspace_root.join("config").join("business.toml"))
        .unwrap_or_else(|| json!({}));
    let personal_identity =
        read_toml_as_json(core_root.join("personal").join("personal-identity.toml"))
            .unwrap_or_else(|| json!({}));
    let business_state =
        read_json_file(workspace_root.join("data/business/soterion-business.json"))
            .unwrap_or_else(|| json!({}));
    let personal_state =
        read_json_file(workspace_root.join("data/personal/soterion-personal.json"))
            .unwrap_or_else(|| json!({}));
    let top_level_human = collect_markdown_file_summaries(&human_root, 24);
    let notes = collect_markdown_file_summaries(&notes_root, 16);
    let docs = collect_markdown_file_summaries(&docs_root, 16);
    let summaries = collect_markdown_file_summaries_recursive(&summaries_root, 24);
    let library_docs = collect_markdown_file_summaries_recursive(&library_root, 24);
    let business_clients =
        collect_json_file_summaries_recursive(&workspace_root.join("data/business/clients"), 24);
    let personal_docs =
        collect_markdown_file_summaries_recursive(&workspace_root.join("data/personal"), 16);

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "human_context_projection",
        "human_portal": {
            "index": summarize_markdown_file(&human_root.join("index.md")),
            "onboard": summarize_markdown_file(&human_root.join("onboard.md")),
            "company_view": summarize_markdown_file(&human_root.join("company_view.md")),
            "top_level_readables": top_level_human,
            "docs": docs,
            "notes": notes,
            "summaries": summaries,
            "library": library_docs,
            "arandur": {
                "index": summarize_markdown_file(&arandur_root.join("README.md")),
                "thoughts": summarize_markdown_file(&arandur_root.join("thoughts.md"))
            },
            "counts": {
                "notes_total": count_files_with_extension(&notes_root, "md"),
                "docs_total": count_files_with_extension(&docs_root, "md"),
                "summaries_total": count_files_with_extension(&summaries_root, "md"),
                "library_docs_total": count_files_with_extension(&library_root, "md"),
                "arandur_docs_total": count_files_with_extension(&arandur_root, "md")
            }
        },
        "business": {
            "config": business_config,
            "state": business_state,
            "clients": business_clients,
            "counts": {
                "client_records_total": business_clients.len()
            }
        },
        "personal": {
            "identity": personal_identity,
            "state": personal_state,
            "documents": personal_docs,
            "counts": {
                "personal_docs_total": personal_docs.len()
            }
        },
        "arda_hints": {
            "primary_panel": "human_and_growth",
            "boardroom_section": "operator_context",
            "alert_on_missing_onboard": summarize_markdown_file(&human_root.join("onboard.md"))["body_preview"]
                .as_str()
                .unwrap_or_default()
                .is_empty(),
            "alert_on_sparse_company_view": summarize_markdown_file(&human_root.join("company_view.md"))["body_preview"]
                .as_str()
                .unwrap_or_default()
                .is_empty(),
            "alert_on_missing_arandur_thoughts": summarize_markdown_file(&arandur_root.join("thoughts.md"))["body_preview"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_business_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("business_runtime.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let business_config = read_toml_as_json(workspace_root.join("config").join("business.toml"))
        .unwrap_or_else(|| json!({}));
    let business_state =
        read_json_file(workspace_root.join("data/business/soterion-business.json"))
            .unwrap_or_else(|| json!({}));
    let business_clients =
        collect_json_file_summaries_recursive(&workspace_root.join("data/business/clients"), 24);
    let company_view =
        summarize_markdown_file(&workspace_root.join("human").join("company_view.md"));

    let mode = business_config
        .get("business")
        .and_then(|value| value.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let state_keys = business_state
        .as_object()
        .map(|map| map.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "business_runtime_projection",
        "mode": mode,
        "company_view": company_view,
        "config": business_config,
        "state": business_state,
        "client_records": business_clients,
        "counts": {
            "client_records_total": business_clients.len(),
            "state_keys_total": state_keys.len(),
        },
        "highlights": {
            "client_paths": business_clients
                .iter()
                .filter_map(|entry| entry.get("path").and_then(Value::as_str))
                .take(4)
                .collect::<Vec<_>>(),
            "state_keys": state_keys.into_iter().take(6).collect::<Vec<_>>(),
        }
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_personal_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("personal_runtime.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let personal_identity =
        read_toml_as_json(core_root.join("personal").join("personal-identity.toml"))
            .unwrap_or_else(|| json!({}));
    let personal_state =
        read_json_file(workspace_root.join("data/personal/soterion-personal.json"))
            .unwrap_or_else(|| json!({}));
    let personal_docs =
        collect_markdown_file_summaries_recursive(&workspace_root.join("data/personal"), 16);
    let onboard = summarize_markdown_file(&workspace_root.join("human").join("onboard.md"));
    let notes_readme =
        summarize_markdown_file(&workspace_root.join("human").join("Notes").join("README.md"));

    let identity = personal_identity
        .get("identity")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let values = identity.get("values").cloned().unwrap_or_else(|| json!({}));
    let time = identity.get("time").cloned().unwrap_or_else(|| json!({}));

    let research_domains = personal_identity
        .get("research_domains")
        .and_then(|value| value.get("active"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let creative_domains = personal_identity
        .get("creative_domains")
        .and_then(|value| value.get("active"))
        .cloned()
        .unwrap_or_else(|| json!([]));

    let state_keys = personal_state
        .as_object()
        .map(|map| map.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "personal_runtime_projection",
        "identity": identity,
        "values": values,
        "time": time,
        "research_domains": research_domains,
        "creative_domains": creative_domains,
        "state": personal_state,
        "documents": personal_docs,
        "onboard": onboard,
        "notes_readme": notes_readme,
        "counts": {
            "personal_docs_total": personal_docs.len(),
            "research_domains_total": research_domains.as_array().map(|items| items.len()).unwrap_or(0),
            "creative_domains_total": creative_domains.as_array().map(|items| items.len()).unwrap_or(0),
            "state_keys_total": state_keys.len()
        },
        "highlights": {
            "priorities": vec![
                time.get("priority_1").and_then(Value::as_str).unwrap_or_default(),
                time.get("priority_2").and_then(Value::as_str).unwrap_or_default(),
                time.get("priority_3").and_then(Value::as_str).unwrap_or_default()
            ].into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>(),
            "values": values
                .as_object()
                .map(|map| map.keys().take(6).cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
            "state_keys": state_keys.into_iter().take(6).collect::<Vec<_>>()
        }
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
