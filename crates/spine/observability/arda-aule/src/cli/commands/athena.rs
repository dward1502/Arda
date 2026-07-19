#![cfg(feature = "full-cli")]
use super::super::*;
use arda_core::llm::{LlmProvider, OpenAiCompatibleProvider};
use regex::Regex;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn default_charon_llm_base_url() -> String {
    format!("http://{}:{}/v1", "127.0.0.1", 5110)
}

fn default_crawl4ai_url() -> String {
    format!("http://{}:{}", "127.0.0.1", 11235)
}

/// Build the LLM provider used by ATHENA for digestion-extraction calls.
///
/// Default: route through the local Charon model-switch using
/// `ARDA_ATHENA_LLM_BASE_URL` or the split loopback host/port fallback;
/// Charon picks the model from its routing matrix when model is `"auto"`.
///
/// Overrides:
/// - `ARDA_ATHENA_LLM_BASE_URL`: override base URL
/// - `ARDA_ATHENA_LLM_MODEL`: override default model id
/// - `ARDA_ATHENA_LLM_API_KEY_ENV`: name of env var to read for bearer auth
/// - `ARDA_ATHENA_LLM_USE_CONFIG=1`: opt out of Charon and use the
///   configured default provider from `config/default.toml`
fn build_athena_extraction_provider(
    config: &arda_core::config::Config,
) -> Arc<dyn LlmProvider> {
    if std::env::var("ARDA_ATHENA_LLM_USE_CONFIG")
        .map(|v| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return super::super::cli_bootstrap::build_provider(config);
    }
    let base_url = std::env::var("ARDA_ATHENA_LLM_BASE_URL")
        .unwrap_or_else(|_| default_charon_llm_base_url());
    let model = std::env::var("ARDA_ATHENA_LLM_MODEL").unwrap_or_else(|_| "auto".to_string());
    let api_key = std::env::var("ARDA_ATHENA_LLM_API_KEY_ENV")
        .ok()
        .and_then(|key| std::env::var(key).ok())
        .filter(|s| !s.trim().is_empty());
    Arc::new(OpenAiCompatibleProvider::new(
        "charon", &base_url, api_key, &model,
    ))
}

fn ensure_athena_status_knowledge_vault(
    value: &mut serde_json::Value,
    store: &AthenaStore,
) -> anyhow::Result<()> {
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    if object.contains_key("knowledge_vault") {
        return Ok(());
    }

    object.insert(
        "knowledge_vault".to_string(),
        serde_json::to_value(store.knowledge_vault_status()?)?,
    );
    Ok(())
}

pub(crate) async fn handle(
    command: AthenaCommands,
    config: &ARDA_core::config::Config,
) -> anyhow::Result<()> {
    let llm = build_athena_extraction_provider(config);
    let store = AthenaStore::from_default_or_workspace_fallback()?.with_llm(llm.clone());
    let default_socket_path =
        socket_path_from_env("ARDA_ATHENA_SOCKET", "data/athena/athena.sock");
    match command {
        AthenaCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            tracing::info!(
                provider = llm.provider_name(),
                model = llm.default_model(),
                "ATHENA daemon starting with LLM-driven digestion extraction"
            );
            let daemon = AthenaDaemon::new(
                store,
                AthenaDaemonConfig {
                    socket_path: expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        AthenaCommands::Status => {
            let mut value = athena_call_or_local(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || Ok(serde_json::to_value(store.status()?)?),
            )
            .await?;
            ensure_athena_status_knowledge_vault(&mut value, &store)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::VaultStatus => {
            let value = serde_json::to_value(store.knowledge_vault_status()?)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::Ingest {
            input,
            submitted_by,
            task_context,
        } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "ingest",
                serde_json::json!({
                    "input": input,
                    "submitted_by": submitted_by,
                    "task_context": task_context
                }),
                || {
                    Ok(serde_json::to_value(store.ingest(
                        &input,
                        &submitted_by,
                        &task_context,
                    )?)?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::IngestBatch {
            input_file,
            submitted_by,
            task_context,
            batch_size,
            max_receipts,
        } => {
            let path = expand_home(&input_file);
            let file = fs::File::open(&path)?;
            let reader = BufReader::new(file);
            let effective_batch_size = batch_size.max(1);
            let mut pending = Vec::with_capacity(effective_batch_size);
            let mut aggregate = BatchIngestReport {
                total_inputs: 0,
                accepted_inputs: 0,
                deduplicated_inputs: 0,
                invalid_inputs: 0,
                receipts: Vec::new(),
            };
            let mut batches_run = 0usize;

            for line in reader.lines() {
                let line = line?;
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                pending.push(trimmed.to_string());
                if pending.len() >= effective_batch_size {
                    let report = athena_ingest_batch_chunk(
                        &default_socket_path,
                        &store,
                        &pending,
                        &submitted_by,
                        &task_context,
                    )
                    .await?;
                    merge_batch_report(&mut aggregate, report, max_receipts);
                    pending.clear();
                    batches_run += 1;
                }
            }

            if !pending.is_empty() {
                let report = athena_ingest_batch_chunk(
                    &default_socket_path,
                    &store,
                    &pending,
                    &submitted_by,
                    &task_context,
                )
                .await?;
                merge_batch_report(&mut aggregate, report, max_receipts);
                batches_run += 1;
            }

            let receipts_omitted = aggregate
                .accepted_inputs
                .saturating_sub(aggregate.receipts.len());
            let value = serde_json::json!({
                "total_inputs": aggregate.total_inputs,
                "accepted_inputs": aggregate.accepted_inputs,
                "deduplicated_inputs": aggregate.deduplicated_inputs,
                "invalid_inputs": aggregate.invalid_inputs,
                "batches_run": batches_run,
                "batch_size": effective_batch_size,
                "max_receipts": max_receipts,
                "receipts_omitted": receipts_omitted,
                "receipts": aggregate.receipts,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::HumanCorpusWave { limit } => {
            let value = ingest_human_corpus_wave(&store, limit)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::HumanScan {
            human_root,
            output,
            contradictions,
            limit,
        } => {
            let human_root = expand_home(&human_root.to_string_lossy());
            let output = expand_home(&output.to_string_lossy());
            let contradictions = expand_home(&contradictions.to_string_lossy());
            let report = arda_varda::human::scan_human_root(
                &human_root,
                &output,
                Some(&contradictions),
                limit,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        AthenaCommands::ImportUrls {
            input_file,
            submitted_by,
            task_context,
        } => {
            let value = store.ingest_url_list_file(
                expand_home(&input_file),
                &submitted_by,
                &task_context,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::ImportXBookmarks {
            input_file,
            submitted_by,
            task_context,
        } => {
            let value = store.ingest_x_bookmarks_export(
                expand_home(&input_file),
                &submitted_by,
                &task_context,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::ImportXSearch {
            query,
            limit,
            hermes_bin,
            capture_file,
            submitted_by,
            task_context,
        } => {
            let capture_path = match capture_file {
                Some(path) => expand_home(&path.to_string_lossy()),
                None => run_hermes_x_search_capture(&query, limit, &hermes_bin)?,
            };
            let report =
                store.ingest_x_search_capture(&capture_path, &submitted_by, &task_context)?;
            let value = serde_json::json!({
                "query": query,
                "limit": limit,
                "capture_path": capture_path,
                "ingest_report": report,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::ImportAiChats {
            input_file,
            submitted_by,
            task_context,
        } => {
            let value = store.ingest_ai_chat_export(
                expand_home(&input_file),
                &submitted_by,
                &task_context,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::HumanDocumentWave { limit } => {
            let value = ingest_human_document_wave(&store, limit)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::Crawl {
            url,
            filter,
            query,
            ingest,
            submitted_by,
            task_context,
        } => {
            let crawl_profile = std::env::var("ARDA_ATHENA_CRAWL_PROFILE")
                .unwrap_or_else(|_| "production".to_string());
            let provider_order = resolve_crawl_provider_order(
                std::env::var("ARDA_ATHENA_CRAWL_PROVIDER")
                    .ok()
                    .as_deref(),
                Some(&crawl_profile),
            );
            let crawl_service =
                std::env::var("ARDA_CRAWL4AI_URL").unwrap_or_else(|_| default_crawl4ai_url());
            let mut crawl = None;
            let mut route_attempts = Vec::new();
            for provider in provider_order {
                match provider.as_str() {
                    "scrapling" => {
                        let url_for_fetch = url.clone();
                        let filter_for_fetch = filter.clone();
                        let query_for_fetch = query.clone();
                        let scrapling_result = tokio::task::spawn_blocking(move || {
                            scrapling_fetch_markdown(
                                &url_for_fetch,
                                &filter_for_fetch,
                                query_for_fetch.as_deref(),
                            )
                        })
                        .await
                        .map_err(|err| {
                            anyhow::anyhow!("scrapling fetch task failed to join: {err}")
                        })?;
                        match scrapling_result {
                            Ok(value) => {
                                route_attempts.push(
                                    serde_json::json!({"provider":"scrapling","status":"ok"}),
                                );
                                crawl = Some(value);
                                break;
                            }
                            Err(err) => route_attempts.push(serde_json::json!({
                                "provider":"scrapling",
                                "status":"failed",
                                "error": err.to_string()
                            })),
                        }
                    }
                    "crawl4ai" => {
                        let start_status = Command::new("bash")
                            .arg("scripts/runtime/crawl4ai_service.sh")
                            .arg("start")
                            .output()?;
                        if !start_status.status.success() {
                            route_attempts.push(serde_json::json!({
                                "provider":"crawl4ai",
                                "status":"failed",
                                "error": format!("failed to start crawl4ai service: {}", String::from_utf8_lossy(&start_status.stderr))
                            }));
                            continue;
                        }
                        match crawl4ai_fetch_markdown(
                            &crawl_service,
                            &url,
                            &filter,
                            query.as_deref(),
                        )
                        .await
                        {
                            Ok(value) => {
                                route_attempts
                                    .push(serde_json::json!({"provider":"crawl4ai","status":"ok"}));
                                crawl = Some(value);
                                break;
                            }
                            Err(err) => route_attempts.push(serde_json::json!({
                                "provider":"crawl4ai",
                                "status":"failed",
                                "error": err.to_string()
                            })),
                        }
                    }
                    other => route_attempts.push(serde_json::json!({
                        "provider": other,
                        "status":"skipped",
                        "error":"unsupported crawl provider"
                    })),
                }
            }
            let crawl = crawl.ok_or_else(|| {
                anyhow::anyhow!(
                    "all crawl providers failed: {}",
                    serde_json::to_string(&route_attempts).unwrap_or_else(|_| "[]".to_string())
                )
            })?;
            let crawl_receipt = store.record_crawl_capture(
                &url,
                &submitted_by,
                &task_context,
                &crawl.provider,
                &crawl,
            )?;
            let ingest_record = if ingest {
                Some(store.ingest(&url, &submitted_by, &task_context)?)
            } else {
                None
            };

            let value = serde_json::json!({
                "crawl": crawl,
                "route_attempts": route_attempts,
                "crawl_receipt": crawl_receipt,
                "ingest": ingest_record,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::Query { query, limit } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "query",
                serde_json::json!({
                    "query": query,
                    "limit": limit
                }),
                || Ok(serde_json::to_value(store.query(&query, limit)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::Deep { source_id, reason } => {
            let resolved_source_id = resolve_athena_source_id(&source_id);
            let value = athena_call_or_local(
                &default_socket_path,
                "deep_analyze",
                serde_json::json!({
                    "source_id": resolved_source_id,
                    "reason": reason
                }),
                || {
                    let queued = store.queue_deep_analysis(&resolved_source_id, "cli", &reason)?;
                    let deep = store.deep_analyze(&resolved_source_id)?;
                    Ok(serde_json::json!({
                        "queued": queued,
                        "deep": deep
                    }))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::DeepProcess {
            limit,
            retry_failed,
        } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "deep_process",
                serde_json::json!({
                    "limit": limit,
                    "retry_failed": retry_failed
                }),
                || Ok(store.process_deep_queue(limit, retry_failed)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::Digest { source_id, limit } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "digest",
                serde_json::json!({
                    "source_id": source_id,
                    "limit": limit
                }),
                || {
                    Ok(serde_json::to_value(
                        store.read_digest(source_id.as_deref(), limit)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::PolicyReadiness { limit } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "policy_readiness",
                serde_json::json!({
                    "limit": limit
                }),
                || Ok(serde_json::to_value(store.policy_readiness(limit)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::PolicyPromote { limit, reevaluate } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "policy_promote",
                serde_json::json!({
                    "limit": limit,
                    "reevaluate": reevaluate
                }),
                || Ok(store.promote_policy_readiness(limit, reevaluate)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::OppositionHarvest {
            source_id,
            topic,
            submitted_by,
        } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "harvest_opposition",
                serde_json::json!({
                    "source_id": source_id,
                    "topic": topic,
                    "submitted_by": submitted_by
                }),
                || {
                    Ok(store.harvest_opposition_evidence(
                        &source_id,
                        topic.as_deref(),
                        &submitted_by,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::PacketIntake {
            root,
            packet_dir,
            output,
            write,
        } => {
            let root = resolve_cli_root(Some(root));
            let value = build_packet_intake_surface(&root, &packet_dir, &output, write)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::PacketPromotionSurface {
            root,
            packet_dir,
            output,
            write,
        } => {
            let root = resolve_cli_root(Some(root));
            let value = build_packet_promotion_surface(&root, &packet_dir, &output, write)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        AthenaCommands::GeneratePlanningTasks { source_id, limit } => {
            let value = athena_call_or_local(
                &default_socket_path,
                "generate_planning_tasks",
                serde_json::json!({
                    "source_id": source_id,
                    "limit": limit
                }),
                || Ok(store.generate_planning_tasks(&source_id, limit)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn resolve_cli_root(root: Option<String>) -> PathBuf {
    root.filter(|value| value.trim() != ".")
        .map(PathBuf::from)
        .or_else(|| std::env::var("ARDA_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(workspace_root)
}

fn packet_path(root: &Path, packet_dir: &str) -> PathBuf {
    let path = PathBuf::from(packet_dir);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn resolve_output_path(root: &Path, output: &str) -> PathBuf {
    let path = PathBuf::from(output);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn repo_display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read_jsonl(path: &Path) -> anyhow::Result<Vec<Value>> {
    let raw = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let value = serde_json::from_str::<Value>(trimmed).map_err(|err| {
            anyhow::anyhow!("invalid JSONL at {}:{}: {err}", path.display(), idx + 1)
        })?;
        rows.push(value);
    }
    Ok(rows)
}

fn packet_matrix_rows(packet_dir: &Path) -> anyhow::Result<Vec<Value>> {
    let matrix_path = packet_dir.join("promotion_matrix.json");
    let matrix = fs::read_to_string(&matrix_path)?;
    let value = serde_json::from_str::<Value>(&matrix).map_err(|err| {
        anyhow::anyhow!("invalid promotion matrix {}: {err}", matrix_path.display())
    })?;
    Ok(value
        .get("rows")
        .or_else(|| value.get("candidates"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn packet_queue_rows(packet_dir: &Path) -> anyhow::Result<Vec<Value>> {
    read_jsonl(&packet_dir.join("recommended_queue_entries.jsonl"))
}

fn build_packet_intake_surface(
    root: &Path,
    packet_dir: &str,
    output: &str,
    write: bool,
) -> anyhow::Result<Value> {
    let packet_dir = packet_path(root, packet_dir);
    let rows = packet_matrix_rows(&packet_dir)?;
    let queue_rows = packet_queue_rows(&packet_dir)?;
    let packet_label = repo_display_path(root, &packet_dir);
    let mut intake_rows = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let source = row
            .get("source")
            .or_else(|| row.get("source_path"))
            .or_else(|| row.get("path"))
            .cloned()
            .unwrap_or(Value::Null);
        let target = row
            .get("target")
            .or_else(|| row.get("suggested_target"))
            .or_else(|| row.get("canonical_target"))
            .cloned()
            .unwrap_or(Value::Null);
        intake_rows.push(json!({
            "contract": "arda.athena.packet_intake.row.v1",
            "authority": "agent_generated",
            "review_required": true,
            "packet": packet_label,
            "row_index": idx,
            "lane": row.get("lane").cloned().unwrap_or(Value::Null),
            "cluster": row.get("cluster").cloned().unwrap_or(Value::Null),
            "source": source,
            "suggested_target": target,
            "claim_gate": row.get("claim_gate").or_else(|| row.get("gate")).cloned().unwrap_or(Value::Null),
            "matrix_status": row.get("status").cloned().unwrap_or(Value::Null),
            "rationale": row.get("rationale").cloned().unwrap_or(Value::Null),
            "promotion_status": "candidate_review_required"
        }));
    }
    let output_path = resolve_output_path(root, output);
    if write {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = intake_rows
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        fs::write(&output_path, format!("{}\n", body))?;
    }
    Ok(json!({
        "contract": "arda.athena.packet_intake.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": now_utc(),
        "packet_dir": repo_display_path(root, &packet_dir),
        "output": repo_display_path(root, &output_path),
        "written": write,
        "promotion_rows": intake_rows.len(),
        "recommended_queue_rows": queue_rows.len(),
        "mutation_policy": {
            "raw_human_inbox": "read_only",
            "canonical_queue": "not_mutated",
            "promotion": "candidate_only"
        },
        "rows": intake_rows
    }))
}

fn markdown_escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn value_summary(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| String::new()),
    }
}

fn build_packet_promotion_surface(
    root: &Path,
    packet_dir: &str,
    output: &str,
    write: bool,
) -> anyhow::Result<Value> {
    let packet_dir = packet_path(root, packet_dir);
    let rows = packet_matrix_rows(&packet_dir)?;
    let queue_rows = packet_queue_rows(&packet_dir)?;
    let output_path = resolve_output_path(root, output);
    let packet_label = repo_display_path(root, &packet_dir);
    let mut md = String::new();
    md.push_str("---\n");
    md.push_str("authority: agent_generated\nreview_required: true\nstatus: candidate\n");
    md.push_str("contract: arda.athena.packet_promotion_surface.v1\n---\n\n");
    md.push_str("# ATHENA Packet Promotion Surface\n\n");
    md.push_str(&format!("- generated_at_utc: `{}`\n", now_utc()));
    md.push_str(&format!("- packet_dir: `{}`\n", packet_label));
    md.push_str("- raw_human_inbox_policy: read_only\n");
    md.push_str("- canonical_queue_policy: recommendation_only_until_review\n");
    md.push_str("- research_claim_policy: review_gated\n\n");
    md.push_str("## Promotion Candidates\n\n");
    md.push_str("| Row | Matrix status | Source | Suggested target | Gate | Rationale |\n");
    md.push_str("|---:|---|---|---|---|---|\n");
    for (idx, row) in rows.iter().enumerate() {
        let status = markdown_escape_cell(&value_summary(row.get("status")));
        let source = markdown_escape_cell(&value_summary(
            row.get("source")
                .or_else(|| row.get("source_path"))
                .or_else(|| row.get("path")),
        ));
        let target = markdown_escape_cell(&value_summary(
            row.get("target")
                .or_else(|| row.get("suggested_target"))
                .or_else(|| row.get("canonical_target")),
        ));
        let gate = markdown_escape_cell(&value_summary(
            row.get("claim_gate").or_else(|| row.get("gate")),
        ));
        let rationale = markdown_escape_cell(&value_summary(row.get("rationale")));
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            idx + 1,
            status,
            source,
            target,
            gate,
            rationale
        ));
    }
    md.push_str("\n## Recommended Queue Entries\n\n");
    md.push_str("These remain review-gated; this surface does not append to `core/projects/tasks/queue.jsonl`.\n\n");
    for row in &queue_rows {
        let id = markdown_escape_cell(&value_summary(row.get("id").or_else(|| row.get("task_id"))));
        let title = markdown_escape_cell(&value_summary(row.get("title")));
        let owner = markdown_escape_cell(&value_summary(row.get("owner")));
        md.push_str(&format!("- `{}` — {} (owner: {})\n", id, title, owner));
    }
    md.push_str("\n## Review Gates\n\n");
    md.push_str("- [ ] Verify source provenance and raw inbox preservation.\n");
    md.push_str("- [ ] Decide whether each candidate belongs in project plans, knowledge notes, or queue recommendations.\n");
    md.push_str("- [ ] Keep all research claims review-gated until independently verified.\n");
    md.push_str("- [ ] Validate JSON/JSONL and run `git diff --check` before commit.\n");
    if write {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, md)?;
    }
    Ok(json!({
        "contract": "arda.athena.packet_promotion_surface.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": now_utc(),
        "packet_dir": repo_display_path(root, &packet_dir),
        "output": repo_display_path(root, &output_path),
        "written": write,
        "promotion_rows": rows.len(),
        "recommended_queue_rows": queue_rows.len()
    }))
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json_pretty(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn canonicalize_human_path(root: &Path, path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root_resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    match resolved.strip_prefix(&root_resolved) {
        Ok(relative) => {
            let mut parts = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            if parts.len() >= 2 && parts[0] == "human" && parts[1] == "Notes" {
                parts[1] = "notes".to_string();
            }
            PathBuf::from_iter(parts).display().to_string()
        }
        Err(_) => path.display().to_string(),
    }
}

fn truncate_chars(input: &str, limit: usize) -> String {
    input.chars().take(limit).collect()
}

fn preview_text(input: &str, limit: usize) -> String {
    truncate_chars(input.trim(), limit)
}

fn slugify(input: &str) -> String {
    let lowered = input.to_lowercase();
    let mut slug = String::with_capacity(lowered.len());
    let mut last_was_sep = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    slug.trim_matches('_').chars().take(80).collect()
}

fn human_corpus_text_candidates(root: &Path, limit: usize) -> Vec<Value> {
    read_json(&root.join("core/state/human_corpus_registry.json"))
        .get("top_ingest_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("class").and_then(Value::as_str) == Some("text"))
        .take(limit)
        .cloned()
        .collect()
}

fn human_document_candidates(root: &Path, limit: usize) -> Vec<Value> {
    read_json(&root.join("core/state/human_corpus_registry.json"))
        .get("crate_idea_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            matches!(
                row.get("class").and_then(Value::as_str),
                Some("document" | "archive")
            )
        })
        .take(limit)
        .cloned()
        .collect()
}

fn ingest_human_corpus_wave(store: &AthenaStore, limit: usize) -> anyhow::Result<Value> {
    let root = workspace_root();
    let out = root.join("core/state/human_corpus_wave.json");
    let mut results = Vec::new();

    for row in human_corpus_text_candidates(&root, limit.max(1)) {
        let path = PathBuf::from(
            row.get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        );
        let canonical = row
            .get("canonical_path")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| canonicalize_human_path(&root, &path));
        match fs::read_to_string(&path) {
            Ok(content) => {
                let payload = format!(
                    "Local corpus note from {canonical}.\n\n{}",
                    truncate_chars(&content, 24_000)
                );
                match store.ingest(
                    &payload,
                    "human_corpus_wave",
                    &format!("local corpus ingest from {canonical}"),
                ) {
                    Ok(receipt) => results.push(json!({
                        "path": path.display().to_string(),
                        "canonical_path": canonical,
                        "ok": true,
                        "exit_code": 0,
                        "stdout_preview": preview_text(&serde_json::to_string_pretty(&receipt)?, 1200),
                        "stderr_preview": "",
                        "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                        "root_id": row.get("root_id").cloned().unwrap_or(Value::Null),
                    })),
                    Err(err) => results.push(json!({
                        "path": path.display().to_string(),
                        "canonical_path": canonical,
                        "ok": false,
                        "exit_code": 1,
                        "stdout_preview": "",
                        "stderr_preview": preview_text(&err.to_string(), 800),
                        "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                        "root_id": row.get("root_id").cloned().unwrap_or(Value::Null),
                    })),
                }
            }
            Err(err) => results.push(json!({
                "path": path.display().to_string(),
                "canonical_path": canonical,
                "ok": false,
                "error": err.to_string(),
            })),
        }
    }

    let payload = json!({
        "schema_version": "arda.human-corpus-wave.v1",
        "generated_at_utc": now_utc(),
        "authority": "bounded_local_note_ingest_wave",
        "results": results,
        "summary": {
            "attempted_total": results.len(),
            "ok_total": results.iter().filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false)).count(),
            "failed_total": results.iter().filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false)).count(),
        },
    });
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": out.display().to_string(),
        "ok_total": payload.get("summary").and_then(|summary| summary.get("ok_total")).cloned().unwrap_or(Value::Null),
    }))
}

fn ingest_human_document_wave(store: &AthenaStore, limit: usize) -> anyhow::Result<Value> {
    let root = workspace_root();
    let extract_root = root.join("data/human/extracted");
    let out = root.join("core/state/human_document_wave.json");
    let mut results = Vec::new();

    for row in human_document_candidates(&root, limit.max(1)) {
        let source_path = PathBuf::from(
            row.get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        );
        let canonical = row
            .get("canonical_path")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| canonicalize_human_path(&root, &source_path));
        match extract_human_document(&root, &extract_root, &source_path) {
            Ok((class_name, content, extracted_path)) => {
                let payload = format!(
                    "Extracted local {class_name} from {canonical}.\n\n{}",
                    truncate_chars(&content, 24_000)
                );
                match store.ingest(
                    &payload,
                    "human_document_wave",
                    &format!("extracted local document ingest from {canonical}"),
                ) {
                    Ok(receipt) => results.push(json!({
                        "path": source_path.display().to_string(),
                        "canonical_path": canonical,
                        "extracted_path": extracted_path.display().to_string(),
                        "canonical_extracted_path": canonicalize_human_path(&root, &extracted_path),
                        "class": class_name,
                        "ok": true,
                        "stage": "ingest",
                        "exit_code": 0,
                        "stdout_preview": preview_text(&serde_json::to_string_pretty(&receipt)?, 1200),
                        "stderr_preview": "",
                        "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                        "root_id": row.get("root_id").cloned().unwrap_or(Value::Null),
                    })),
                    Err(err) => results.push(json!({
                        "path": source_path.display().to_string(),
                        "canonical_path": canonical,
                        "extracted_path": extracted_path.display().to_string(),
                        "canonical_extracted_path": canonicalize_human_path(&root, &extracted_path),
                        "class": class_name,
                        "ok": false,
                        "stage": "ingest",
                        "exit_code": 1,
                        "stdout_preview": "",
                        "stderr_preview": preview_text(&err.to_string(), 800),
                        "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                        "root_id": row.get("root_id").cloned().unwrap_or(Value::Null),
                    })),
                }
            }
            Err(err) => results.push(json!({
                "path": source_path.display().to_string(),
                "canonical_path": canonical,
                "ok": false,
                "stage": "extract",
                "error": err.to_string(),
                "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                "root_id": row.get("root_id").cloned().unwrap_or(Value::Null),
            })),
        }
    }

    let payload = json!({
        "schema_version": "arda.human-document-wave.v1",
        "generated_at_utc": now_utc(),
        "authority": "bounded_docx_and_archive_extraction_wave",
        "results": results,
        "summary": {
            "attempted_total": results.len(),
            "ok_total": results.iter().filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false)).count(),
            "failed_total": results.iter().filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false)).count(),
            "extracted_root": extract_root.display().to_string(),
        },
    });
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": out.display().to_string(),
        "ok_total": payload.get("summary").and_then(|summary| summary.get("ok_total")).cloned().unwrap_or(Value::Null),
    }))
}

fn extract_human_document(
    root: &Path,
    extract_root: &Path,
    source_path: &Path,
) -> anyhow::Result<(String, String, PathBuf)> {
    let (class_name, content) = match source_path
        .extension()
        .and_then(|suffix| suffix.to_str())
        .map(|suffix| suffix.to_ascii_lowercase())
        .as_deref()
    {
        Some("docx") => ("document".to_string(), extract_docx_content(source_path)?),
        Some("zip") => ("archive".to_string(), extract_zip_inventory(source_path)?),
        _ => ("unknown".to_string(), String::new()),
    };
    if content.trim().is_empty() {
        anyhow::bail!("empty extracted content");
    }
    fs::create_dir_all(extract_root)?;
    let extracted_path = extract_root.join(format!(
        "{}.md",
        slugify(
            source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("extracted")
        )
    ));
    let header = format!(
        "# Extracted {}\n\n- source_path: `{}`\n- canonical_source_path: `{}`\n- extracted_at_utc: `{}`\n\n",
        title_case(&class_name),
        source_path.display(),
        canonicalize_human_path(root, source_path),
        now_utc(),
    );
    fs::write(&extracted_path, header + &content + "\n")?;
    Ok((class_name, content, extracted_path))
}

fn extract_docx_content(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(path)
        .arg("word/document.xml")
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to read word/document.xml from {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let xml = String::from_utf8(output.stdout)?;
    let paragraph_breaks = Regex::new(r"</w:p>")?;
    let tag_pattern = Regex::new(r"<[^>]+>")?;
    let whitespace = Regex::new(r"\n{3,}")?;
    let text = paragraph_breaks.replace_all(&xml, "\n\n");
    let text = tag_pattern.replace_all(&text, "");
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");
    Ok(whitespace.replace_all(text.trim(), "\n\n").to_string())
}

fn extract_zip_inventory(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("zipinfo").arg("-1").arg(path).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to inspect archive {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let listing = String::from_utf8(output.stdout)?;
    let lines = listing
        .lines()
        .take(200)
        .map(|line| format!("- {}", line.trim()))
        .collect::<Vec<_>>();
    Ok(if lines.is_empty() {
        "Archive inventory:\n".to_string()
    } else {
        format!("Archive inventory:\n\n{}", lines.join("\n"))
    })
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn run_hermes_x_search_capture(
    query: &str,
    limit: usize,
    hermes_bin: &str,
) -> anyhow::Result<PathBuf> {
    let prompt = format!(
        "Use the Hermes x_search tool to search X/Twitter for this query: {query:?}. \
Return only valid JSON with shape {{\"query\": string, \"items\": [{{\"url\": string, \
\"author\": string|null, \"text\": string|null, \"created_at\": string|null}}]}}. \
Include at most {limit} results. Do not use xurl or the X API; use only the x_search tool."
    );
    let output = Command::new(hermes_bin)
        .arg("-t")
        .arg("x_search")
        .arg("--ignore-rules")
        .arg("-z")
        .arg(prompt)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Hermes x_search capture failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout)?;
    if stdout.trim().is_empty() {
        anyhow::bail!("Hermes x_search capture returned empty output");
    }
    let capture_root = arda_root().join("data/athena/x_search");
    fs::create_dir_all(&capture_root)?;
    let capture_path = capture_root.join(format!(
        "x_search_{}.json",
        Utc::now().format("%Y%m%dT%H%M%SZ")
    ));
    fs::write(&capture_path, stdout)?;
    Ok(capture_path)
}
