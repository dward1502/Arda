use super::super::*;

pub(crate) async fn handle(command: MnemosyneCommands) -> anyhow::Result<()> {
    let service = MnemosyneService::from_default_or_fallback()?;
    match command {
        MnemosyneCommands::Status => {
            let out = service.status()?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Paths => {
            let out = serde_json::to_value(service.paths())?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Stats => {
            let out = serde_json::to_value(service.stats()?)?;
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
            let out = serde_json::to_value(service.encode(event)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::RecallRecent {
            hours,
            crate_name,
            scope,
            query,
            limit,
        } => {
            let memories = if let Some(query) = query.as_deref() {
                service.recall_relevant(
                    query,
                    hours,
                    crate_name.as_deref(),
                    scope.as_deref(),
                    limit,
                )?
            } else {
                service.recall_recent_scoped(hours, crate_name.as_deref(), scope.as_deref())?
            };
            let out = serde_json::to_value(memories)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::KnowledgeSeeds { query, limit } => {
            let out = serde_json::to_value(service.recall_knowledge_seeds(query.as_deref(), limit)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::Consolidate { hours } => {
            let out = serde_json::to_value(service.consolidate(hours)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::IdentityState => {
            let out = serde_json::to_value(service.identity_state()?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        MnemosyneCommands::ObsidianSync {
            vault_path,
            max_files,
        } => {
            let out = serde_json::to_value(service.sync_obsidian(&vault_path, max_files)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}
