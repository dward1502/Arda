use super::*;
use crate::types::CouncilApprovalDecision;

impl HermesService {
    pub fn council_command_seats(&self) -> Vec<CouncilCommandSeat> {
        vec![
            CouncilCommandSeat {
                seat: "first".to_string(),
                agent_id: "arandur".to_string(),
                role: "CEO/main orchestrator; final direction and broad situational command"
                    .to_string(),
                authority: "sovereign_direction".to_string(),
                use_when: "setting strategy, choosing priorities, and resolving executive direction"
                    .to_string(),
            },
            CouncilCommandSeat {
                seat: "second".to_string(),
                agent_id: "prometheus".to_string(),
                role: "executor pipeline; task lifecycle, routing, ledger, and accountability"
                    .to_string(),
                authority: "execution_coordination".to_string(),
                use_when: "turning decisions into bounded tasks and tracking completion evidence"
                    .to_string(),
            },
            CouncilCommandSeat {
                seat: "third".to_string(),
                agent_id: "counsel_or_oracle".to_string(),
                role: "Counsel for pressure-test review; Oracle for truth, validation, or triad judgment"
                    .to_string(),
                authority: "advisory_or_validation".to_string(),
                use_when: "select Counsel for second-order critique; select Oracle when factual validation or governance proof is required"
                    .to_string(),
            },
        ]
    }

    pub fn boardroom_post(&self, post: BoardroomPost) -> Result<()> {
        append_jsonl(&self.boardroom_path, &post)?;
        append_jsonl(
            &self.messages_path,
            &serde_json::json!({
                "direction": "boardroom",
                "post": post,
            }),
        )?;
        self.emit_memory_event(
            "boardroom_posted",
            &format!(
                "HERMES posted boardroom message from {} mentions={} topic={}",
                post.from_agent,
                if post.mentions.is_empty() {
                    "none".to_string()
                } else {
                    post.mentions.join(",")
                },
                post.body.chars().take(120).collect::<String>()
            ),
            Some(0.85),
            vec![
                "hermes".to_string(),
                "boardroom".to_string(),
                "checkpoint".to_string(),
                "decision".to_string(),
                format!("from_{}", post.from_agent.to_ascii_lowercase()),
            ],
        );
        self.emit_relationship_signal_background(
            post.from_agent.clone(),
            if post.mentions.is_empty() {
                "boardroom".to_string()
            } else {
                post.mentions[0].clone()
            },
            0.82,
            0.78,
            0.74,
            "boardroom_posted",
        );
        self.emit_work_signal_background(
            "hermes".to_string(),
            0.9,
            JouleWorkUnit::Attention,
            "boardroom_posted",
        );
        Ok(())
    }

    pub fn boardroom_recent(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let content = fs::read_to_string(&self.boardroom_path)?;
        let mut lines = content
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .collect::<Vec<_>>();
        lines.reverse();
        lines.truncate(limit.max(1));
        Ok(lines)
    }

    pub fn recent_council_sessions(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.council_sessions_path, limit)
    }

    pub fn record_council_discussion_note(
        &self,
        session_id: &str,
        agent: &str,
        summary: &str,
        risk: CommsEventRisk,
        source_class: &str,
    ) -> Result<CouncilDiscussionNote> {
        if contains_final_approval_claim(summary) {
            let _blocked = self.record_comms_event(
                CommsEventType::OutboundProjection,
                "governance-audit",
                CommsEventVisibility::Internal,
                CommsEventRisk::High,
                &format!(
                    "Blocked council discussion from {agent}: attempted to present discussion as final approval"
                ),
                vec![format!("council_session:{session_id}")],
                PromotionState::Blocked,
                true,
            )?;
            return Err(AnnunimasError::Task(
                "council discussion cannot present itself as final approval".to_string(),
            ));
        }
        if source_class.eq_ignore_ascii_case("local_inference") && risk != CommsEventRisk::Low {
            let _blocked = self.record_comms_event(
                CommsEventType::OutboundProjection,
                "governance-audit",
                CommsEventVisibility::Internal,
                CommsEventRisk::High,
                &format!(
                    "Blocked local inference council note from {agent}: only low-risk summaries are permitted"
                ),
                vec![format!("council_session:{session_id}")],
                PromotionState::Blocked,
                true,
            )?;
            return Err(AnnunimasError::Task(
                "local inference council notes must remain low-risk summaries".to_string(),
            ));
        }

        let created_at_utc = Utc::now().to_rfc3339();
        let note_id = format!(
            "council_note_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let note = CouncilDiscussionNote {
            schema_version: "annunimas.hermes.council_discussion_note.v1".to_string(),
            note_id: note_id.clone(),
            session_id: session_id.to_string(),
            agent: agent.to_string(),
            summary: format!("discussion-only: {summary}"),
            risk: risk.clone(),
            source_class: source_class.to_string(),
            semantic_channel: "council".to_string(),
            discussion_only: true,
            promotion_state: PromotionState::Unpromoted,
            canonical_refs: vec![note_id.clone(), format!("council_session:{session_id}")],
            created_at_utc,
        };
        append_jsonl(&self.council_sessions_path, &note)?;
        let _event = self.record_comms_event(
            CommsEventType::Status,
            "council",
            CommsEventVisibility::OperatorVisible,
            risk,
            &note.summary,
            note.canonical_refs.clone(),
            PromotionState::Unpromoted,
            true,
        )?;
        Ok(note)
    }

    pub fn render_council_discussion_projection(
        &self,
        note: &CouncilDiscussionNote,
        surface: &str,
    ) -> CouncilDiscussionProjection {
        let dispatch_channel = self
            .resolve_semantic_discord_channel(&note.semantic_channel)
            .map(|resolution| resolution.discord_recipient)
            .unwrap_or_else(|_| "boardroom".to_string());
        let title = format!("Council discussion-only note: {}", note.session_id);
        let body = truncate_projection_body(&format!(
            "Discussion-only council note from {}. Not approved; not a task proposal unless promoted through a canonical task event.\n\n{}",
            note.agent, note.summary
        ));
        CouncilDiscussionProjection {
            surface: surface.to_string(),
            semantic_channel: note.semantic_channel.clone(),
            dispatch_channel,
            note_id: note.note_id.clone(),
            session_id: note.session_id.clone(),
            title,
            body,
        }
    }

    pub fn promote_council_discussion_to_task(
        &self,
        note: &CouncilDiscussionNote,
        task_ref: &str,
    ) -> Result<CouncilDiscussionPromotion> {
        if !task_ref.starts_with("task:") {
            let _blocked = self.record_comms_event(
                CommsEventType::OutboundProjection,
                "governance-audit",
                CommsEventVisibility::Internal,
                CommsEventRisk::High,
                &format!(
                    "Blocked council note promotion for {}: missing canonical task event",
                    note.note_id
                ),
                vec![note.note_id.clone(), format!("candidate_ref:{task_ref}")],
                PromotionState::Blocked,
                true,
            )?;
            return Err(AnnunimasError::Task(
                "council discussion promotion requires a canonical task event ref".to_string(),
            ));
        }
        let promotion_id = format!(
            "council_promotion_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let promotion = CouncilDiscussionPromotion {
            schema_version: "annunimas.hermes.council_discussion_promotion.v1".to_string(),
            promotion_id: promotion_id.clone(),
            note_id: note.note_id.clone(),
            session_id: note.session_id.clone(),
            task_ref: task_ref.to_string(),
            promotion_state: PromotionState::Projected,
            is_authoritative: false,
            canonical_write_authorized: false,
            queue_mutated: false,
            requires_human_approval: true,
            authority_boundary: "discord_projection_only_not_canonical_authority".to_string(),
            canonical_refs: vec![note.note_id.clone(), task_ref.to_string(), promotion_id],
            promoted_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.council_sessions_path, &promotion)?;
        let _event = self.record_comms_event(
            CommsEventType::OutboundProjection,
            "tasks",
            CommsEventVisibility::OperatorVisible,
            note.risk.clone(),
            &format!(
                "Promoted council discussion {} to canonical task event {}",
                note.note_id, task_ref
            ),
            promotion.canonical_refs.clone(),
            PromotionState::Projected,
            true,
        )?;
        Ok(promotion)
    }

    pub fn record_council_approval_decision(
        &self,
        session_id: &str,
        promotion_id: Option<&str>,
        note_id: Option<&str>,
        approver: &str,
        approved: bool,
        status: &str,
        reason: &str,
    ) -> Result<CouncilApprovalDecision> {
        let decision_id = format!(
            "council_approval_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let mut canonical_refs = vec![format!("council_session:{session_id}"), decision_id.clone()];
        if let Some(promotion_id) = promotion_id {
            canonical_refs.push(promotion_id.to_string());
        }
        if let Some(note_id) = note_id {
            canonical_refs.push(note_id.to_string());
        }
        let decision = CouncilApprovalDecision {
            schema_version: "annunimas.hermes.council_approval_decision.v1".to_string(),
            decision_id,
            session_id: session_id.to_string(),
            promotion_id: promotion_id.map(ToOwned::to_owned),
            note_id: note_id.map(ToOwned::to_owned),
            approver: approver.to_string(),
            approved,
            status: status.to_string(),
            reason: reason.to_string(),
            canonical_refs,
            created_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.council_sessions_path, &decision)?;
        let _event = self.record_comms_event(
            CommsEventType::Status,
            "council",
            CommsEventVisibility::OperatorVisible,
            if approved {
                CommsEventRisk::Low
            } else {
                CommsEventRisk::Medium
            },
            &format!("council approval decision: {status} - {reason}"),
            decision.canonical_refs.clone(),
            PromotionState::Unpromoted,
            true,
        )?;
        Ok(decision)
    }

    pub fn approve_council_promotion(
        &self,
        session_id: &str,
        promotion_id: &str,
        approver: &str,
    ) -> Result<CouncilApprovalDecision> {
        self.record_council_approval_decision(
            session_id,
            Some(promotion_id),
            None,
            approver,
            true,
            "approved",
            "operator approval granted",
        )
    }

    pub fn reject_council_promotion(
        &self,
        session_id: &str,
        promotion_id: &str,
        approver: &str,
        reason: &str,
    ) -> Result<CouncilApprovalDecision> {
        self.record_council_approval_decision(
            session_id,
            Some(promotion_id),
            None,
            approver,
            false,
            "rejected",
            reason,
        )
    }

    pub fn approve_council_note(
        &self,
        session_id: &str,
        note_id: &str,
        approver: &str,
    ) -> Result<CouncilApprovalDecision> {
        self.record_council_approval_decision(
            session_id,
            None,
            Some(note_id),
            approver,
            true,
            "approved",
            "operator approval granted",
        )
    }

    pub fn route_local_council_summary(
        &self,
        session_id: &str,
        summary: &str,
        source_ref: Option<&str>,
        route_hint: Option<CharonRouteHint>,
    ) -> Result<LocalCouncilSummaryRoute> {
        if contains_final_approval_claim(summary) {
            let _blocked = self.record_comms_event(
                CommsEventType::OutboundProjection,
                "governance-audit",
                CommsEventVisibility::Internal,
                CommsEventRisk::High,
                &format!(
                    "Blocked local council summary for {session_id}: attempted authority decision"
                ),
                vec![format!("council_session:{session_id}")],
                PromotionState::Blocked,
                true,
            )?;
            return Err(AnnunimasError::Task(
                "local council summaries cannot carry authority decisions".to_string(),
            ));
        }

        let channel_resolution = self.resolve_semantic_discord_channel("council")?;
        let hint = route_hint.unwrap_or_else(default_charon_route_hint);
        let provider_used = hint
            .provider
            .clone()
            .filter(|provider| !provider.trim().is_empty())
            .unwrap_or_else(|| "charon-prod-default".to_string());
        let source_task = source_ref
            .map(str::trim)
            .filter(|candidate| candidate.starts_with("task:"))
            .map(ToOwned::to_owned);
        let promotable = source_task.is_some();
        let mut canonical_refs = vec![format!("council_session:{session_id}")];
        if let Some(task_ref) = &source_task {
            canonical_refs.push(task_ref.clone());
        }
        let estimated_tokens = match (hint.estimated_input_tokens, hint.estimated_output_tokens) {
            (Some(input), Some(output)) => Some(input.saturating_add(output)),
            (Some(input), None) => Some(input),
            (None, Some(output)) => Some(output),
            (None, None) => None,
        };
        let fallback_used = hint.fallback_used
            || hint
                .provider
                .as_ref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true);
        let fallback_reason = hint.fallback_reason.clone().or_else(|| {
            if fallback_used {
                Some(
                    "charon route hint unavailable; using default local council summary route"
                        .to_string(),
                )
            } else {
                None
            }
        });
        let route = LocalCouncilSummaryRoute {
            schema_version: "annunimas.hermes.local_council_summary_route.v1".to_string(),
            route_id: format!("lcsr_{}", uuid::Uuid::new_v4().simple()),
            session_id: session_id.to_string(),
            summary: format!("non-authoritative local inference summary: {summary}"),
            semantic_channel: channel_resolution.semantic_channel.clone(),
            dispatch_channel: channel_resolution.discord_recipient.clone(),
            output_classification: "low_risk_summary".to_string(),
            source_task,
            canonical_refs,
            promotable,
            is_authoritative: false,
            provider_used: Some(provider_used.clone()),
            model_used: hint.model.clone().filter(|model| !model.trim().is_empty()),
            route_evidence: hint.route_evidence.clone(),
            latency_ms: hint.latency_ms,
            estimated_tokens,
            fallback_metadata: LocalCouncilSummaryFallbackMetadata {
                provider: provider_used,
                reason: fallback_reason,
                fallback_used,
                semantic_channel_fallback_used: channel_resolution.fallback_used,
                semantic_channel_env_key: channel_resolution.env_key.clone(),
            },
            created_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.council_sessions_path, &route)?;
        let promotion_state = if route.promotable {
            PromotionState::Projected
        } else {
            PromotionState::Unpromoted
        };
        let _event = self.record_comms_event(
            CommsEventType::Status,
            &route.semantic_channel,
            CommsEventVisibility::OperatorVisible,
            CommsEventRisk::Low,
            &route.summary,
            route.canonical_refs.clone(),
            promotion_state,
            true,
        )?;
        Ok(route)
    }

    pub fn council_open(
        &self,
        topic: &str,
        participants: Vec<String>,
    ) -> Result<serde_json::Value> {
        let session_id = format!(
            "council_{}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        let now = Utc::now().to_rfc3339();
        append_jsonl(
            &self.council_sessions_path,
            &serde_json::json!({
                "event": "opened",
                "session_id": session_id,
                "topic": topic,
                "participants": participants,
                "ts_utc": now,
            }),
        )?;
        self.boardroom_post(BoardroomPost {
            from_agent: "prometheus".to_string(),
            message_type: "council_open".to_string(),
            priority: "high".to_string(),
            subject: format!("Council Session Opened: {topic}"),
            body: "Convening council gate discussion.".to_string(),
            mentions: vec![
                "athena".to_string(),
                "hades".to_string(),
                "charon".to_string(),
            ],
            thread_id: Some(session_id.clone()),
            posted_at_utc: Utc::now().to_rfc3339(),
        })?;
        Ok(serde_json::json!({
            "session_id": session_id,
            "topic": topic,
            "opened_at_utc": now
        }))
    }

    pub fn council_report(
        &self,
        session_id: &str,
        from_agent: &str,
        body: &str,
    ) -> Result<serde_json::Value> {
        append_jsonl(
            &self.council_sessions_path,
            &serde_json::json!({
                "event": "report",
                "session_id": session_id,
                "from_agent": from_agent,
                "body": body,
                "ts_utc": Utc::now().to_rfc3339(),
            }),
        )?;
        self.boardroom_post(BoardroomPost {
            from_agent: from_agent.to_string(),
            message_type: "council_report".to_string(),
            priority: "normal".to_string(),
            subject: format!("Council report from {}", from_agent),
            body: body.to_string(),
            mentions: vec!["prometheus".to_string()],
            thread_id: Some(session_id.to_string()),
            posted_at_utc: Utc::now().to_rfc3339(),
        })?;
        Ok(serde_json::json!({
            "session_id": session_id,
            "reported_by": from_agent,
            "accepted": true
        }))
    }

    pub fn council_close(&self, session_id: &str, outcome: &str) -> Result<serde_json::Value> {
        append_jsonl(
            &self.council_sessions_path,
            &serde_json::json!({
                "event": "closed",
                "session_id": session_id,
                "outcome": outcome,
                "ts_utc": Utc::now().to_rfc3339(),
            }),
        )?;
        self.boardroom_post(BoardroomPost {
            from_agent: "prometheus".to_string(),
            message_type: "council_close".to_string(),
            priority: "normal".to_string(),
            subject: "Council session closed".to_string(),
            body: outcome.to_string(),
            mentions: Vec::new(),
            thread_id: Some(session_id.to_string()),
            posted_at_utc: Utc::now().to_rfc3339(),
        })?;
        Ok(serde_json::json!({
            "session_id": session_id,
            "closed": true,
            "outcome": outcome
        }))
    }

    pub fn record_operating_room_event(
        &self,
        kind: OperatingRoomEventKind,
        topic: &str,
        subject: &str,
        body: &str,
        evidence_paths: Vec<String>,
        discord_projection_permitted: bool,
    ) -> Result<OperatingRoomEvent> {
        let safety_state = if kind == OperatingRoomEventKind::Command {
            "blocked_action"
        } else {
            "observe_only"
        };
        let event = OperatingRoomEvent {
            schema_version: "annunimas.hermes.operating_room_event.v1".to_string(),
            event_id: format!("ore_{}", uuid::Uuid::new_v4().simple()),
            kind,
            topic: topic.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            evidence_paths,
            safety_state: safety_state.to_string(),
            discord_projection_permitted,
            created_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.root.join("operating_room_events.jsonl"), &event)?;
        Ok(event)
    }

    pub fn render_operating_room_event_for_discord(&self, event: &OperatingRoomEvent) -> String {
        let receipt = event
            .evidence_paths
            .first()
            .map(String::as_str)
            .unwrap_or("missing");
        let next_action = if event.safety_state == "observe_only" {
            "observe canonical state and act through Annunimas CLI/operator gates"
        } else {
            "do not execute from Discord; require local operator approval"
        };
        let redacted_body = redact_operating_room_body(&event.body);
        let mut rendered = format!(
            "HERMES operating room event\ntrace: {}\nkind: {}\ntopic: {}\nsubject: {}\nsafety: {}\nreceipt: {}\nnext action: {}\nsummary: {}",
            event.event_id,
            event.kind,
            event.topic,
            event.subject,
            event.safety_state,
            receipt,
            next_action,
            redacted_body,
        );
        if rendered.len() > 1900 {
            rendered.truncate(1897);
            rendered.push_str("...");
        }
        rendered
    }

    pub async fn dispatch_operating_room_event_to_discord(
        &self,
        event_id: &str,
        channel: &str,
    ) -> Result<serde_json::Value> {
        let channel_resolution = self.resolve_semantic_discord_channel(channel)?;
        let event = self.find_operating_room_event(event_id)?;
        if !event.discord_projection_permitted {
            return self.record_blocked_operating_room_dispatch(
                &event,
                &channel_resolution,
                "discord_projection_not_permitted",
            );
        }
        if event.safety_state != "observe_only" || event.kind == OperatingRoomEventKind::Command {
            return self.record_blocked_operating_room_dispatch(
                &event,
                &channel_resolution,
                "unsafe_action_projection_blocked",
            );
        }

        let comms_event =
            self.record_operating_room_comms_event(&event, &channel_resolution.semantic_channel)?;
        let body = self.render_operating_room_event_for_discord(&event);
        let mut msg = OutboundMessage::new(
            "discord",
            channel_resolution.discord_recipient.clone(),
            format!("Operating room event: {}", event.subject),
            body,
        );
        msg.priority = "normal".to_string();
        let send_result = self.send(msg).await?;
        let dispatch = serde_json::json!({
            "event_id": event.event_id,
            "topic": event.topic,
            "queued": true,
            "dispatch_provider": "discord",
            "dispatch_channel": channel_resolution.discord_recipient.clone(),
            "semantic_channel": channel_resolution.semantic_channel.clone(),
            "semantic_channel_fallback_used": channel_resolution.fallback_used,
            "semantic_channel_env_key": channel_resolution.env_key.clone(),
            "comms_event_id": comms_event.event_id,
            "dispatched_at_utc": Utc::now().to_rfc3339(),
            "send_result": send_result,
        });
        append_jsonl(
            &self.root.join("operating_room_dispatches.jsonl"),
            &dispatch,
        )?;
        Ok(dispatch)
    }

    fn find_operating_room_event(&self, event_id: &str) -> Result<OperatingRoomEvent> {
        let events_path = self.root.join("operating_room_events.jsonl");
        let content = fs::read_to_string(&events_path).map_err(AnnunimasError::Ledger)?;
        for line in content.lines().filter(|line| !line.trim().is_empty()).rev() {
            let event: OperatingRoomEvent = match serde_json::from_str(line) {
                Ok(event) => event,
                Err(_) => continue,
            };
            if event.event_id == event_id {
                return Ok(event);
            }
        }
        Err(AnnunimasError::Task(format!(
            "operating room event not found: {event_id}"
        )))
    }

    fn record_blocked_operating_room_dispatch(
        &self,
        event: &OperatingRoomEvent,
        channel_resolution: &SemanticChannelResolution,
        blocked_reason: &str,
    ) -> Result<serde_json::Value> {
        let comms_event = self.record_blocked_operating_room_comms_event(
            event,
            &channel_resolution.semantic_channel,
            blocked_reason,
        )?;
        let dispatch = serde_json::json!({
            "event_id": event.event_id,
            "topic": event.topic,
            "queued": false,
            "dispatch_provider": "discord",
            "dispatch_channel": channel_resolution.discord_recipient.clone(),
            "semantic_channel": channel_resolution.semantic_channel.clone(),
            "semantic_channel_fallback_used": channel_resolution.fallback_used,
            "semantic_channel_env_key": channel_resolution.env_key.clone(),
            "blocked_reason": blocked_reason,
            "comms_event_id": comms_event.event_id,
            "dispatched_at_utc": Utc::now().to_rfc3339(),
        });
        append_jsonl(
            &self.root.join("operating_room_dispatches.jsonl"),
            &dispatch,
        )?;
        Ok(dispatch)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "operator quorum packet mirrors CLI/API fields and call-site compatibility"
    )]
    pub fn project_boardroom_quorum_packet(
        &self,
        session_id: &str,
        topic: &str,
        evidence_paths: Vec<String>,
        oracle_query_id: Option<String>,
        oracle_verdict_path: Option<PathBuf>,
        charon_route_evidence: Option<String>,
        quorum_threshold: usize,
        approvals: Vec<String>,
    ) -> Result<BoardroomQuorumPacket> {
        let oracle = self.boardroom_oracle_link(oracle_query_id, oracle_verdict_path)?;
        let charon_route = parse_charon_route_evidence(charon_route_evidence);
        let mut review_reasons = Vec::new();
        if !oracle.verdict_found {
            review_reasons.push("oracle_verdict_missing".to_string());
        } else if oracle.outcome.as_deref() != Some("Pass") {
            review_reasons.push(format!(
                "oracle_outcome_not_pass:{}",
                oracle.outcome.as_deref().unwrap_or("missing")
            ));
        }
        if approvals.len() < quorum_threshold {
            review_reasons.push(format!(
                "quorum_threshold_unmet:{}/{}",
                approvals.len(),
                quorum_threshold
            ));
        }
        if charon_route.selected_provider.is_none() || charon_route.selected_model.is_none() {
            review_reasons.push("charon_route_evidence_missing".to_string());
        }
        let (quorum_result, status_reason) = if review_reasons.is_empty() {
            (
                "passed".to_string(),
                "oracle_quorum_and_charon_route_verified".to_string(),
            )
        } else {
            ("review_required".to_string(), review_reasons.join(";"))
        };
        let packet = BoardroomQuorumPacket {
            schema_version: "annunimas.hermes.boardroom_quorum.v1".to_string(),
            packet_id: format!("boardroom_quorum_{}", uuid::Uuid::new_v4().simple()),
            session_id: session_id.to_string(),
            topic: topic.to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            evidence_paths,
            oracle,
            quorum: BoardroomQuorumDecision {
                threshold: quorum_threshold,
                approvals,
                result: quorum_result.clone(),
            },
            charon_route,
            discord_projection_permitted: false,
            operator_approval_required: true,
            operator_approved: false,
            status: quorum_result,
            status_reason,
        };
        append_jsonl(&self.root.join("boardroom_quorum_packets.jsonl"), &packet)?;
        Ok(packet)
    }

    pub fn render_boardroom_quorum_review_packet(&self, packet: &BoardroomQuorumPacket) -> String {
        let provider = packet
            .charon_route
            .selected_provider
            .as_deref()
            .unwrap_or("unresolved");
        let model = packet
            .charon_route
            .selected_model
            .as_deref()
            .unwrap_or("unresolved");
        let oracle = packet.oracle.query_id.as_deref().unwrap_or("missing");
        let approval = if packet.operator_approval_required && !packet.operator_approved {
            "required"
        } else if packet.operator_approved {
            "approved"
        } else {
            "not_required"
        };
        let mut rendered = format!(
            "HERMES boardroom quorum review\ntrace: {}\nsession: {}\ntopic: {}\nstatus: {}\nreason: {}\nquorum: {} approvals={}/{}\noracle: {} verdict_found={} outcome={} locator={} resonance={}\ncharon: {} / {}\ndiscord projection: {}\noperator approval: {}",
            packet.packet_id,
            packet.session_id,
            packet.topic,
            packet.status,
            packet.status_reason,
            packet.quorum.result,
            packet.quorum.approvals.len(),
            packet.quorum.threshold,
            oracle,
            packet.oracle.verdict_found,
            packet.oracle.outcome.as_deref().unwrap_or("missing"),
            packet.oracle.verdict_locator.as_deref().unwrap_or("missing"),
            packet
                .oracle
                .resonance_score
                .map(|score| format!("{score:.3}"))
                .unwrap_or_else(|| "missing".to_string()),
            provider,
            model,
            packet.discord_projection_permitted,
            approval,
        );
        if rendered.len() > 1900 {
            rendered.truncate(1897);
            rendered.push_str("...");
        }
        rendered
    }

    pub async fn dispatch_boardroom_quorum_packet(
        &self,
        packet_id: &str,
        provider: &str,
        channel: &str,
        operator_approval_note: &str,
    ) -> Result<serde_json::Value> {
        let packet = self.find_boardroom_quorum_packet(packet_id)?;
        let approval_note = operator_approval_note.trim();
        if approval_note.is_empty() {
            return self.record_blocked_boardroom_quorum_dispatch(
                &packet,
                provider,
                channel,
                approval_note,
                "operator_approval_note_required",
            );
        }
        if packet.status != "passed" {
            return self.record_blocked_boardroom_quorum_dispatch(
                &packet,
                provider,
                channel,
                approval_note,
                &format!("packet_status_not_passed:{}", packet.status),
            );
        }

        let rendered = self.render_boardroom_quorum_review_packet(&packet);
        let mut msg = OutboundMessage::new(
            provider,
            channel,
            format!("Boardroom quorum review: {}", packet.topic),
            rendered,
        );
        msg.priority = "high".to_string();
        let send_result = self.send(msg).await?;
        let dispatch = serde_json::json!({
            "packet_id": packet.packet_id,
            "session_id": packet.session_id,
            "topic": packet.topic,
            "queued": true,
            "dispatch_provider": provider,
            "dispatch_channel": channel,
            "operator_approved": true,
            "operator_approval_note": approval_note,
            "dispatched_at_utc": Utc::now().to_rfc3339(),
            "send_result": send_result,
        });
        append_jsonl(
            &self.root.join("boardroom_quorum_dispatches.jsonl"),
            &dispatch,
        )?;
        Ok(dispatch)
    }

    fn find_boardroom_quorum_packet(&self, packet_id: &str) -> Result<BoardroomQuorumPacket> {
        let packets_path = self.root.join("boardroom_quorum_packets.jsonl");
        let content = fs::read_to_string(&packets_path).map_err(AnnunimasError::Ledger)?;
        for line in content.lines().filter(|line| !line.trim().is_empty()).rev() {
            let packet: BoardroomQuorumPacket = match serde_json::from_str(line) {
                Ok(packet) => packet,
                Err(_) => continue,
            };
            if packet.packet_id == packet_id {
                return Ok(packet);
            }
        }
        Err(AnnunimasError::Task(format!(
            "boardroom quorum packet not found: {packet_id}"
        )))
    }

    fn record_blocked_boardroom_quorum_dispatch(
        &self,
        packet: &BoardroomQuorumPacket,
        provider: &str,
        channel: &str,
        operator_approval_note: &str,
        blocked_reason: &str,
    ) -> Result<serde_json::Value> {
        let dispatch = serde_json::json!({
            "packet_id": packet.packet_id,
            "session_id": packet.session_id,
            "topic": packet.topic,
            "queued": false,
            "dispatch_provider": provider,
            "dispatch_channel": channel,
            "operator_approved": false,
            "operator_approval_note": operator_approval_note,
            "blocked_reason": blocked_reason,
            "dispatched_at_utc": Utc::now().to_rfc3339(),
        });
        append_jsonl(
            &self.root.join("boardroom_quorum_dispatches.jsonl"),
            &dispatch,
        )?;
        Ok(dispatch)
    }

    fn boardroom_oracle_link(
        &self,
        query_id: Option<String>,
        verdict_path: Option<PathBuf>,
    ) -> Result<BoardroomOracleLink> {
        let mut link = BoardroomOracleLink {
            query_id: query_id.clone(),
            verdict_locator: verdict_path.as_ref().map(|path| path.display().to_string()),
            verdict_found: false,
            outcome: None,
            triad_scores: BoardroomTriadScores::default(),
            resonance_score: None,
        };
        let Some(path) = verdict_path else {
            return Ok(link);
        };
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(link),
            Err(err) => return Err(AnnunimasError::Ledger(err)),
        };
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let matches_query = query_id
                .as_ref()
                .map(|id| value.get("query_id").and_then(|v| v.as_str()) == Some(id.as_str()))
                .unwrap_or(true);
            if !matches_query {
                continue;
            }
            link.verdict_found = true;
            link.outcome = value
                .get("outcome")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            link.resonance_score = value.get("resonance_score").and_then(|v| v.as_f64());
            link.triad_scores = BoardroomTriadScores {
                aurelius: score_from_gate(&value, "aurelius"),
                bacon: score_from_gate(&value, "bacon"),
                sun_tzu: score_from_gate(&value, "sun_tzu"),
            };
            break;
        }
        Ok(link)
    }
}

fn contains_final_approval_claim(summary: &str) -> bool {
    let lower = summary.to_ascii_lowercase();
    let has_approval = lower.contains("final approval")
        || lower.contains("approved")
        || lower.contains("approval granted")
        || lower.contains("execute immediately");
    let has_discussion_boundary = lower.contains("not final approval")
        || lower.contains("not approved")
        || lower.contains("discussion-only")
        || lower.contains("discussion only");
    has_approval && !has_discussion_boundary
}

fn truncate_projection_body(body: &str) -> String {
    const MAX_PROJECTION_CHARS: usize = 600;
    let mut output = body.chars().take(MAX_PROJECTION_CHARS).collect::<String>();
    if body.chars().count() > MAX_PROJECTION_CHARS {
        output.push('…');
    }
    output
}

fn default_charon_route_hint() -> CharonRouteHint {
    let provider = std::env::var("ANNUNIMAS_COUNCIL_DEFAULT_PROVIDER")
        .ok()
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    CharonRouteHint {
        provider: provider.clone(),
        model: None,
        route_evidence: Some(
            "fallback default route; charon region/hint unavailable".to_string(),
        ),
        latency_ms: None,
        estimated_input_tokens: None,
        estimated_output_tokens: None,
        fallback_used: provider.is_none(),
        fallback_reason: provider.map(|_| {
            "local model probe issued malformed /v1/chat/completions response; using configured council default provider".to_string()
        }),
    }
}

pub(super) fn redact_operating_room_body(body: &str) -> String {
    body.split_whitespace()
        .map(|token| {
            let lower = token.to_ascii_lowercase();
            if lower.contains("token=")
                || lower.contains("secret=")
                || lower.contains("api_key=")
                || lower.contains("apikey=")
                || lower.contains("password=")
                || lower.contains("passwd=")
                || lower.starts_with("ghp_")
                || lower.starts_with("gho_")
                || lower.starts_with("github_pat_")
                || lower.starts_with("xoxb-")
                || lower.starts_with("discord.")
            {
                "[REDACTED]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_charon_route_evidence(route: Option<String>) -> BoardroomCharonRouteEvidence {
    let (selected_provider, selected_model) = route
        .as_deref()
        .and_then(|raw| raw.split_once(':'))
        .map(|(provider, model)| (Some(provider.to_string()), Some(model.to_string())))
        .unwrap_or((None, None));
    BoardroomCharonRouteEvidence {
        route_evidence: route,
        selected_provider,
        selected_model,
    }
}

fn score_from_gate(value: &serde_json::Value, gate: &str) -> Option<f64> {
    value
        .get("gates")
        .and_then(|gates| gates.get(gate))
        .and_then(|gate_value| gate_value.get("score"))
        .and_then(|score| score.as_f64())
}
