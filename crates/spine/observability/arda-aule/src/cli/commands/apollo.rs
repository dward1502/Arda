#![cfg(feature = "full-cli")]
use super::super::*;

pub(crate) async fn handle(command: ApolloCommands) -> anyhow::Result<()> {
    let service = ApolloService::from_default_or_workspace_fallback()?;
    let default_socket_path = socket_path_from_env("ARDA_APOLLO_SOCKET", "data/apollo/apollo.sock");
    match command {
        ApolloCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = ApolloDaemon::new(
                service,
                ApolloDaemonConfig {
                    socket_path: apollo_expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        ApolloCommands::Status => {
            let out = apollo_call_or_local(&default_socket_path, "status", json!({}), || async {
                service.status().await
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        ApolloCommands::Submit {
            task_id,
            agent_id,
            payload,
            priority,
            timeout_secs,
        } => {
            let payload_value = parse_json_input(&payload)?;
            let request = ExecutionRequest {
                task_id,
                agent_id,
                payload: payload_value.clone(),
                priority: parse_execution_priority(&priority),
                timeout_secs,
            };
            let out = apollo_call_or_local(
                &default_socket_path,
                "submit",
                json!({
                    "task_id": request.task_id,
                    "agent_id": request.agent_id,
                    "payload": request.payload,
                    "priority": priority,
                    "timeout_secs": request.timeout_secs,
                }),
                || async { Ok(json!({"task_id": service.submit(request).await?})) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        ApolloCommands::Execute { task_id } => {
            let out = apollo_call_or_local(
                &default_socket_path,
                "execute",
                json!({ "task_id": task_id }),
                || async { Ok(json!({"result": service.execute(&task_id).await?})) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        ApolloCommands::ExecuteApproved {
            task_id,
            approval_evidence,
        } => {
            if approval_evidence.trim().is_empty() {
                anyhow::bail!("--approval-evidence is required for execute-approved");
            }
            let out = apollo_call_or_local(
                &default_socket_path,
                "execute",
                json!({
                    "task_id": task_id,
                    "approval_evidence": approval_evidence,
                    "approval_gate": "operator_supplied"
                }),
                || async { Ok(json!({"result": service.execute(&task_id).await?})) },
            )
            .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "contract": "arda.apollo.execute_approved.v1",
                    "task_id": task_id,
                    "approval_evidence": approval_evidence,
                    "execution": out
                }))?
            );
        }
        ApolloCommands::Interrupt {
            task_id,
            content,
            source,
            sender,
            disposition,
            run_id,
            session_id,
        } => {
            let out = apollo_call_or_local(
                &default_socket_path,
                "interrupt",
                json!({
                    "task_id": task_id,
                    "content": content,
                    "source": source,
                    "sender": sender,
                    "disposition": disposition,
                    "run_id": run_id,
                    "session_id": session_id
                }),
                || async {
                    Ok(json!({
                        "interrupt": service.attach_interrupt(InterruptionAttachmentRequest {
                            task_id: &task_id,
                            source: &source,
                            sender: &sender,
                            content: &content,
                            disposition: &disposition,
                            run_id: run_id.clone(),
                            session_id: session_id.clone(),
                        }).await?
                    }))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        ApolloCommands::Paths => {
            let out = apollo_call_or_local(&default_socket_path, "paths", json!({}), || async {
                Ok(serde_json::to_value(service.runtime_paths())?)
            })
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}
