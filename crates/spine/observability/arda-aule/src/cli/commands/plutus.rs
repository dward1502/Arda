#![cfg(feature = "full-cli")]
use super::super::*;

pub(crate) async fn handle(command: PlutusCommands) -> anyhow::Result<()> {
    let service = PlutusService::from_default_or_workspace_fallback()?;
    let default_socket_path = socket_path_from_env("ARDA_PLUTUS_SOCKET", "data/plutus/plutus.sock");
    match command {
        PlutusCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = PlutusDaemon::new(
                service,
                PlutusDaemonConfig {
                    socket_path: plutus_expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        PlutusCommands::Status => {
            let out = plutus_call_or_local(&default_socket_path, "status", json!({}), || async {
                service.status().await
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::RegisterModel {
            provider,
            input_rate,
            output_rate,
            batch_size,
        } => {
            let payload = json!({
                "provider": provider,
                "input_rate": input_rate,
                "output_rate": output_rate,
                "batch_size": batch_size
            });
            let out =
                plutus_call_or_local(&default_socket_path, "register_model", payload, || async {
                    service
                        .register_model(CostModelConfig {
                            provider,
                            input_rate,
                            output_rate,
                            batch_size,
                        })
                        .await?;
                    Ok(json!({"registered": true}))
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::RecordSpend {
            provider,
            input_tokens,
            output_tokens,
        } => {
            let out = plutus_call_or_local(
                &default_socket_path,
                "record_spend",
                json!({
                    "provider": provider,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens
                }),
                || async {
                    Ok(json!({
                        "cost": service.record_spend(&provider, input_tokens, output_tokens).await?
                    }))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::TrackWork {
            agent_id,
            amount,
            unit,
            task_id,
        } => {
            let parsed_unit = parse_joulework_unit(&unit);
            let out = plutus_call_or_local(
                &default_socket_path,
                "track_work",
                json!({
                    "agent_id": agent_id,
                    "amount": amount,
                    "unit": unit,
                    "task_id": task_id
                }),
                || async {
                    service
                        .track_work(&agent_id, amount, parsed_unit, task_id.clone())
                        .await?;
                    Ok(json!({"tracked": true}))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::Credit { account, amount } => {
            let out = plutus_call_or_local(
                &default_socket_path,
                "credit",
                json!({"account": account, "amount": amount}),
                || async {
                    service.credit(&account, amount).await?;
                    Ok(json!({"credited": true}))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::Relationship {
            from,
            to,
            trust,
            attention,
            reciprocity,
        } => {
            let out = plutus_call_or_local(
                &default_socket_path,
                "relationship",
                json!({
                    "from": from,
                    "to": to,
                    "trust": trust,
                    "attention": attention,
                    "reciprocity": reciprocity
                }),
                || async {
                    Ok(json!({
                        "score": service.record_relationship(&from, &to, trust, attention, reciprocity).await?
                    }))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        PlutusCommands::Paths => {
            let out = plutus_call_or_local(&default_socket_path, "paths", json!({}), || async {
                Ok(serde_json::to_value(service.runtime_paths())?)
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}
