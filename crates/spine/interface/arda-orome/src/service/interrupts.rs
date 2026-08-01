use super::*;

impl HermesService {
    pub fn retry_reroute_dlq(&self, limit: usize) -> Result<serde_json::Value> {
        let latest = self.load_dlq_latest();
        let mut pending = latest
            .into_values()
            .filter(|v| v.get("status").and_then(|s| s.as_str()) == Some("pending"))
            .collect::<Vec<_>>();
        pending.sort_by(|a, b| {
            let ats = a.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            let bts = b.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            ats.cmp(bts)
        });
        let mut recovered = 0usize;
        let mut failed = 0usize;
        let mut processed = 0usize;
        for entry in pending.into_iter().take(limit.max(1)) {
            let Some(dlq_id) = entry.get("dlq_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let payload = entry
                .get("payload")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            processed += 1;
            match self.send_prometheus_ipc("interrupt_reroute", payload.clone()) {
                Ok(result) => {
                    if let Err(err) = self.validate_reroute_ack_contract(&result) {
                        let err_msg = err.to_string();
                        let attempt =
                            entry.get("attempt").and_then(|v| v.as_u64()).unwrap_or(1) + 1;
                        append_jsonl(
                            &self.reroute_dlq_path,
                            &serde_json::json!({
                                "dlq_id": dlq_id,
                                "status": "pending",
                                "attempt": attempt,
                                "error": err_msg,
                                "payload": payload,
                                "ts_utc": Utc::now().to_rfc3339()
                            }),
                        )?;
                        self.record_reroute_metric(
                            "dlq_retry_ack_missing",
                            false,
                            None,
                            Some(&err_msg),
                        );
                        failed += 1;
                        continue;
                    }
                    self.record_reroute_ack("dlq_recovered", dlq_id, &result);
                    append_jsonl(
                        &self.reroute_dlq_path,
                        &serde_json::json!({
                            "dlq_id": dlq_id,
                            "status": "recovered",
                            "result": result,
                            "recovered_at_utc": Utc::now().to_rfc3339(),
                            "ts_utc": Utc::now().to_rfc3339()
                        }),
                    )?;
                    self.record_reroute_metric("dlq_recovered", true, None, None);
                    recovered += 1;
                }
                Err(err) => {
                    let attempt = entry.get("attempt").and_then(|v| v.as_u64()).unwrap_or(1) + 1;
                    append_jsonl(
                        &self.reroute_dlq_path,
                        &serde_json::json!({
                            "dlq_id": dlq_id,
                            "status": "pending",
                            "attempt": attempt,
                            "error": err.to_string(),
                            "payload": payload,
                            "ts_utc": Utc::now().to_rfc3339()
                        }),
                    )?;
                    self.record_reroute_metric(
                        "dlq_retry_failed",
                        false,
                        None,
                        Some(&err.to_string()),
                    );
                    failed += 1;
                }
            }
        }

        Ok(serde_json::json!({
            "processed": processed,
            "recovered": recovered,
            "failed": failed,
            "dlq_path": self.reroute_dlq_path,
        }))
    }

    pub fn recent_reroute_metrics(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.reroute_metrics_path, limit)
    }

    pub fn recent_reroute_acks(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.reroute_acks_path, limit)
    }

    pub fn interrupt(&self, msg: InterruptionMessage) -> Result<serde_json::Value> {
        let (disposition, disposition_reason) = classify_interruption_intent(&msg.content);
        let (policy_authorized, policy_reason) =
            evaluate_interrupt_authority(&msg.sender, &disposition);
        let triad = self.governance_triad_for_interrupt(&msg, &disposition);
        let triad_score = (triad.aurelius_score + triad.bacon_score + triad.sun_tzu_score) / 3.0;
        let context = self.resolve_interruption_context(&msg);
        let blocked_by_policy = !policy_authorized
            && matches!(
                disposition,
                InterruptionDisposition::Reroute | InterruptionDisposition::Override
            );
        let requires_operator_review = matches!(disposition, InterruptionDisposition::Override)
            || !triad.passed
            || blocked_by_policy;
        let policy_safe = (triad.passed
            || !matches!(disposition, InterruptionDisposition::Override))
            && !blocked_by_policy;
        let event_id = format!("int_{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
        let ts_utc = Utc::now().to_rfc3339();
        let recommended_action = if blocked_by_policy {
            "policy_blocked_escalation"
        } else {
            match disposition {
                InterruptionDisposition::Note => "record_note_and_continue",
                InterruptionDisposition::Reroute => "queue_reroute_for_next_router_cycle",
                InterruptionDisposition::Override => "escalate_override_for_policy_review",
            }
        };
        let reroute_result = if matches!(disposition, InterruptionDisposition::Reroute) {
            if blocked_by_policy {
                self.append_interrupt_policy_escalation(
                    &event_id,
                    &msg.sender,
                    &disposition,
                    &policy_reason,
                    &ts_utc,
                );
                Some(serde_json::json!({
                    "queued": 0,
                    "deferred": false,
                    "blocked": true,
                    "reason": policy_reason
                }))
            } else {
                let payload = serde_json::json!({
                    "event_id": &event_id,
                    "source": &msg.source,
                    "sender": &msg.sender,
                    "content": &msg.content,
                    "run_id": &msg.run_id,
                    "session_id": &msg.session_id,
                    "task_id": &msg.task_id,
                    "context": context.clone(),
                    "triad_passed": triad.passed,
                    "triad_score": triad_score,
                    "policy_safe": policy_safe,
                    "requires_operator_review": requires_operator_review
                });
                if self.allow_reroute_handoff() {
                    let start = Instant::now();
                    match self.send_prometheus_ipc("interrupt_reroute", payload.clone()) {
                        Ok(value) => {
                            if let Err(err) = self.validate_reroute_ack_contract(&value) {
                                let reason = format!("prometheus_ack_missing: {}", err);
                                let dlq_id = format!(
                                    "dlq_{}",
                                    &uuid::Uuid::new_v4().simple().to_string()[..10]
                                );
                                let _ = append_jsonl(
                                    &self.reroute_dlq_path,
                                    &serde_json::json!({
                                        "dlq_id": dlq_id,
                                        "status": "pending",
                                        "attempt": 1,
                                        "event_id": &event_id,
                                        "reason": reason,
                                        "payload": payload,
                                        "ts_utc": Utc::now().to_rfc3339()
                                    }),
                                );
                                self.record_reroute_metric(
                                    "ack_missing",
                                    false,
                                    Some(start.elapsed().as_millis() as u64),
                                    Some(&reason),
                                );
                                Some(serde_json::json!({
                                    "queued": 0,
                                    "deferred": false,
                                    "error": reason,
                                    "dlq_path": self.reroute_dlq_path
                                }))
                            } else {
                                self.record_reroute_ack("forwarded", &event_id, &value);
                                self.record_reroute_metric(
                                    "forwarded",
                                    true,
                                    Some(start.elapsed().as_millis() as u64),
                                    None,
                                );
                                Some(value)
                            }
                        }
                        Err(err) => {
                            let reason = format!("prometheus_handoff_failed: {}", err);
                            let dlq_id =
                                format!("dlq_{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
                            let _ = append_jsonl(
                                &self.reroute_dlq_path,
                                &serde_json::json!({
                                    "dlq_id": dlq_id,
                                    "status": "pending",
                                    "attempt": 1,
                                    "event_id": &event_id,
                                    "reason": reason,
                                    "payload": payload,
                                    "ts_utc": Utc::now().to_rfc3339()
                                }),
                            );
                            self.record_reroute_metric(
                                "failed",
                                false,
                                Some(start.elapsed().as_millis() as u64),
                                Some(&reason),
                            );
                            Some(serde_json::json!({
                                "queued": 0,
                                "deferred": false,
                                "error": reason,
                                "dlq_path": self.reroute_dlq_path
                            }))
                        }
                    }
                } else {
                    let reason = "reroute_rate_limited";
                    let payload_for_defer = payload.clone();
                    let _ = append_jsonl(
                        &self.reroute_deferred_path,
                        &serde_json::json!({
                            "event_id": &event_id,
                            "reason": reason,
                            "disposition": "reroute",
                            "payload": payload_for_defer,
                            "deferred_at_utc": &ts_utc
                        }),
                    );
                    self.record_reroute_metric("deferred", false, None, Some(reason));
                    Some(serde_json::json!({
                        "queued": 0,
                        "deferred": true,
                        "reason": reason,
                        "deferred_path": self.reroute_deferred_path,
                    }))
                }
            }
        } else if blocked_by_policy {
            self.append_interrupt_policy_escalation(
                &event_id,
                &msg.sender,
                &disposition,
                &policy_reason,
                &ts_utc,
            );
            Some(serde_json::json!({
                "queued": 0,
                "deferred": false,
                "blocked": true,
                "reason": policy_reason
            }))
        } else {
            None
        };

        append_jsonl(
            &self.messages_path,
            &serde_json::json!({
                "direction": "interrupt",
                "event_id": &event_id,
                "source": &msg.source,
                "sender": &msg.sender,
                "content": &msg.content,
                "received_at_utc": &msg.received_at_utc,
                "channel": &msg.channel,
                "run_id": &msg.run_id,
                "session_id": &msg.session_id,
                "task_id": &msg.task_id,
                "disposition": &disposition,
                "disposition_reason": &disposition_reason,
                "triad_passed": triad.passed,
                "triad_score": triad_score,
                "policy_safe": policy_safe,
                "requires_operator_review": requires_operator_review,
                "policy_authorized": policy_authorized,
                "policy_reason": &policy_reason,
                "recommended_action": recommended_action,
                "ts_utc": &ts_utc,
            }),
        )?;

        append_jsonl(
            &self.interruptions_path,
            &serde_json::json!({
                "event_id": &event_id,
                "disposition": &disposition,
                "disposition_reason": &disposition_reason,
                "policy_safe": policy_safe,
                "requires_operator_review": requires_operator_review,
                "policy_authorized": policy_authorized,
                "policy_reason": &policy_reason,
                "triad_passed": triad.passed,
                "triad_score": triad_score,
                "message": &msg,
                "context": context.clone(),
                "action": {
                    "recommended": recommended_action,
                    "executed": "none_non_blocking",
                },
                "ts_utc": &ts_utc,
            }),
        )?;

        self.append_warden_interruption_event(
            &event_id,
            &disposition,
            policy_safe,
            requires_operator_review,
            context.as_ref(),
            &ts_utc,
        );
        self.append_apollo_interrupt_hook(&event_id, &disposition, context.as_ref(), &ts_utc);
        self.emit_memory_event(
            "interruption_captured",
            &format!(
                "HERMES captured async interruption disposition={:?} policy_safe={} operator_review={} because {}",
                disposition,
                policy_safe,
                requires_operator_review,
                disposition_reason
            ),
            Some(if policy_safe { 0.9 } else { 0.6 }),
            vec![
                "hermes".to_string(),
                "interrupt".to_string(),
                "checkpoint".to_string(),
                format!("disposition_{:?}", disposition).to_ascii_lowercase(),
                if requires_operator_review {
                    "operator_review".to_string()
                } else {
                    "operator_safe".to_string()
                },
            ],
        );
        let interruption_trust = if policy_safe { 0.86 } else { 0.52 };
        let interruption_reciprocity = if requires_operator_review { 0.44 } else { 0.78 };
        let interruption_longevity = match disposition {
            InterruptionDisposition::Note => 0.62,
            InterruptionDisposition::Reroute => 0.7,
            InterruptionDisposition::Override => 0.56,
        };
        self.emit_relationship_signal_background(
            "hermes".to_string(),
            normalize_relationship_target(
                msg.channel.as_deref().unwrap_or(&msg.sender),
                &msg.source,
            ),
            interruption_trust,
            interruption_reciprocity,
            interruption_longevity,
            "interrupt_captured",
        );
        self.emit_work_signal_background(
            "hermes".to_string(),
            if policy_safe { 0.92 } else { 0.58 },
            JouleWorkUnit::Attention,
            "interrupt_captured",
        );

        if matches!(disposition, InterruptionDisposition::Reroute) {
            if let Some(result) = reroute_result.as_ref() {
                let deferred = result
                    .get("deferred")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let blocked = result
                    .get("blocked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !deferred && !blocked {
                    self.refresh_reroute_handoff_cooldown();
                }
            }
        }

        Ok(serde_json::json!({
            "captured": true,
            "event_id": event_id,
            "disposition": disposition,
            "policy_safe": policy_safe,
            "requires_operator_review": requires_operator_review,
            "policy_authorized": policy_authorized,
            "policy_reason": policy_reason,
            "triad_passed": triad.passed,
            "triad_score": triad_score,
            "recommended_action": recommended_action,
            "context": context,
            "reroute_result": reroute_result,
            "receipt": format!("Interruption {} recorded; active task execution unchanged.", ts_utc),
        }))
    }

    fn governance_triad_for_interrupt(
        &self,
        msg: &InterruptionMessage,
        disposition: &InterruptionDisposition,
    ) -> arda_governance::TriadResult {
        let mut task = Task::new(&msg.content, "interrupt");
        task.clarifications_requested = if msg.content.contains('?') { 1 } else { 0 };
        task.clarifications_resolved = if matches!(disposition, InterruptionDisposition::Note) {
            1
        } else {
            0
        };
        task.joule_cost_estimated = match disposition {
            InterruptionDisposition::Note => 0.8,
            InterruptionDisposition::Reroute => 1.6,
            InterruptionDisposition::Override => 2.4,
        };
        task.joule_cost_actual = task.joule_cost_estimated;
        let cfg = TriadConfig {
            strict: false,
            required_passes: Some(2),
        };
        triad_validate(&task, Some(&cfg))
    }

    fn allow_reroute_handoff(&self) -> bool {
        let max_per_sec = std::env::var("ANNUNIMAS_HERMES_REROUTE_MAX_PER_SEC")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5)
            .max(1);
        let now = Instant::now();
        let mut window = match self.reroute_timestamps.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        while let Some(front) = window.front() {
            if now.duration_since(*front).as_millis() >= 1000 {
                let _ = window.pop_front();
            } else {
                break;
            }
        }
        if window.len() >= max_per_sec {
            return false;
        }
        window.push_back(now);
        true
    }

    fn refresh_reroute_handoff_cooldown(&self) {
        let mut window = match self.reroute_timestamps.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if window.is_empty() {
            window.push_back(Instant::now());
        } else if let Some(back) = window.back_mut() {
            *back = Instant::now();
        }
    }

    fn record_reroute_metric(
        &self,
        event: &str,
        handed_off: bool,
        latency_ms: Option<u64>,
        reason: Option<&str>,
    ) {
        let _ = append_jsonl(
            &self.reroute_metrics_path,
            &serde_json::json!({
                "ts_utc": Utc::now().to_rfc3339(),
                "event": event,
                "handed_off": handed_off,
                "latency_ms": latency_ms,
                "reason": reason,
            }),
        );
    }

    fn validate_reroute_ack_contract(&self, value: &serde_json::Value) -> Result<()> {
        let ack = value.get("ack").ok_or_else(|| ArdaError::Agent {
            agent: "hermes".to_string(),
            message: "missing ack contract from prometheus interrupt_reroute".to_string(),
        })?;
        let acknowledged = ack
            .get("acknowledged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let ack_id = ack.get("ack_id").and_then(|v| v.as_str()).unwrap_or("");
        if !acknowledged || ack_id.trim().is_empty() {
            return Err(ArdaError::Agent {
                agent: "hermes".to_string(),
                message: "invalid ack contract from prometheus interrupt_reroute".to_string(),
            });
        }
        Ok(())
    }

    fn record_reroute_ack(&self, event: &str, correlation_id: &str, value: &serde_json::Value) {
        let _ = append_jsonl(
            &self.reroute_acks_path,
            &serde_json::json!({
                "ts_utc": Utc::now().to_rfc3339(),
                "event": event,
                "correlation_id": correlation_id,
                "ack": value.get("ack").cloned().unwrap_or(serde_json::json!({})),
                "queued": value.get("queued").cloned().unwrap_or(serde_json::json!(0)),
                "duplicates_ignored": value.get("duplicates_ignored").cloned().unwrap_or(serde_json::json!(0))
            }),
        );
    }

    fn resolve_interruption_context(&self, msg: &InterruptionMessage) -> Option<serde_json::Value> {
        let mut task_ids = Vec::new();
        if let Some(task_id) = msg.task_id.as_ref().filter(|v| !v.trim().is_empty()) {
            task_ids.push(task_id.clone());
        } else {
            let mut latest_by_task = std::collections::HashMap::<String, String>::new();
            for line in read_jsonl_lines(&default_prometheus_orders_path()) {
                let task_id = line
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                let status = line
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_ascii_lowercase());
                if let (Some(task_id), Some(status)) = (task_id, status) {
                    latest_by_task.insert(task_id, status);
                }
            }
            let mut active = latest_by_task
                .into_iter()
                .filter(|(_, status)| status == "open" || status == "assigned")
                .map(|(task_id, _)| task_id)
                .collect::<Vec<_>>();
            active.sort();
            task_ids.extend(active.into_iter().take(8));
        }

        let run_id = msg.run_id.clone().filter(|v| !v.trim().is_empty());
        let session_id = msg.session_id.clone().filter(|v| !v.trim().is_empty());
        let active_agents = active_agents_from_world_state();
        if run_id.is_none() && session_id.is_none() && task_ids.is_empty() && active_agents == 0 {
            return None;
        }
        Some(serde_json::json!({
            "run_id": run_id,
            "session_id": session_id,
            "task_ids": task_ids,
            "active_agents": active_agents,
            "source": if msg.task_id.is_some() { "explicit_task_link" } else { "runtime_artifacts" }
        }))
    }

    fn append_warden_interruption_event(
        &self,
        event_id: &str,
        disposition: &InterruptionDisposition,
        policy_safe: bool,
        requires_operator_review: bool,
        context: Option<&serde_json::Value>,
        ts_utc: &str,
    ) {
        let queue_path = default_warden_queue_path();
        if let Some(parent) = queue_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = append_jsonl(
            &queue_path,
            &serde_json::json!({
                "informant_id": "hermes_interrupt_bridge",
                "crate_name": "hermes",
                "event_type": "interrupt_captured",
                "ts_utc": ts_utc,
                "content": format!(
                    "async interruption {} captured disposition={:?} policy_safe={} operator_review={}",
                    event_id, disposition, policy_safe, requires_operator_review
                ),
                "confidence_hint": if policy_safe { 0.84 } else { 0.62 },
                "tags": ["hermes", "interrupt", "ceo_observability"],
                "payload": {
                    "event_id": event_id,
                    "disposition": disposition,
                    "policy_safe": policy_safe,
                    "requires_operator_review": requires_operator_review,
                    "context": context,
                }
            }),
        );
    }

    fn append_apollo_interrupt_hook(
        &self,
        event_id: &str,
        disposition: &InterruptionDisposition,
        context: Option<&serde_json::Value>,
        ts_utc: &str,
    ) {
        let hook_path = default_apollo_interrupt_hook_path();
        if let Some(parent) = hook_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = append_jsonl(
            &hook_path,
            &serde_json::json!({
                "event_id": event_id,
                "disposition": disposition,
                "context": context,
                "ts_utc": ts_utc,
                "source": "hermes_interrupt",
            }),
        );
    }

    fn append_interrupt_policy_escalation(
        &self,
        event_id: &str,
        sender: &str,
        disposition: &InterruptionDisposition,
        reason: &str,
        ts_utc: &str,
    ) {
        let escalations_path = default_prometheus_escalations_path();
        if let Some(parent) = escalations_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = append_jsonl(
            &escalations_path,
            &serde_json::json!({
                "ts_utc": ts_utc,
                "source": "hermes_interrupt_policy",
                "reason": "interrupt_authority_policy.denied",
                "severity": "warning",
                "event_id": event_id,
                "sender": sender,
                "disposition": disposition,
                "detail": reason
            }),
        );
    }

    fn load_dlq_latest(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut latest = std::collections::HashMap::new();
        for value in read_jsonl_lines(&self.reroute_dlq_path) {
            let Some(id) = value.get("dlq_id").and_then(|v| v.as_str()) else {
                continue;
            };
            latest.insert(id.to_string(), value);
        }
        latest
    }

    fn send_prometheus_ipc(
        &self,
        cmd: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let socket_path = std::env::var("ANNUNIMAS_PROMETHEUS_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/prometheus/prometheus.sock"));
        if !socket_path.exists() {
            return Err(ArdaError::Agent {
                agent: "prometheus".to_string(),
                message: format!("PROMETHEUS socket missing at {}", socket_path.display()),
            });
        }
        let mut stream = UnixStream::connect(&socket_path).map_err(|e| ArdaError::Agent {
            agent: "prometheus".to_string(),
            message: format!(
                "failed to connect to PROMETHEUS socket {}: {e}",
                socket_path.display()
            ),
        })?;
        let req = CommandEnvelope::new(cmd, payload);
        let mut encoded = serde_json::to_vec(&req)?;
        encoded.push(b'\n');
        stream.write_all(&encoded).map_err(|e| ArdaError::Agent {
            agent: "prometheus".to_string(),
            message: format!("failed to write PROMETHEUS IPC request: {e}"),
        })?;
        let mut line = String::new();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut line).map_err(|e| ArdaError::Agent {
            agent: "prometheus".to_string(),
            message: format!("failed to read PROMETHEUS IPC response: {e}"),
        })?;
        let response = serde_json::from_str::<ResponseEnvelope>(line.trim()).map_err(|e| {
            ArdaError::Agent {
                agent: "prometheus".to_string(),
                message: format!("invalid PROMETHEUS IPC response: {e}"),
            }
        })?;
        response.into_result("prometheus")
    }
}

pub(super) fn default_warden_queue_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_WARDEN_QUEUE_PATH") {
        return PathBuf::from(custom);
    }
    annunimas_root().join("data/warden/informant_queue.jsonl")
}

fn annunimas_root() -> PathBuf {
    arda_core::layout::arda_root_from(env!("CARGO_MANIFEST_DIR"))
}

pub(super) fn default_prometheus_orders_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_PROMETHEUS_ORDERS_PATH") {
        return PathBuf::from(custom);
    }
    annunimas_root().join("data/prometheus/orders.jsonl")
}

pub(super) fn default_apollo_interrupt_hook_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_APOLLO_INTERRUPT_QUEUE_PATH") {
        return PathBuf::from(custom);
    }
    annunimas_root().join("data/apollo/interruptions.jsonl")
}

pub(super) fn default_prometheus_escalations_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_PROMETHEUS_ESCALATIONS_PATH") {
        return PathBuf::from(custom);
    }
    annunimas_root().join("data/prometheus/escalations.jsonl")
}
