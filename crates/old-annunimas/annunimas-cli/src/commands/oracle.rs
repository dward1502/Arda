use super::super::*;

pub(crate) async fn handle(command: OracleCommands) -> anyhow::Result<()> {
    let service = OracleService::from_default_or_workspace_fallback()?;
    let default_socket_path =
        socket_path_from_env("ANNUNIMAS_ORACLE_SOCKET", "data/oracle/oracle.sock");
    match command {
        OracleCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = OracleDaemon::new(
                service,
                OracleDaemonConfig {
                    socket_path: oracle_expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        OracleCommands::Status => {
            let out = oracle_call_or_local(&default_socket_path, "status", json!({}), || async {
                service.status().await
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        OracleCommands::Evaluate {
            task,
            id,
            requester,
            context,
        } => {
            let out = oracle_call_or_local(
                &default_socket_path,
                "evaluate",
                json!({
                    "id": id,
                    "task": task,
                    "requester": requester,
                    "context": context
                }),
                || async {
                    Ok(serde_json::to_value(
                        service
                            .evaluate(OracleQuery {
                                id: id.clone().unwrap_or_else(|| "oracle_cli".to_string()),
                                task,
                                context,
                                requester,
                                timestamp: Utc::now(),
                            })
                            .await?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        OracleCommands::Verdicts { limit } => {
            let out = oracle_call_or_local(
                &default_socket_path,
                "verdicts",
                json!({ "limit": limit }),
                || async { Ok(serde_json::to_value(service.recent_verdicts(limit)?)?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        OracleCommands::Readiness => {
            let out =
                serde_json::to_value(annunimas_governance::default_governance_readiness_report())?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        OracleCommands::PhilosopherProfiles {
            profiles_path,
            format,
        } => {
            let profile_source = profiles_path.clone();
            let resolved_profiles_path = resolve_profile_path(&profiles_path);
            let profiles =
                annunimas_governance::load_philosopher_profiles(&resolved_profiles_path)?;
            let projection = profiles.status_projection(profile_source);
            match format {
                PhilosopherProfilesFormat::Json => {
                    let out = serde_json::to_value(projection)?;
                    println!("{}", serde_json::to_string_pretty(&out)?);
                }
                PhilosopherProfilesFormat::Compact => {
                    println!("{}", format_philosopher_profiles_compact(&projection));
                }
                PhilosopherProfilesFormat::Status => {
                    let out = philosopher_profiles_status_surface(&projection);
                    println!("{}", serde_json::to_string_pretty(&out)?);
                }
            }
        }
        OracleCommands::Paths => {
            let out = oracle_call_or_local(&default_socket_path, "paths", json!({}), || async {
                Ok(serde_json::to_value(service.runtime_paths())?)
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}

fn resolve_profile_path(profiles_path: &str) -> PathBuf {
    let path = Path::new(profiles_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        annunimas_root().join(path)
    }
}

fn format_philosopher_profiles_compact(
    projection: &annunimas_governance::PhilosopherProfileStatusProjection,
) -> String {
    let blocking = if projection.autonomous_blocking_enabled {
        "enabled"
    } else {
        "disabled"
    };
    let corpus_promotion = if projection.generated_corpus_promotion_enabled {
        "enabled"
    } else {
        "disabled"
    };
    let mut lines = vec![
        "Triad Philosopher Profiles".to_string(),
        "──────────────────────────".to_string(),
        format!("Schema:      {}", projection.schema_version),
        format!("Source:      {}", projection.profile_source),
        format!(
            "Review:      {} / {}",
            projection.review_mode, projection.profile_maturity
        ),
        format!("Profiles:    {}", projection.profile_count),
        format!("Autonomy:    blocking={blocking}; corpus_promotion={corpus_promotion}"),
    ];

    for profile in &projection.profiles {
        lines.push(format!(
            "  • {} — {} [{}], confidence_floor={:.2}, blocking=false",
            profile.id, profile.display_name, profile.lens, profile.confidence_floor
        ));
    }

    lines.join("\n")
}

fn philosopher_profiles_status_surface(
    projection: &annunimas_governance::PhilosopherProfileStatusProjection,
) -> serde_json::Value {
    let profile_ids = projection
        .profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();

    json!({
        "schema_version": projection.schema_version,
        "surface": "annunimas.governance.philosopher_profiles.status.v1",
        "status": "read_only_non_blocking",
        "profile_source": projection.profile_source,
        "profile_count": projection.profile_count,
        "profile_ids": profile_ids,
        "review_mode": projection.review_mode,
        "profile_maturity": projection.profile_maturity,
        "chain_id": projection.chain_id,
        "chain_version": projection.chain_version,
        "autonomous_blocking_enabled": projection.autonomous_blocking_enabled,
        "generated_corpus_promotion_enabled": projection.generated_corpus_promotion_enabled,
    })
}
