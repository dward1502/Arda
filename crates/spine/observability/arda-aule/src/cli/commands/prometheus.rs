#![cfg(feature = "full-cli")]
use super::super::*;
use crate::commands::arandur::resolve_root;
use crate::commands::manwe_telemetry;

pub(crate) async fn handle(command: PrometheusCommands) -> anyhow::Result<()> {
    let service = PrometheusService::from_core("core")?;
    let default_socket_path = service.socket_path();
    match command {
        PrometheusCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = PrometheusDaemon::new(
                service,
                PrometheusDaemonConfig {
                    socket_path: expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        PrometheusCommands::Status => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.status()?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::OpsDashboard => {
            let value = build_ops_dashboard("core").await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::OpsBriefing { root, format } => {
            let value = build_operations_briefing(&resolve_root(root))?;
            match format {
                OpsBriefingFormat::Json => println!("{}", serde_json::to_string_pretty(&value)?),
                OpsBriefingFormat::Text => println!("{}", format_operations_briefing_text(&value)),
            }
        }
        PrometheusCommands::Maintenance {
            sweep_type,
            cooldown_seconds,
            r#async,
            prune,
            prune_threshold_pct,
        } => {
            let value = if r#async {
                spawn_maintenance_cycle(&sweep_type, cooldown_seconds, prune, prune_threshold_pct)?
            } else {
                run_maintenance_cycle(&sweep_type, cooldown_seconds, prune, prune_threshold_pct)
                    .await?
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::Roster => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "roster",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.roster())?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::Thoughts { limit } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "thoughts",
                serde_json::json!({ "limit": limit }),
                || Ok(serde_json::to_value(service.thoughts(limit)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::Escalate {
            limit,
            include_resolved,
        } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "escalations",
                serde_json::json!({
                    "limit": limit,
                    "include_resolved": include_resolved
                }),
                || {
                    Ok(serde_json::to_value(
                        service.escalations(limit, include_resolved)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::ResolveEscalation {
            escalation_id,
            note,
        } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "resolve_escalation",
                serde_json::json!({
                    "escalation_id": escalation_id,
                    "note": note
                }),
                || {
                    Ok(serde_json::to_value(
                        service.resolve_escalation(&escalation_id, &note)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::ReconcileRuntime {
            before,
            apply,
            note,
        } => {
            let cutoff = chrono::DateTime::parse_from_rfc3339(&before)?.with_timezone(&chrono::Utc);
            let value = prometheus_call_or_local(
                &default_socket_path,
                "reconcile_runtime",
                serde_json::json!({
                    "before": cutoff.to_rfc3339(),
                    "apply": apply,
                    "note": note
                }),
                || {
                    Ok(serde_json::to_value(
                        service.reconcile_runtime(cutoff, apply, &note)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::CouncilFanout {
            topic,
            participants,
            context,
        } => {
            let context_json = context
                .as_deref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
            let value = prometheus_call_or_local(
                &default_socket_path,
                "council_fanout",
                serde_json::json!({
                    "topic": topic,
                    "participants": participants,
                    "context": context_json
                }),
                || Ok(service.council_fanout(&topic, participants, context_json)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::ExecutionIntents {
            limit,
            include_terminal,
        } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "execution_intents",
                serde_json::json!({
                    "limit": limit,
                    "include_terminal": include_terminal
                }),
                || {
                    Ok(serde_json::to_value(
                        service.execution_intents(limit, include_terminal)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::ExecutionIntentRecovery => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "execution_intents_recovery",
                serde_json::json!({}),
                || Ok(service.execution_intents_recovery()?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::TransitionIntent {
            intent_id,
            status,
            note,
        } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "transition_execution_intent",
                serde_json::json!({
                    "intent_id": intent_id,
                    "status": status,
                    "note": note
                }),
                || Ok(service.transition_execution_intent(&intent_id, &status, note.as_deref())?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::CompactIntents {
            retention_days,
            max_keep,
        } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "compact_execution_intents",
                serde_json::json!({
                    "retention_days": retention_days,
                    "max_keep": max_keep
                }),
                || Ok(service.compact_execution_intents(retention_days, max_keep)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::DriftCheck { reconcile } => {
            let value = prometheus_call_or_local(
                &default_socket_path,
                "drift_detect_reconcile",
                serde_json::json!({
                    "auto_open": reconcile
                }),
                || Ok(service.drift_detect_reconcile(reconcile)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::PersonalitySet {
            agent_id,
            personality,
            comms_style,
            notes,
        } => {
            let value = set_agent_personality(&agent_id, &personality, &comms_style, notes)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::PersonalityList => {
            let value = list_agent_personalities()?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        PrometheusCommands::Autopilot { command } => autopilot_handle(command).await?,
        PrometheusCommands::Arandur { command } => arandur::handle(command)?,
        PrometheusCommands::Charon { command } => manwe_telemetry::handle(command)?,
        PrometheusCommands::Ruleset { command } => match command {
            RulesetCommands::Status => {
                let value = serde_json::to_value(load_active_ruleset())?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            RulesetCommands::Set {
                profile,
                reason,
                expires_at_utc,
            } => {
                let value = set_active_ruleset(&profile, &reason, expires_at_utc)?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        },
    }
    Ok(())
}

async fn autopilot_handle(command: AutopilotCommands) -> anyhow::Result<()> {
    use crate::prometheus::autopilot::{ceo_loop, AutopilotConfig, CeoAutopilot};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    let resolve_root = |root: Option<String>| -> PathBuf {
        root.map(PathBuf::from)
            .or_else(|| std::env::var("ARDA_ROOT").ok().map(PathBuf::from))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };
    match command {
        AutopilotCommands::Once { root, read_only } => {
            let mut cfg = AutopilotConfig::from_root(resolve_root(root));
            cfg.read_only = read_only;
            let mut auto = CeoAutopilot::from_world(cfg);
            let report = auto.run_cycle().await;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        AutopilotCommands::Run {
            root,
            interval,
            read_only,
        } => {
            let mut cfg = AutopilotConfig::from_root(resolve_root(root));
            cfg.interval = Duration::from_secs(interval);
            cfg.read_only = read_only;
            let auto = CeoAutopilot::from_world(cfg);
            let stop = Arc::new(AtomicBool::new(false));
            let s2 = stop.clone();
            tokio::spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                s2.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            eprintln!(
                "ceo-autopilot starting (root={}, interval={}s, read_only={})",
                auto.config().root.display(),
                interval,
                auto.config().read_only
            );
            ceo_loop(auto, stop).await;
        }
        AutopilotCommands::Status { root } => {
            let path = resolve_root(root).join("data/ceo/autopilot.state.json");
            match std::fs::read_to_string(&path) {
                Ok(s) => println!("{}", s),
                Err(_) => println!("{{\"error\":\"no autopilot state at {}\"}}", path.display()),
            }
        }
        AutopilotCommands::KnowledgeTriage {
            root,
            dry_run,
            write,
            source_roots,
        } => {
            let resolved_root = resolve_root(root);
            let mut cfg =
                crate::prometheus::autopilot::KnowledgeTriageConfig::for_root(&resolved_root)
                    .with_dry_run(dry_run || !write);
            if !source_roots.is_empty() {
                cfg = cfg.with_source_roots(
                    source_roots
                        .into_iter()
                        .map(|source_root| expand_home(&source_root))
                        .collect(),
                );
            }
            let report = crate::prometheus::autopilot::run_knowledge_triage(&cfg)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        AutopilotCommands::PromoteKnowledgeTasks {
            root,
            lane,
            dry_run,
            write,
            approval_evidence,
            source_roots,
        } => {
            if lane != "safe-local" {
                anyhow::bail!(
                    "unsupported promotion lane '{lane}'; only safe-local is write-eligible"
                );
            }
            let resolved_root = resolve_root(root);
            let mut cfg =
                crate::prometheus::autopilot::KnowledgeTriageConfig::for_root(&resolved_root)
                    .with_dry_run(dry_run || !write);
            if let Some(approval_evidence) = approval_evidence {
                cfg = cfg.with_approval_evidence(approval_evidence);
            }
            if !source_roots.is_empty() {
                cfg = cfg.with_source_roots(
                    source_roots
                        .into_iter()
                        .map(|source_root| expand_home(&source_root))
                        .collect(),
                );
            }
            let report = crate::prometheus::autopilot::promote_knowledge_tasks(&cfg)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        AutopilotCommands::ExecuteKnowledgeTasks {
            root,
            dry_run,
            write,
        } => {
            let resolved_root = resolve_root(root);
            let cfg = crate::prometheus::autopilot::KnowledgeTriageConfig::for_root(&resolved_root)
                .with_dry_run(dry_run || !write);
            let report = crate::prometheus::autopilot::execute_knowledge_task_queue(&cfg)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}
