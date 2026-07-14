use super::*;

impl HermesService {
    fn provider_poll_limit() -> usize {
        std::env::var("ANNUNIMAS_HERMES_POLL_MAX_CONCURRENCY")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    }

    pub async fn poll_providers_once(&self) -> Result<usize> {
        let Some(result) = try_run_bounded_async(
            "hermes_provider_poll",
            Self::provider_poll_limit(),
            || async move {
                let messages = self.providers.poll_once().await?;
                let mut count = 0usize;
                for (provider_id, inbound) in messages {
                    if self
                        .ingest_polled_message(&provider_id, inbound, false)
                        .await?
                    {
                        count += 1;
                    }
                }
                Ok(count)
            },
        )
        .await
        else {
            return Err(AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: "provider poll concurrency gate saturated".to_string(),
            });
        };

        result
    }

    pub fn ingest_external(
        &self,
        provider: &str,
        sender: &str,
        content: &str,
        channel: Option<String>,
        is_illuvatar: bool,
    ) -> Result<IntentResult> {
        let mut msg = InboundMessage::new(provider.to_string(), sender.to_string(), content);
        msg.channel = channel;
        msg.is_illuvatar = is_illuvatar;
        self.classify(msg)
    }

    pub(super) fn charon_route_hint(&self, msg: &InboundMessage) -> Option<String> {
        let socket_path = std::env::var("ANNUNIMAS_CHARON_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/charon/charon.sock"));
        if !socket_path.exists() {
            return None;
        }
        let payload = serde_json::json!({
            "agent_id": "hermes",
            "task_type": "chat",
            "priority": "normal",
            "messages": [{
                "role": "user",
                "content": msg.content
            }],
            "options": {
                "strict": false,
                "workload_role": "orchestrator",
                "context_priority": "high",
                "quality_priority": "high",
                "cost_policy": "free_first",
                "privacy_requirement": "internal"
            }
        });
        self.send_charon_ipc(&socket_path, "route", payload)
            .ok()
            .and_then(|value| {
                let provider = value.get("provider_id").and_then(|v| v.as_str())?;
                let model = value.get("model_id").and_then(|v| v.as_str())?;
                Some(format!("{provider}:{model}"))
            })
    }

    pub(super) fn send_charon_ipc(
        &self,
        socket_path: &Path,
        cmd: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(socket_path).map_err(|e| AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "failed to connect to CHARON socket {}: {e}",
                socket_path.display()
            ),
        })?;
        let req = CommandEnvelope::new(cmd, payload);
        let mut encoded = serde_json::to_vec(&req)?;
        encoded.push(b'\n');
        stream
            .write_all(&encoded)
            .map_err(|e| AnnunimasError::Agent {
                agent: "charon".to_string(),
                message: format!("failed to write CHARON IPC request: {e}"),
            })?;
        let mut line = String::new();
        let mut reader = BufReader::new(stream);
        reader
            .read_line(&mut line)
            .map_err(|e| AnnunimasError::Agent {
                agent: "charon".to_string(),
                message: format!("failed to read CHARON IPC response: {e}"),
            })?;
        let response = serde_json::from_str::<ResponseEnvelope>(line.trim()).map_err(|e| {
            AnnunimasError::Agent {
                agent: "charon".to_string(),
                message: format!("invalid CHARON IPC response: {e}"),
            }
        })?;
        response.into_result("charon")
    }

    async fn ingest_polled_message(
        &self,
        provider_id: &str,
        inbound: McpMessage,
        is_illuvatar: bool,
    ) -> Result<bool> {
        if inbound.sender_is_bot {
            return Ok(false);
        }
        let dedup_key = format!("{provider_id}:{}", inbound.id);
        {
            let mut seen = self.seen_inbound_ids.lock().await;
            if seen.contains(&dedup_key) {
                return Ok(false);
            }
            seen.insert(dedup_key);
        }

        let mut msg = InboundMessage::new(provider_id, inbound.sender, inbound.content);
        msg.channel = inbound
            .channel_target
            .clone()
            .or_else(|| Some(format!("{}", inbound.channel)));
        msg.is_illuvatar = is_illuvatar;
        let mut decision_ctx: Option<DecisionExecutionContext> = None;
        if let Some(choice) = normalize_choice(&msg.content) {
            if let Some((prompt, option)) = self.resolve_decision_choice(
                provider_id,
                &msg.sender,
                msg.channel.as_deref().unwrap_or(""),
                &choice,
            ) {
                self.record_decision_hop(
                    "choice_resolved",
                    provider_id,
                    msg.channel.as_deref().unwrap_or(""),
                    &msg.sender,
                    Some(&prompt.prompt_id),
                    Some(&choice),
                    Some(&option.action),
                    None,
                    true,
                    None,
                );
                msg.content = option.action.clone();
                decision_ctx = Some(DecisionExecutionContext {
                    prompt_id: prompt.prompt_id,
                    choice,
                    selected_action: option.action,
                    selected_label: option.label,
                });
            }
        }

        let _ = self.classify(msg.clone())?;
        self.record_decision_hop(
            "inbound_classified",
            provider_id,
            msg.channel.as_deref().unwrap_or(""),
            &msg.sender,
            decision_ctx.as_ref().map(|d| d.prompt_id.as_str()),
            decision_ctx.as_ref().map(|d| d.choice.as_str()),
            decision_ctx.as_ref().map(|d| d.selected_action.as_str()),
            None,
            true,
            None,
        );
        if msg.is_illuvatar && decision_ctx.is_none() {
            let _ = self.fanout_illuvatar_directive(provider_id, &msg).await;
        }
        if let Some(ctx) = decision_ctx {
            self.execute_decision_action(provider_id, &msg, &ctx)
                .await?;
        }
        self.maybe_send_illuvatar_decision_prompt(provider_id, &msg)
            .await?;
        Ok(true)
    }

    pub async fn fanout_illuvatar_directive(
        &self,
        source_provider: &str,
        msg: &InboundMessage,
    ) -> Result<serde_json::Value> {
        let expected_sender = std::env::var("ANNUNIMAS_ILLUVATAR_DISCORD_USER")
            .unwrap_or_else(|_| "illuvatar".to_string());
        if !msg.sender.eq_ignore_ascii_case(&expected_sender) {
            return Ok(serde_json::json!({"fanout": false, "reason": "sender_not_illuvatar"}));
        }
        if msg.content.trim().is_empty() {
            return Ok(serde_json::json!({"fanout": false, "reason": "empty_content"}));
        }

        let channel = msg
            .channel
            .clone()
            .unwrap_or_else(|| "boardroom".to_string());
        let mut provider_ids = self.providers.configured_provider_ids();
        provider_ids.sort();
        provider_ids.dedup();

        let mut results = Vec::new();
        let mut dispatched = 0usize;
        for provider in provider_ids {
            let mut outbound = OutboundMessage::new(
                provider.clone(),
                channel.clone(),
                format!("Illuvatar Directive ({source_provider})"),
                msg.content.clone(),
            );
            outbound.priority = "high".to_string();
            outbound.stream =
                provider.eq_ignore_ascii_case("discord") && outbound.body.len() > 1800;
            match self.send(outbound).await {
                Ok(res) => {
                    let ok = res
                        .get("dispatched")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if ok {
                        dispatched += 1;
                    }
                    results.push(serde_json::json!({
                        "provider": provider,
                        "dispatched": ok,
                        "result": res
                    }));
                }
                Err(err) => results.push(serde_json::json!({
                    "provider": provider,
                    "dispatched": false,
                    "error": err.to_string()
                })),
            }
        }

        let summary = format!(
            "Illuvatar directive fanout dispatched {}/{} providers.",
            dispatched,
            results.len()
        );
        let _ = self.boardroom_post(BoardroomPost {
            from_agent: "hermes".to_string(),
            message_type: "illuvatar_fanout".to_string(),
            priority: "high".to_string(),
            subject: "Illuvatar Fanout".to_string(),
            body: format!("{summary}\n\nDirective: {}", msg.content),
            mentions: vec![
                "prometheus".to_string(),
                "athena".to_string(),
                "hades".to_string(),
                "charon".to_string(),
                "mnemosyne".to_string(),
                "warden".to_string(),
            ],
            thread_id: None,
            posted_at_utc: Utc::now().to_rfc3339(),
        });
        self.emit_memory_event(
            "illuvatar_fanout",
            &summary,
            Some(0.9),
            vec![
                "hermes".to_string(),
                "illuvatar".to_string(),
                "fanout".to_string(),
            ],
        );
        Ok(serde_json::json!({
            "fanout": true,
            "source_provider": source_provider,
            "sender": msg.sender,
            "channel": channel,
            "dispatched": dispatched,
            "providers_total": results.len(),
            "results": results
        }))
    }

    pub(super) async fn execute_decision_action(
        &self,
        provider_id: &str,
        msg: &InboundMessage,
        ctx: &DecisionExecutionContext,
    ) -> Result<()> {
        let action = ctx.selected_action.trim().to_ascii_lowercase();
        let channel = msg.channel.as_deref().unwrap_or("discord");
        let sender = msg.sender.as_str();

        let report = if action == "drain queued tasks" {
            let limit = std::env::var("ANNUNIMAS_HERMES_DECISION_DRAIN_LIMIT")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(5);
            let result = self.drain_queued_tasks(limit, sender)?;
            let completed = result
                .completed
                .iter()
                .map(|task| {
                    if task.title.is_empty() {
                        task.task_id.clone()
                    } else {
                        format!("{} — {}", task.task_id, task.title)
                    }
                })
                .collect::<Vec<_>>();
            let completed_block = if completed.is_empty() {
                "none".to_string()
            } else {
                completed.join("\n")
            };
            let status = if result.completed.is_empty() {
                "no_change"
            } else {
                "completed"
            };
            format!(
                "Status: {status}\nWhat was done: Drained {} queued task(s) with limit {}.\nCompleted:\n{}\nRemaining queued tasks: {}",
                result.attempted,
                limit,
                completed_block,
                result.remaining
            )
        } else if let Some(task_id) = action.strip_prefix("execute queued task ") {
            let task_id = task_id.trim();
            let result = self.complete_queued_task(task_id, sender)?;
            if result.updated {
                format!(
                    "Status: completed\nWhat was done: Executed {} ({}) and marked task {}{} completed.",
                    ctx.selected_label,
                    ctx.choice,
                    result.task_id,
                    if result.title.is_empty() {
                        "".to_string()
                    } else {
                        format!(" — {}", result.title)
                    }
                )
            } else if result.found {
                format!(
                    "Status: no_change\nWhat was done: Evaluated {} ({}) and found task {}{} already not queued.",
                    ctx.selected_label,
                    ctx.choice,
                    result.task_id,
                    if result.title.is_empty() {
                        "".to_string()
                    } else {
                        format!(" — {}", result.title)
                    }
                )
            } else {
                format!(
                    "Status: failed\nWhat was done: Could not execute {} ({}) because task {} was not found in queue.",
                    ctx.selected_label, ctx.choice, task_id
                )
            }
        } else if action == "show top queued tasks and plan" {
            let top = self.load_queued_task_entries(3)?;
            if top.is_empty() {
                "Status: idle\nWhat was done: Checked queue and found no queued tasks. Entering chat mode until new tasks arrive.".to_string()
            } else {
                let lines = top
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        format!(
                            "{}. {} ({})",
                            i + 1,
                            t.title.as_deref().unwrap_or(&t.task_id),
                            t.task_id
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Status: planning\nWhat was done: Reviewed top queued tasks.\nTop queued tasks:\n{lines}\n\nReply A/B/C to continue."
                )
            }
        } else if action == "enter chat mode" {
            "Status: chat_mode\nWhat was done: Chat mode enabled. Waiting for direct instructions or new queued tasks.".to_string()
        } else {
            format!(
                "Status: acknowledged\nWhat was done: Action accepted but no direct executor is wired for '{}'.",
                ctx.selected_action
            )
        };

        self.record_decision_hop(
            "action_executed",
            provider_id,
            channel,
            sender,
            Some(&ctx.prompt_id),
            Some(&ctx.choice),
            Some(&ctx.selected_action),
            None,
            true,
            None,
        );
        let outbound = OutboundMessage::new(
            provider_id.to_string(),
            channel.to_string(),
            format!("Decision Completion {}", ctx.prompt_id),
            report.clone(),
        );
        let send_out = self.send(outbound).await?;
        let dispatched = send_out
            .get("dispatched")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.record_decision_hop(
            "completion_report_sent",
            provider_id,
            channel,
            sender,
            Some(&ctx.prompt_id),
            Some(&ctx.choice),
            Some(&ctx.selected_action),
            Some(&report),
            dispatched,
            send_out.get("error").and_then(|v| v.as_str()),
        );
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use crate::mcp::McpChannelType;
    use chrono::Utc;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[tokio::test]
    async fn ingest_polled_message_deduplicates_message_ids() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let inbound = McpMessage {
            id: "msg-1".to_string(),
            sender: "operator".to_string(),
            content: "blorb".to_string(),
            timestamp: Utc::now(),
            channel: McpChannelType::Discord,
            channel_target: Some("boardroom".to_string()),
            sender_is_bot: false,
        };

        let first = service
            .ingest_polled_message("discord", inbound.clone(), false)
            .await
            .expect("first ingest");
        let second = service
            .ingest_polled_message("discord", inbound, false)
            .await
            .expect("second ingest");

        assert!(first);
        assert!(!second);
    }
}
