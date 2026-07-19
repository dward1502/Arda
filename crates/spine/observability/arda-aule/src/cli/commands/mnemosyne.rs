#![cfg(feature = "full-cli")]
use super::super::*;

pub(crate) async fn handle(command: MnemosyneCommands) -> anyhow::Result<()> {
    let service = MnemosyneService::from_default_or_fallback()?;
    let default_socket_path = socket_path_from_env(
        "ARDA_MNEMOSYNE_SOCKET",
        "data/mnemosyne/mnemosyne.sock",
    );
    match command {
        MnemosyneCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = MnemosyneDaemon::new(
                service,
                MnemosyneDaemonConfig {
                    socket_path: expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        MnemosyneCommands::Status => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || Ok(service.status()?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Paths => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "paths",
                serde_json::json!({}),
                || Ok(service.paths()),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Stats => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "stats",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.stats()?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Encode {
            event_type,
            informant_id,
            crate_name,
            content,
            confidence,
            tags,
        } => {
            let mut event_tags = if tags.is_empty() {
                vec!["manual".to_string()]
            } else {
                tags
            };
            if !event_tags.iter().any(|tag| tag == "manual") {
                event_tags.push("manual".to_string());
            }
            let event = InformantEvent {
                informant_id,
                crate_name,
                event_type,
                ts_utc: chrono::Utc::now().to_rfc3339(),
                content,
                confidence_hint: confidence,
                tags: event_tags,
            };
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "encode",
                serde_json::to_value(&event)?,
                || Ok(serde_json::to_value(service.encode(event)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::RecallRecent {
            hours,
            crate_name,
            scope,
            query,
            limit,
        } => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "recall_recent",
                serde_json::json!({
                    "hours": hours,
                    "crate_name": crate_name,
                    "scope": scope,
                    "query": query,
                    "limit": limit
                }),
                || {
                    let memories = if let Some(query) = query.as_deref() {
                        service.recall_relevant(
                            query,
                            hours,
                            crate_name.as_deref(),
                            scope.as_deref(),
                            limit,
                        )?
                    } else {
                        service.recall_recent_scoped(
                            hours,
                            crate_name.as_deref(),
                            scope.as_deref(),
                        )?
                    };
                    Ok(serde_json::to_value(memories)?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::KnowledgeSeeds { query, limit } => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "recall_knowledge_seeds",
                serde_json::json!({
                    "query": query,
                    "limit": limit
                }),
                || {
                    Ok(serde_json::to_value(
                        service.recall_knowledge_seeds(query.as_deref(), limit)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Consolidate { hours } => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "consolidate",
                serde_json::json!({ "hours": hours }),
                || Ok(serde_json::to_value(service.consolidate(hours)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::IdentityState => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "identity_state",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.identity_state()?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::ObsidianSync {
            vault_path,
            max_files,
        } => {
            let out = mnemosyne_call_or_local(
                &default_socket_path,
                "obsidian_sync",
                serde_json::json!({
                    "vault_path": vault_path,
                    "max_files": max_files,
                }),
                || {
                    Ok(serde_json::to_value(
                        service.sync_obsidian(&vault_path, max_files)?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}
