use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::*;

pub(crate) fn export_numenor_prime_merge_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let numenor_root = numenor_prime_root();
    let out = root.join("core/state/numenor_prime_merge_registry.json");
    let top_roles = BTreeMap::from([
        ("Elros", "historical_human_corpus"),
        ("Operations", "historical_operations_ledger"),
        ("Knowledge", "historical_knowledge_corpus"),
        ("Documents", "historical_docs_and_governance"),
        ("ATHENA", "legacy_agent_system"),
        ("Agents", "legacy_agent_definitions"),
        ("CodeVault", "patterns_and_reference_snippets"),
        ("Projects", "project_codebases"),
        ("Eregion", "active_code_forge"),
        ("Tools", "tooling_and_daemons"),
        ("Systems", "legacy_system_namespaces"),
        ("Valinor", "archive_or_refactor_corpus"),
        ("_HQ", "crosslink_and_reports"),
    ]);
    let ignore = BTreeSet::from([
        ".git",
        ".cache",
        ".ruff_cache",
        ".plans",
        ".ralph",
        ".sisyphus",
    ]);
    let manifests = [
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "deno.json",
        "deno.jsonc",
        "tauri.conf.json",
        "docker-compose.yml",
    ];
    let mut top = Vec::new();
    if numenor_root.exists() {
        for child in fs::read_dir(&numenor_root)? {
            let child = child?.path();
            let name = child
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            if name.starts_with('.') || ignore.contains(name) {
                continue;
            }
            let role = top_roles
                .get(name)
                .copied()
                .unwrap_or("unclassified_external_surface");
            let lane = match role {
                "historical_human_corpus"
                | "historical_operations_ledger"
                | "historical_knowledge_corpus"
                | "historical_docs_and_governance"
                | "legacy_agent_system"
                | "legacy_agent_definitions"
                | "patterns_and_reference_snippets"
                | "crosslink_and_reports" => "reference_then_selective_promotion",
                "project_codebases" | "active_code_forge" | "tooling_and_daemons" => {
                    "inventory_then_project_level_review"
                }
                "archive_or_refactor_corpus" => "cold_archive_or_later_review",
                _ => "manual_review",
            };
            top.push(json!({
                "name": name,
                "path": child.display().to_string(),
                "exists": child.exists(),
                "role": role,
                "merge_lane": lane,
                "size_bytes": if child.is_dir() { dir_size_for_registry(&child, 6) } else { child.metadata().map(|m| m.len()).unwrap_or(0) },
            }));
        }
    }
    let mut projects = Vec::new();
    for project_root in [
        numenor_root.join("Eregion/KhazadForge"),
        numenor_root.join("Projects"),
    ] {
        if !project_root.exists() {
            continue;
        }
        for child in fs::read_dir(&project_root)? {
            let child = child?.path();
            if !child.is_dir() {
                continue;
            }
            let name = child
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            let manifests_found = manifests
                .iter()
                .filter_map(|m| {
                    if child.join(m).exists() {
                        Some((*m).to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            projects.push(json!({
                "name": name,
                "path": child.display().to_string(),
                "parent": project_root.display().to_string(),
                "size_bytes": dir_size_for_registry(&child, 5),
                "manifests": manifests_found,
                "project_type": if manifests_found.is_empty() { "workspace_or_collection" } else { "codebase" },
                "recommended_lane": if matches!(name, "skylightpros" | "ARDA_HUD" | "nanoclaw") { "active_delivery_or_runtime_review" } else { "catalog_then_selective_absorption" },
            }));
        }
    }
    let payload = json!({
        "schema_version": "annunimas.numenor-prime-merge-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "numenor_prime_external_intake_registry",
        "source_root": numenor_root.display().to_string(),
        "status": "inventory_ready_for_controlled_absorption",
        "summary": {
            "top_level_surfaces_total": top.len(),
            "project_codebases_total": projects.len(),
            "historical_corpora_total": top.iter().filter(|row| row["merge_lane"] == "reference_then_selective_promotion").count(),
            "project_review_total": top.iter().filter(|row| row["merge_lane"] == "inventory_then_project_level_review").count(),
        },
        "top_level_surfaces": top,
        "project_codebases": projects,
    });
    write_pretty_json(&out, &payload)?;
    Ok(json!({ "out": rel(&out, &root) }))
}

pub(crate) fn export_valinor_merge_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let valinor_root_dir = valinor_root();
    let out = root.join("core/state/valinor_merge_registry.json");
    let roles = BTreeMap::from([
        ("ACTIVE_INDEX.yaml", "incubation_lane_registry"),
        ("INDEX.md", "valinor_doctrine"),
        ("INDEX.yaml", "valinor_root_index"),
        ("DECISIONS_2026-02-14.md", "strategic_decision_log"),
        ("EXECUTION_BOARD_2026-02-14.md", "execution_board"),
        ("LANE_RUN_QUEUE.yaml", "lane_run_queue"),
        (
            "FUTURE_TOOLS_AND_AGENTS_NOTES_2026-02.md",
            "tooling_and_agent_scouting",
        ),
        (
            "Realmgate_Warriors_Task_Plan.txt",
            "project_specific_task_memory",
        ),
        ("CRUSTIES", "canonical_project_doc_suite"),
        ("INFRA", "deployment_pattern_corpus"),
        ("OPPORTUNITIES", "opportunity_corpus"),
        ("archive", "archived_opportunity_and_platform_memory"),
        ("refactor", "prototype_and_refactor_memory"),
    ]);
    let lanes = BTreeMap::from([
        ("incubation_lane_registry", "reference_then_curated_summary"),
        ("valinor_doctrine", "reference_then_curated_summary"),
        ("valinor_root_index", "reference_then_curated_summary"),
        ("strategic_decision_log", "reference_then_curated_summary"),
        ("execution_board", "reference_then_curated_summary"),
        ("lane_run_queue", "historical_process_memory"),
        (
            "tooling_and_agent_scouting",
            "reference_then_selective_promotion",
        ),
        ("project_specific_task_memory", "historical_project_memory"),
        ("canonical_project_doc_suite", "project_doc_promotion"),
        ("deployment_pattern_corpus", "infra_pattern_review"),
        ("opportunity_corpus", "opportunity_summary_then_selection"),
        (
            "archived_opportunity_and_platform_memory",
            "archive_reference_then_project_extraction",
        ),
        (
            "prototype_and_refactor_memory",
            "prototype_pattern_extraction",
        ),
    ]);
    let manifests = [
        "package.json",
        "Cargo.toml",
        "docker-compose.yml",
        "Dockerfile",
        "requirements.txt",
        "traefik.yaml",
    ];
    let mut top = Vec::new();
    if valinor_root_dir.exists() {
        for child in fs::read_dir(&valinor_root_dir)? {
            let child = child?.path();
            let name = child
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            if name.starts_with('.') || matches!(name, ".git" | ".cache" | "__pycache__") {
                continue;
            }
            let role = roles
                .get(name)
                .copied()
                .unwrap_or("unclassified_valinor_surface");
            top.push(json!({
                "name": name,
                "path": child.display().to_string(),
                "exists": child.exists(),
                "role": role,
                "merge_lane": lanes.get(role).copied().unwrap_or("manual_review"),
                "size_bytes": if child.is_dir() { dir_size_for_registry(&child, 6) } else { child.metadata().map(|m| m.len()).unwrap_or(0) },
            }));
        }
    }
    let mut infra = Vec::new();
    let infra_root = valinor_root().join("INFRA/deployments");
    if infra_root.exists() {
        for child in fs::read_dir(&infra_root)? {
            let child = child?.path();
            if child.is_dir() {
                infra.push(json!({
                    "name": child.file_name().and_then(|v| v.to_str()).unwrap_or_default(),
                    "path": child.display().to_string(),
                    "manifests": manifests.iter().filter_map(|m| if child.join(m).exists() { Some((*m).to_string()) } else { None }).collect::<Vec<_>>(),
                    "size_bytes": dir_size_for_registry(&child, 4),
                    "recommended_lane": "infra_pattern_review",
                }));
            }
        }
    }
    let payload = json!({
        "schema_version": "annunimas.valinor-merge-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "valinor_external_intake_registry",
        "source_root": valinor_root().display().to_string(),
        "status": "inventory_ready_for_controlled_absorption",
        "summary": {
            "top_level_surfaces_total": top.len(),
            "infra_projects_total": infra.len(),
        },
        "top_level_surfaces": top,
        "infra_projects": infra,
    });
    write_pretty_json(&out, &payload)?;
    Ok(json!({ "out": rel(&out, &root) }))
}

fn dir_size_for_registry(path: &Path, max_depth: usize) -> u64 {
    let root_depth = path.components().count();
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() {
                if candidate.components().count().saturating_sub(root_depth) < max_depth {
                    stack.push(candidate);
                }
            } else if candidate.is_file() {
                total += candidate.metadata().map(|meta| meta.len()).unwrap_or(0);
            }
        }
    }
    total
}

pub(crate) fn export_client_delivery_portfolio_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/client_delivery_portfolio.json");
    let registry_path = root.join("core/clients/_registry.toml");
    let clients_root = root.join("data/business/clients");
    let registry = read_toml_or(&registry_path, toml::Value::Table(Default::default()));
    let client_entries = registry
        .get("client")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut clients = Vec::new();
    let mut high_responsibility_clients = 0usize;
    let mut awaiting_inputs_total = 0usize;

    for entry in client_entries {
        let Some(entry) = entry.as_table() else {
            continue;
        };
        let slug = toml_string(entry.get("slug")).trim().to_string();
        if slug.is_empty() {
            continue;
        }

        let control = read_json_or(&clients_root.join(&slug).join("control.json"), json!({}));
        let profile = read_toml_or(
            &clients_root.join(&slug).join("profile.toml"),
            toml::Value::Table(Default::default()),
        );
        let engagement = control
            .get("engagement")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let infrastructure = control
            .get("infrastructure")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let project_lanes = control
            .get("project_lanes")
            .cloned()
            .unwrap_or_else(|| json!([]));
        let next_required_inputs = control
            .get("next_required_inputs")
            .cloned()
            .unwrap_or_else(|| json!([]));

        let active = toml_string(entry.get("status")) == "active";
        let strategic_priority = engagement
            .get("strategic_priority")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if active && strategic_priority == "high" {
            high_responsibility_clients += 1;
        }
        if active
            && next_required_inputs
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        {
            awaiting_inputs_total += 1;
        }

        clients.push(json!({
            "slug": slug,
            "name": toml_to_json(entry.get("name")),
            "status": toml_to_json(entry.get("status")),
            "priority": toml_to_json(entry.get("priority")),
            "clearance": toml_to_json(entry.get("clearance")),
            "client_type": toml_to_json(entry.get("type")),
            "engagement": {
                "relationship": engagement.get("relationship").cloned().unwrap_or(Value::Null),
                "compensation_model": engagement.get("compensation_model").cloned().unwrap_or(Value::Null),
                "payment_posture": payment_posture(
                    engagement
                        .get("compensation_model")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ),
                "strategic_priority": engagement.get("strategic_priority").cloned().unwrap_or(Value::Null),
                "tech_lead_role": engagement.get("tech_lead_role").cloned().unwrap_or(Value::Null),
                "decision_owner": engagement.get("decision_owner").cloned().unwrap_or(Value::Null),
            },
            "business": {
                "industry": toml_table_value(&profile, &["business", "industry"]),
                "location": toml_table_value(&profile, &["business", "location"]),
                "description": toml_table_value(&profile, &["business", "description"]),
            },
            "delivery_posture": {
                "github_connected": infrastructure.get("github_connected").and_then(Value::as_bool).unwrap_or(false),
                "aws_amplify_connected": infrastructure.get("aws_amplify_connected").and_then(Value::as_bool).unwrap_or(false),
                "website_operations": infrastructure.get("website_operations").and_then(Value::as_bool).unwrap_or(false),
                "repo_identifiers_known": infrastructure.get("repo_identifiers_known").and_then(Value::as_bool).unwrap_or(false),
                "amplify_app_identifiers_known": infrastructure.get("amplify_app_identifiers_known").and_then(Value::as_bool).unwrap_or(false),
                "amplify_app_id": infrastructure.get("amplify_app_id").cloned().unwrap_or(Value::Null),
                "amplify_app_name": infrastructure.get("amplify_app_name").cloned().unwrap_or(Value::Null),
                "deploy_branch": infrastructure.get("deploy_branch").cloned().unwrap_or(Value::Null),
                "canonical_repo_path": infrastructure.get("canonical_repo_path").cloned().unwrap_or(Value::Null),
                "github_remote": infrastructure.get("github_remote").cloned().unwrap_or(Value::Null),
                "default_branch": infrastructure.get("default_branch").cloned().unwrap_or(Value::Null),
            },
            "technical_baseline": control.get("technical_baseline").cloned().unwrap_or_else(|| json!({})),
            "project_lanes": project_lanes,
            "next_required_inputs": next_required_inputs,
        }));
    }

    clients.sort_by(|a, b| {
        let a_priority = a.get("priority").and_then(Value::as_i64).unwrap_or(99);
        let b_priority = b.get("priority").and_then(Value::as_i64).unwrap_or(99);
        (
            a_priority,
            a.get("slug").and_then(Value::as_str).unwrap_or(""),
        )
            .cmp(&(
                b_priority,
                b.get("slug").and_then(Value::as_str).unwrap_or(""),
            ))
    });

    let payload = json!({
        "schema_version": "annunimas.client-delivery-portfolio.v1",
        "generated_at_utc": now_utc(),
        "authority": "core_clients_registry + business_client_controls",
        "mission": {
            "goal": "Separate client identity, delivery projects, and infrastructure ownership so Annunimas can handle external work without collapsing it into the internal project queue."
        },
        "organization_model": {
            "layers": [
                {"layer": "client", "purpose": "relationship, clearance, payment posture, decision ownership"},
                {"layer": "project", "purpose": "bounded delivery lane per site, app, or operating initiative"},
                {"layer": "infrastructure", "purpose": "GitHub, AWS Amplify, secrets, deployment environments, and branch policy"},
            ],
            "rules": [
                "Do not mix external client work into the generic sovereign project registry without a client anchor.",
                "Each external website or app gets its own project lane under a client.",
                "Infrastructure credentials stay separate from task intent and content updates.",
                "Payment posture and relationship pressure must be visible so priority is not inferred from cash alone.",
            ],
        },
        "clients": clients,
        "recommended_execution_order": [
            "stabilize client portfolio and project lanes",
            "capture GitHub and Amplify identifiers",
            "create per-client website delivery tasks",
            "execute content/deployment changes",
        ],
        "summary": {
            "clients_total": clients.len(),
            "active_clients_total": clients.iter().filter(|row| row.get("status").and_then(Value::as_str) == Some("active")).count(),
            "high_responsibility_clients_total": high_responsibility_clients,
            "awaiting_inputs_total": awaiting_inputs_total,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "clients_total": payload["summary"]["clients_total"],
    }))
}

pub(crate) fn export_client_delivery_readiness_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/client_delivery_readiness.json");
    let portfolio = read_json_or(
        &root.join("core/state/client_delivery_portfolio.json"),
        json!({}),
    );
    let clients = portfolio
        .get("clients")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let readiness = clients
        .iter()
        .filter(|client| client.get("client_type").and_then(Value::as_str) == Some("external"))
        .map(readiness_for_client)
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "annunimas.client-delivery-readiness.v1",
        "generated_at_utc": now_utc(),
        "authority": "client_delivery_portfolio + local_repo_truth",
        "clients": readiness,
        "summary": {
            "clients_total": readiness.len(),
            "build_verified_total": readiness.iter().filter(|client| client.get("build_verified").and_then(Value::as_bool) == Some(true)).count(),
            "blocked_on_infra_total": readiness.iter().filter(|client| client.get("readiness").and_then(Value::as_str) == Some("blocked_on_infra_mapping")).count(),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_valinor_project_dossiers_impl() -> Result<Value> {
    let root = workspace_root();
    let numenor_root = numenor_prime_root();
    let valinor_root_dir = valinor_root();
    let khazadforge_root = numenor_root.join("Eregion/KhazadForge");
    let out_path = root.join("core/state/valinor_project_dossiers.json");
    let clients = client_map(&root);
    let standard = read_json_or(
        &root.join("core/state/project_dossier_standard.json"),
        json!({}),
    );
    let posture = read_json_or(
        &root.join("core/state/portfolio_classification_posture.json"),
        json!({}),
    );
    let skylight = clients
        .get("skylight-pros")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let wgtt = clients.get("wgtt").cloned().unwrap_or_else(|| json!({}));

    let mut dossiers = vec![
        json!({
            "slug": "skylight-pros",
            "kind": "client_delivery_project",
            "valinor_lineage": [
                valinor_root_dir.join("archive/BUSINESS_OPPORTUNITIES/SKYLIGHTPROS_HARDENING").display().to_string(),
                valinor_root_dir.join("DECISIONS_2026-02-14.md").display().to_string(),
            ],
            "current_anchor": skylight.get("delivery_posture").and_then(|v| v.get("canonical_repo_path")).cloned().unwrap_or_else(|| json!("")),
            "current_status": "active_delivery",
            "readiness": "ready_for_content_and_code",
            "owner": "prometheus",
            "lane": "client_delivery",
            "reason": "Live repo, Amplify app id, deploy branch, and delivery posture are already known.",
            "next_actions": [
                "Continue real site/content execution in the live Skylight repo.",
                "Use Valinor hardening pack only as reference memory for backlog shaping and quality gates.",
            ],
            "source_lineage": "valinor + client_delivery_portfolio",
            "execution_constraints": execution_constraints_for("client_delivery_project", "ready_for_content_and_code"),
        }),
        json!({
            "slug": "wgtt",
            "kind": "client_delivery_project",
            "valinor_lineage": [
                valinor_root_dir.join("archive/BUSINESS_OPPORTUNITIES/WGTT_HARDENING").display().to_string(),
                valinor_root_dir.join("OPPORTUNITIES/Games/Weathertop_v2_Grand_Line_Vision.md").display().to_string(),
            ],
            "current_anchor": wgtt.get("delivery_posture").and_then(|v| v.get("canonical_repo_path")).cloned().unwrap_or_else(|| json!("")),
            "current_status": "delivery_pending_inputs",
            "readiness": "blocked_on_infra_mapping",
            "owner": "prometheus",
            "lane": "client_delivery",
            "reason": "Repo is known, but Amplify app id and deployment mapping are still missing.",
            "next_actions": [
                "Capture Amplify app id/app name and branch mapping.",
                "Then convert the Valinor hardening pack into a bounded delivery plan rather than raw archive reference.",
            ],
            "source_lineage": "valinor + client_delivery_portfolio",
            "execution_constraints": execution_constraints_for("client_delivery_project", "blocked_on_infra_mapping"),
        }),
        json!({
            "slug": "realmgate-warriors",
            "kind": "internal_project",
            "valinor_lineage": [
                valinor_root_dir.join("Realmgate_Warriors_Task_Plan.txt").display().to_string(),
                valinor_root_dir.join("archive/BUSINESS_OPPORTUNITIES/REALMGATEWARRIORS_HARDENING").display().to_string(),
            ],
            "current_anchor": khazadforge_root.join("realmgateWarriors").display().to_string(),
            "current_status": "cataloged_not_active",
            "readiness": "guided",
            "owner": "prometheus",
            "lane": "internal_product",
            "reason": "Historical task plan and hardening pack exist, but no active execution lane is currently open.",
            "next_actions": [
                "Promote a bounded hardening or product direction before reopening execution.",
                "Do not import the old task plan directly into the live queue.",
            ],
            "source_lineage": "valinor_project_memory",
            "execution_constraints": execution_constraints_for("internal_project", "guided"),
        }),
        json!({
            "slug": "crusties",
            "kind": "internal_product",
            "valinor_lineage": [
                valinor_root_dir.join("CRUSTIES").display().to_string(),
                valinor_root_dir.join("archive/BUSINESS_OPPORTUNITIES/CRUSTIES_MVP").display().to_string(),
            ],
            "current_anchor": khazadforge_root.join("crusties").display().to_string(),
            "current_status": "structured_memory_ready",
            "readiness": "planned",
            "owner": "prometheus",
            "lane": "internal_product",
            "reason": "CRUSTIES now has both a canonical doc suite and an archived MVP pack, but no current Annunimas execution plan has been raised yet.",
            "next_actions": [
                "Convert CRUSTIES memory into an Annunimas-native product dossier and ship/no-ship decision gate.",
                "Keep whimsical framing subordinate to actual product and economics discipline.",
            ],
            "source_lineage": "valinor_project_memory",
            "execution_constraints": execution_constraints_for("internal_product", "planned"),
        }),
        json!({
            "slug": "citadel-arda",
            "kind": "platform_memory",
            "valinor_lineage": [
                valinor_root_dir.join("archive/CITADEL").display().to_string(),
                valinor_root_dir.join("refactor/ARDA_HUB_HTML_COMPONENTIZATION_PLAN_2026-02-26.md").display().to_string(),
                valinor_root_dir.join("refactor/ARDA_HUD_CORPORATE_DESK_INTEGRATION_PLAN_2026-02-26.md").display().to_string(),
            ],
            "current_anchor": root.join("apps").display().to_string(),
            "current_status": "pattern_memory_only",
            "readiness": "guided",
            "owner": "prometheus",
            "lane": "platform_design_and_runtime",
            "reason": "This lineage is strategically important, but it should feed current ARDA/Annunimas work as pattern memory, not as a direct backlog import.",
            "next_actions": [
                "Mine the refactor and CITADEL archive only when current ARDA or platform design work opens a bounded lane.",
                "Prefer modern Annunimas runtime truth over older Valinor platform theory.",
            ],
            "source_lineage": "valinor_platform_memory",
            "execution_constraints": execution_constraints_for("platform_memory", "guided"),
        }),
    ];

    let required_fields = standard
        .get("required_fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    for dossier in &mut dossiers {
        let readiness = dossier
            .get("readiness")
            .and_then(Value::as_str)
            .unwrap_or("");
        let kind = dossier.get("kind").and_then(Value::as_str).unwrap_or("");
        let current_status = dossier
            .get("current_status")
            .and_then(Value::as_str)
            .unwrap_or("");
        dossier["review_posture"] =
            Value::String(classify_review_posture(readiness, kind, current_status).to_string());
        let missing = required_fields
            .iter()
            .filter(|field| is_missing_field(dossier, field))
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        dossier["dossier_standard_compliance"] = json!({
            "standard_loaded": !required_fields.is_empty(),
            "required_fields_total": required_fields.len(),
            "missing_required_fields": missing,
            "compliant": missing.is_empty(),
        });
    }

    let payload = json!({
        "schema_version": "annunimas.valinor-project-dossiers.v1",
        "generated_at_utc": now_utc(),
        "authority": "valinor_merge_registry + client_delivery_portfolio + curated_project_conversion + project_dossier_standard + portfolio_classification_posture",
        "status": "converted_to_annunimas_native_dossiers",
        "summary": {
            "dossiers_total": dossiers.len(),
            "active_delivery_total": dossiers.iter().filter(|row| row.get("current_status").and_then(Value::as_str) == Some("active_delivery")).count(),
            "guided_total": dossiers.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("guided")).count(),
            "planned_total": dossiers.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("planned")).count(),
            "blocked_total": dossiers.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("blocked_on_infra_mapping")).count(),
            "build_now_total": dossiers.iter().filter(|row| row.get("review_posture").and_then(Value::as_str) == Some("build_now")).count(),
            "incubate_total": dossiers.iter().filter(|row| row.get("review_posture").and_then(Value::as_str) == Some("incubate")).count(),
            "park_total": dossiers.iter().filter(|row| row.get("review_posture").and_then(Value::as_str) == Some("park")).count(),
            "dossier_standard_compliant_total": dossiers
                .iter()
                .filter(|row| row.get("dossier_standard_compliance").and_then(|v| v.get("compliant")).and_then(Value::as_bool) == Some(true))
                .count(),
        },
        "conversion_rules": {
            "do_not_import_old_tasks_as_live_queue": true,
            "prefer_current_annunimas_runtime_truth": true,
            "client_delivery_uses_client_portfolio_as_anchor": true,
            "valinor_archive_is_reference_memory_not_authority": true,
        },
        "governance_binding": {
            "project_dossier_standard_loaded": !required_fields.is_empty(),
            "portfolio_classification_posture_loaded": posture.get("labels").is_some(),
        },
        "dossiers": dossiers,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "dossiers_total": payload["summary"]["dossiers_total"],
    }))
}

pub(crate) fn export_imported_corpus_plan_portfolio_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/imported_corpus_plan_portfolio.json");
    let state_root = root.join("core/state");
    let entries = collect_imported_corpus_entries(&state_root);
    let payload = json!({
        "schema_version": "annunimas.imported-corpus-plan-portfolio.v1",
        "generated_at_utc": now_utc(),
        "authority": "numenor_prime_batches + valinor_batches + valinor_project_dossiers",
        "status": "comprehensive_plans_prepared_for_review",
        "planning_rule": {
            "system_lift_goes_first": true,
            "do_not_execute_new_code_until_review": true,
            "all_imported_items_must_land_in_a_plan_group": true,
        },
        "summary": {
            "entries_total": entries.len(),
            "system_lift_total": entries.iter().filter(|row| row.get("plan_group").and_then(Value::as_str) == Some("system_lift")).count(),
            "delivery_and_product_total": entries.iter().filter(|row| row.get("plan_group").and_then(Value::as_str) == Some("delivery_and_product")).count(),
            "opportunity_and_research_total": entries.iter().filter(|row| row.get("plan_group").and_then(Value::as_str) == Some("opportunity_and_research")).count(),
        },
        "plan_groups": [
            {
                "id": "system_lift",
                "human_plan": "human/plans/SYSTEM_LIFT_FROM_IMPORTED_CORPUS.md",
                "goal": "Use imported memory to strengthen Annunimas runtime, tooling, intake discipline, and platform patterns first.",
            },
            {
                "id": "delivery_and_product",
                "human_plan": "human/plans/DELIVERY_AND_PRODUCT_PORTFOLIO_FROM_IMPORTED_CORPUS.md",
                "goal": "Turn imported client/product memory into bounded execution plans after system lift review.",
            },
            {
                "id": "opportunity_and_research",
                "human_plan": "human/plans/OPPORTUNITY_AND_RESEARCH_PORTFOLIO_FROM_IMPORTED_CORPUS.md",
                "goal": "Preserve imported opportunity and research memory as a structured reservoir until selected.",
            },
        ],
        "entries": entries,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "entries_total": payload["summary"]["entries_total"],
    }))
}

fn payment_posture(compensation_model: &str) -> String {
    match compensation_model.trim().to_ascii_lowercase().as_str() {
        "minimal_pay" => "underpaid_priority".to_string(),
        "no_pay" => "unpaid_obligation".to_string(),
        "" => "unknown".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "unknown".to_string(),
    }
}

fn readiness_for_client(client: &Value) -> Value {
    let slug = client
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let posture = client
        .get("delivery_posture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let repo_path_str = posture
        .get("canonical_repo_path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let repo_path = if repo_path_str.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(&repo_path_str))
    };
    let env_info = repo_path
        .as_deref()
        .map(|path| parse_env_example_groups(&path.join(".env.example")))
        .unwrap_or_else(|| json!({"present": false, "groups": {}, "variables_total": 0}));
    let amplify_known = posture
        .get("amplify_app_identifiers_known")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let repo_known = posture
        .get("repo_identifiers_known")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut blockers = Vec::new();
    if !repo_known {
        blockers.push("repo_identity_unknown");
    }
    if !amplify_known {
        blockers.push("amplify_app_identifier_missing");
    }
    if !env_info
        .get("present")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        blockers.push("env_template_missing");
    }

    let build_verified = slug == "skylight-pros";
    let mut readiness = if build_verified && repo_known {
        "ready_for_content_and_code"
    } else {
        "partial"
    };
    if !blockers.is_empty() && !amplify_known {
        readiness = "blocked_on_infra_mapping";
    }

    json!({
        "slug": slug,
        "repo_path_present": repo_path.as_deref().map(Path::exists).unwrap_or(false),
        "env_template": env_info,
        "build_verified": build_verified,
        "amplify_app_id": posture.get("amplify_app_id").cloned().unwrap_or(Value::Null),
        "deploy_branch": posture.get("deploy_branch").cloned().unwrap_or(Value::Null),
        "readiness": readiness,
        "blockers": blockers,
        "next_step": if blockers.contains(&"amplify_app_identifier_missing") {
            "map_amplify_app_and_deploy_branch"
        } else {
            "execute_content_and_delivery_changes"
        },
    })
}

fn parse_env_example_groups(path: &Path) -> Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return json!({"present": false, "groups": {}, "variables_total": 0});
    };
    let mut groups = BTreeMap::new();
    let mut current = "ungrouped".to_string();
    let mut total = 0usize;
    for raw_line in raw.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            current = line
                .trim_start_matches('#')
                .trim()
                .to_ascii_lowercase()
                .replace(' ', "_");
            if current.is_empty() {
                current = "ungrouped".to_string();
            }
            groups.entry(current.clone()).or_insert(0usize);
            continue;
        }
        if !line.contains('=') {
            continue;
        }
        *groups.entry(current.clone()).or_insert(0usize) += 1;
        total += 1;
    }
    json!({"present": true, "groups": groups, "variables_total": total})
}

fn client_map(root: &Path) -> BTreeMap<String, Value> {
    read_json_or(
        &root.join("core/state/client_delivery_portfolio.json"),
        json!({}),
    )
    .get("clients")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter_map(|row| {
        row.get("slug")
            .and_then(Value::as_str)
            .map(|slug| (slug.to_string(), row.clone()))
    })
    .collect()
}

fn classify_review_posture(readiness: &str, kind: &str, current_status: &str) -> &'static str {
    if readiness == "ready_for_content_and_code" {
        "build_now"
    } else if matches!(readiness, "blocked_on_infra_mapping" | "planned" | "guided") {
        "incubate"
    } else if matches!(
        current_status,
        "cataloged_not_active" | "pattern_memory_only"
    ) {
        "park"
    } else if current_status.contains("archive") || current_status.contains("reference") {
        "archive"
    } else if kind == "platform_memory" {
        "park"
    } else {
        "incubate"
    }
}

fn execution_constraints_for(kind: &str, readiness: &str) -> Vec<Value> {
    let mut constraints = vec![
        Value::String("Do not import historical tasks directly into the live queue.".to_string()),
        Value::String(
            "Keep Valinor lineage as reference memory, not sovereign authority.".to_string(),
        ),
    ];
    if kind == "client_delivery_project" {
        constraints.push(Value::String(
            "Execution requires client delivery mapping and current repo/deployment anchor."
                .to_string(),
        ));
    }
    if readiness == "blocked_on_infra_mapping" {
        constraints.push(Value::String(
            "Do not open execution until infrastructure mapping is explicit.".to_string(),
        ));
    }
    if kind == "platform_memory" {
        constraints.push(Value::String(
            "Platform memory must be pattern-mined, not replayed as a parallel architecture."
                .to_string(),
        ));
    }
    constraints
}

fn is_missing_field(dossier: &Value, field: &str) -> bool {
    let Some(value) = dossier.get(field) else {
        return true;
    };
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(rows) => rows.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

fn collect_imported_corpus_entries(state_root: &Path) -> Vec<Value> {
    let mut batch_paths = fs::read_dir(state_root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    (name.starts_with("numenor_prime_promotion_batch_0")
                        || name.starts_with("valinor_promotion_batch_0"))
                        && name.ends_with(".json")
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    batch_paths.sort();

    let mut entries = Vec::new();
    for path in batch_paths {
        let payload = read_json_or(&path, json!({}));
        let batch_id = payload
            .get("batch_id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
            });
        for item in payload
            .get("promoted_items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            entries.push(plan_shape(item, batch_id));
        }
    }
    let dossier_payload =
        read_json_or(&state_root.join("valinor_project_dossiers.json"), json!({}));
    for item in dossier_payload
        .get("dossiers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let mut shaped = plan_shape(item, "valinor_project_dossiers");
        shaped["current_status"] = item.get("current_status").cloned().unwrap_or(Value::Null);
        shaped["current_anchor"] = item.get("current_anchor").cloned().unwrap_or(Value::Null);
        shaped["next_actions"] = item
            .get("next_actions")
            .cloned()
            .unwrap_or_else(|| json!([]));
        shaped["readiness"] = item
            .get("readiness")
            .cloned()
            .unwrap_or(shaped["readiness"].clone());
        entries.push(shaped);
    }
    entries
}

fn plan_shape(item: &Value, batch_id: &str) -> Value {
    let (plan_group, priority, lane) = classify_imported_item(item);
    let label = item
        .get("project")
        .or_else(|| item.get("slug"))
        .or_else(|| item.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("item");
    let source = item.get("source").and_then(Value::as_str).unwrap_or("");
    let kind = item
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("memory_item");
    let summary = item
        .get("summary")
        .or_else(|| item.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let (execution_mode, readiness, next_step) = match plan_group {
        "system_lift" => (
            "plan_now_execute_after_review",
            "guided",
            "turn imported memory into bounded system implementation plans and runtime contracts",
        ),
        "delivery_and_product" => (
            "plan_now_execute_after_review",
            if summary.contains("ready_for_content_and_code")
                || item.get("readiness").and_then(Value::as_str)
                    == Some("ready_for_content_and_code")
            {
                "ready"
            } else {
                "guided"
            },
            "turn imported memory into bounded delivery or product execution plans",
        ),
        _ => (
            "plan_now_hold_until_selected",
            "planned",
            "keep as ready planning reservoir until explicitly selected for execution",
        ),
    };

    json!({
        "id": slugify(&format!("{batch_id}:{label}:{kind_or_lane}", kind_or_lane = if kind.is_empty() { lane } else { kind })),
        "label": label,
        "kind": if kind.is_empty() { "memory_item" } else { kind },
        "batch_id": batch_id,
        "source": source,
        "summary": summary,
        "plan_group": plan_group,
        "lane": lane,
        "priority": priority,
        "execution_mode": execution_mode,
        "readiness": readiness,
        "next_step": next_step,
    })
}

fn classify_imported_item(item: &Value) -> (&'static str, &'static str, &'static str) {
    let project = item
        .get("project")
        .or_else(|| item.get("slug"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = item
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let source = item
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();

    if ["athena", "hermes", "arda_hud", "citadel", "citadel_base"]
        .iter()
        .any(|token| project.contains(token))
    {
        return ("system_lift", "high", "system_and_platform");
    }
    if [
        "knowledge_systems",
        "model_selection",
        "agent_operations",
        "deployment_stack",
        "security",
        "automation",
        "engineering_process",
        "multi_project",
        "knowledge_graph",
    ]
    .iter()
    .any(|token| project.contains(token))
    {
        return ("system_lift", "high", "system_and_tooling");
    }
    if [
        "infra_pattern",
        "framework_gate",
        "project_registry",
        "tool_ingest",
        "tool_research",
        "tool_digest",
        "operations_process",
        "root_doctrine",
        "decision_memory",
        "refactor_memory",
    ]
    .iter()
    .any(|token| kind.contains(token))
    {
        return ("system_lift", "high", "system_and_tooling");
    }
    if ["skylight", "wgtt", "realmgate", "crusties"]
        .iter()
        .any(|token| project.contains(token))
    {
        return ("delivery_and_product", "high", "delivery_or_product");
    }
    if [
        "project_overview",
        "project_doc_suite",
        "historical_project_memory",
    ]
    .iter()
    .any(|token| kind.contains(token))
    {
        return ("delivery_and_product", "medium", "delivery_or_product");
    }
    if kind.contains("opportunit")
        || source.contains("opportunit")
        || kind.contains("portfolio_memory")
        || kind.contains("archive_memory")
    {
        return ("opportunity_and_research", "medium", "future_selection");
    }
    ("opportunity_and_research", "low", "reference_memory")
}

fn slugify(value: &str) -> String {
    value.to_ascii_lowercase().replace([' ', '/'], "_")
}

fn toml_string(value: Option<&toml::Value>) -> String {
    value
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn toml_to_json(value: Option<&toml::Value>) -> Value {
    value.map(toml_value_to_json).unwrap_or(Value::Null)
}

fn toml_table_value(value: &toml::Value, path: &[&str]) -> Value {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Value::Null;
        };
        current = next;
    }
    toml_value_to_json(current)
}

fn toml_value_to_json(value: &toml::Value) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}
