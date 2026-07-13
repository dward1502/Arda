use super::super::*;
use annunimas_hermes::{DecisionOption, OutboundMessage};
use std::fs;
use std::path::Path;

fn load_env_file(path: &Path) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || std::env::var_os(key).is_some() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        std::env::set_var(key, value);
    }
}

fn hydrate_discord_env() {
    let root = std::env::current_dir().unwrap_or_else(|_| ".".into());
    for rel in ["config/.env", ".env"] {
        load_env_file(&root.join(rel));
    }
}

pub(crate) async fn handle(command: HermesCommands) -> anyhow::Result<()> {
    let service = HermesService::from_default_or_fallback()?;
    let default_socket_path =
        socket_path_from_env("ANNUNIMAS_HERMES_SOCKET", "data/hermes/hermes.sock");
    match command {
        HermesCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = HermesDaemon::new(
                service,
                HermesDaemonConfig {
                    socket_path: expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        HermesCommands::DiscordStart {
            token,
            token_env,
            application_id,
            guild_id,
            channel_id,
            ready_message,
        } => {
            hydrate_discord_env();
            let token = token
                .or_else(|| token_env.as_deref().and_then(|key| std::env::var(key).ok()))
                .or_else(|| std::env::var("DISCORD_BOT_TOKEN").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing Discord token: pass --token, --token-env, or DISCORD_BOT_TOKEN"
                    )
                })?;
            let application_id = application_id
                .or_else(|| {
                    std::env::var("DISCORD_APP_ID")
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                })
                .ok_or_else(|| anyhow::anyhow!("missing DISCORD_APP_ID or --application-id"))?;
            let guild_id = guild_id.or_else(|| {
                std::env::var("DISCORD_GUILD_ID")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
            });
            let channel_id = channel_id.or_else(|| {
                [
                    "DISCORD_CHANNEL_WORK_STREAM",
                    "DISCORD_CHANNEL_TASKS",
                    "DISCORD_HOME_CHANNEL",
                    "DISCORD_CHANNEL_ID",
                ]
                .into_iter()
                .find_map(|key| {
                    std::env::var(key)
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                })
            });

            let mut bot = DiscordBot::new(token, application_id);
            if let Some(guild_id) = guild_id {
                bot = bot.with_guild(guild_id);
            }
            if let Some(channel_id) = channel_id {
                bot = bot.with_channel(channel_id);
            }
            if let Some(ready_message) = ready_message {
                bot = bot.with_ready_message(ready_message);
            }
            bot.start().await?;
        }
        HermesCommands::DiscordChannels { command } => {
            hydrate_discord_env();
            match command {
                DiscordChannelsCommands::Plan => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&service.discord_channel_plan())?
                    );
                }
                DiscordChannelsCommands::Apply { dry_run, approve } => {
                    if !dry_run {
                        anyhow::bail!(
                            "discord channel apply is read-only in Gate 8; use --dry-run"
                        );
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &service.apply_discord_channel_plan_dry_run(approve)
                        )?
                    );
                }
            }
        }
        HermesCommands::Status => {
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || async { Ok(serde_json::to_value(service.status().await?)?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Providers => {
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "providers",
                serde_json::json!({}),
                || async { Ok(service.providers_status().await) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Subcomponents => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "subcomponents",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.subcomponents())?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Boardroom { limit } => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "boardroom_recent",
                serde_json::json!({ "limit": limit }),
                || Ok(serde_json::to_value(service.boardroom_recent(limit)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Classify {
            source,
            sender,
            content,
            channel,
            illuvatar,
        } => {
            let mut msg = InboundMessage::new(source, sender, content);
            msg.channel = channel;
            msg.is_illuvatar = illuvatar;
            let out = hermes_call_or_local(
                &default_socket_path,
                "classify",
                serde_json::to_value(&msg)?,
                || Ok(serde_json::to_value(service.classify(msg)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Send {
            provider,
            channel,
            subject,
            body,
            stream,
        } => {
            let mut msg = OutboundMessage::new(provider, channel, subject, body);
            msg.stream = stream;
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "send",
                serde_json::to_value(&msg)?,
                || async { Ok(service.send(msg).await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::RetryOutbound { limit } => {
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "retry_outbound",
                serde_json::json!({ "limit": limit }),
                || async { Ok(service.retry_outbound_queue(limit).await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::RetryRerouteDlq { limit } => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "retry_reroute_dlq",
                serde_json::json!({ "limit": limit }),
                || Ok(service.retry_reroute_dlq(limit)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::BoardroomPost {
            from_agent,
            subject,
            body,
        } => {
            let post = BoardroomPost::new(from_agent, "report", subject, body);
            let out = hermes_call_or_local(
                &default_socket_path,
                "boardroom_post",
                serde_json::to_value(&post)?,
                || {
                    service.boardroom_post(post)?;
                    Ok(serde_json::json!({"posted": true}))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::CalendarSync => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "calendar_sync",
                serde_json::json!({}),
                || Ok(service.calendar_sync()?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::IngestExternal {
            provider,
            sender,
            content,
            channel,
            illuvatar,
        } => {
            let prompt_channel = channel.clone().unwrap_or_else(|| "boardroom".to_string());
            let out = hermes_call_or_local(
                &default_socket_path,
                "ingest_external",
                serde_json::json!({
                    "provider": provider,
                    "sender": sender,
                    "content": content,
                    "channel": channel,
                    "is_illuvatar": illuvatar
                }),
                || {
                    Ok(serde_json::to_value(service.ingest_external(
                        &provider, &sender, &content, channel, illuvatar,
                    )?)?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
            if let Err(err) = maybe_send_illuvatar_decision_prompt(
                &service,
                &provider,
                &sender,
                &prompt_channel,
                &content,
                illuvatar,
            )
            .await
            {
                tracing::warn!(error = %err, "failed to send illuvatar decision prompt");
            }
        }
        HermesCommands::PollOnce => {
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "poll_once",
                serde_json::json!({}),
                || async {
                    Ok(serde_json::json!({
                        "processed": service.poll_providers_once().await?
                    }))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Interrupt {
            content,
            source,
            sender,
            channel,
            run_id,
            session_id,
            task_id,
        } => {
            let mut msg = InterruptionMessage::new(source, sender, content);
            msg.channel = channel;
            msg.run_id = run_id;
            msg.session_id = session_id;
            msg.task_id = task_id;
            let out = hermes_call_or_local(
                &default_socket_path,
                "interrupt",
                serde_json::to_value(&msg)?,
                || Ok(service.interrupt(msg)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::IlluvatarFanout {
            content,
            provider,
            sender,
            channel,
        } => {
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "illuvatar_fanout",
                serde_json::json!({
                    "source_provider": provider,
                    "sender": sender,
                    "content": content,
                    "channel": channel
                }),
                || async {
                    let mut msg =
                        InboundMessage::new(provider.clone(), sender.clone(), content.clone());
                    msg.channel = channel.clone();
                    msg.is_illuvatar = true;
                    Ok(service.fanout_illuvatar_directive(&provider, &msg).await?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::DecisionPrompt {
            provider,
            channel,
            question,
            source,
            sender,
            a_label,
            a_action,
            b_label,
            b_action,
            c_label,
            c_action,
        } => {
            let mut options = vec![
                DecisionOption {
                    key: "a".to_string(),
                    label: a_label,
                    action: a_action,
                },
                DecisionOption {
                    key: "b".to_string(),
                    label: b_label,
                    action: b_action,
                },
            ];
            if let Some(action) = c_action {
                options.push(DecisionOption {
                    key: "c".to_string(),
                    label: c_label.unwrap_or_else(|| "Option C".to_string()),
                    action,
                });
            }
            let prompt = service.create_decision_prompt(
                &source,
                &sender,
                &channel,
                &question,
                options.clone(),
            )?;
            let msg_body = format_decision_prompt_message(&prompt);
            let msg = OutboundMessage::new(
                provider,
                channel,
                format!("Decision Prompt {}", prompt.prompt_id),
                msg_body,
            );
            let out = hermes_call_or_local_async(
                &default_socket_path,
                "send",
                serde_json::to_value(&msg)?,
                || async { Ok(service.send(msg).await?) },
            )
            .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "prompt": prompt,
                    "dispatch": out
                }))?
            );
        }
        HermesCommands::CouncilOpen {
            topic,
            participants,
        } => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "council_open",
                serde_json::json!({
                    "topic": topic,
                    "participants": participants
                }),
                || Ok(service.council_open(&topic, participants)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::CouncilReport {
            session_id,
            from_agent,
            body,
        } => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "council_report",
                serde_json::json!({
                    "session_id": session_id,
                    "from_agent": from_agent,
                    "body": body
                }),
                || Ok(service.council_report(&session_id, &from_agent, &body)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::CouncilClose {
            session_id,
            outcome,
        } => {
            let out = hermes_call_or_local(
                &default_socket_path,
                "council_close",
                serde_json::json!({
                    "session_id": session_id,
                    "outcome": outcome
                }),
                || Ok(service.council_close(&session_id, &outcome)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::BoardroomQuorum {
            session_id,
            topic,
            evidence_path,
            oracle_query_id,
            oracle_verdict_path,
            charon_route_evidence,
            quorum_threshold,
            approval,
            render,
        } => {
            let packet = service.project_boardroom_quorum_packet(
                &session_id,
                &topic,
                evidence_path,
                oracle_query_id,
                oracle_verdict_path,
                charon_route_evidence,
                quorum_threshold,
                approval,
            )?;
            if render {
                println!("{}", service.render_boardroom_quorum_review_packet(&packet));
            } else {
                println!("{}", serde_json::to_string_pretty(&packet)?);
            }
        }
        HermesCommands::BoardroomQuorumDispatch {
            packet_id,
            provider,
            channel,
            operator_approval_note,
        } => {
            let out = service
                .dispatch_boardroom_quorum_packet(
                    &packet_id,
                    &provider,
                    &channel,
                    &operator_approval_note,
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HermesCommands::Paths => {
            let out =
                hermes_call_or_local(&default_socket_path, "paths", serde_json::json!({}), || {
                    Ok(service.paths())
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}
