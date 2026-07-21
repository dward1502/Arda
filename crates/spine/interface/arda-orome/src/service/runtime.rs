use super::*;

impl HermesService {
    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_root();
        match Self::new(&primary) {
            Ok(v) => Ok(v),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(PathBuf::from("data").join("hermes"))
            }
        }
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let messages_path = root.join("messages.jsonl");
        let boardroom_path = root.join("boardroom.jsonl");
        let outbound_queue_path = root.join("outbound_queue.jsonl");
        let interruptions_path = root.join("interruptions.jsonl");
        let reroute_deferred_path = root.join("reroute_deferred.jsonl");
        let reroute_dlq_path = root.join("reroute_dlq.jsonl");
        let reroute_metrics_path = root.join("reroute_metrics.jsonl");
        let reroute_acks_path = root.join("reroute_acks.jsonl");
        let decision_prompts_path = root.join("decision_prompts.jsonl");
        let decision_responses_path = root.join("decision_responses.jsonl");
        let decision_metrics_path = root.join("decision_metrics.jsonl");
        let comms_events_path = root.join("comms_events.jsonl");
        let calendar_cache_path = root.join("calendar_cache.jsonl");
        let council_sessions_path = root.join("council_sessions.jsonl");

        touch(&messages_path)?;
        touch(&boardroom_path)?;
        touch(&outbound_queue_path)?;
        touch(&interruptions_path)?;
        touch(&reroute_deferred_path)?;
        touch(&reroute_dlq_path)?;
        touch(&reroute_metrics_path)?;
        touch(&reroute_acks_path)?;
        touch(&decision_prompts_path)?;
        touch(&decision_responses_path)?;
        touch(&decision_metrics_path)?;
        touch(&comms_events_path)?;
        touch(&calendar_cache_path)?;
        touch(&council_sessions_path)?;

        Ok(Self {
            root,
            messages_path,
            boardroom_path,
            outbound_queue_path,
            interruptions_path,
            reroute_deferred_path,
            reroute_dlq_path,
            reroute_metrics_path,
            reroute_acks_path,
            decision_prompts_path,
            decision_responses_path,
            decision_metrics_path,
            comms_events_path,
            calendar_cache_path,
            council_sessions_path,
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
            providers: Arc::new(ProviderRuntime::from_defaults()),
            seen_inbound_ids: Arc::new(Mutex::new(HashSet::new())),
            reroute_timestamps: Arc::new(StdMutex::new(VecDeque::new())),
        })
    }

    pub fn calendar_sync(&self) -> Result<serde_json::Value> {
        let snapshot = serde_json::json!({
            "synced_at_utc": Utc::now().to_rfc3339(),
            "upcoming_events": [],
            "source": "stub_v1"
        });
        append_jsonl(&self.calendar_cache_path, &snapshot)?;
        Ok(snapshot)
    }

    pub fn paths(&self) -> serde_json::Value {
        serde_json::json!({
            "root": self.root,
            "messages": self.messages_path,
            "boardroom": self.boardroom_path,
            "outbound_queue": self.outbound_queue_path,
            "interruptions": self.interruptions_path,
            "reroute_deferred": self.reroute_deferred_path,
            "reroute_dlq": self.reroute_dlq_path,
            "reroute_metrics": self.reroute_metrics_path,
            "reroute_acks": self.reroute_acks_path,
            "decision_prompts": self.decision_prompts_path,
            "decision_responses": self.decision_responses_path,
            "decision_metrics": self.decision_metrics_path,
            "comms_events": self.comms_events_path,
            "calendar_cache": self.calendar_cache_path,
            "council_sessions": self.council_sessions_path,
            "l3_readiness_projection": self.l3_readiness_projection_path(),
        })
    }

    pub fn l3_readiness_projection(&self) -> Result<serde_json::Value> {
        let path = self.l3_readiness_projection_path();
        if !path.exists() {
            return Ok(serde_json::json!({
                "schema_version": "arda.l3-readiness-projection.missing.v1",
                "status": {
                    "level": "missing",
                    "bounded_mutation_ready": false,
                    "broad_mutation_authorized": false,
                    "external_side_effects_authorized": false,
                    "destructive_actions_authorized": false
                },
                "projection_policy": {
                    "read_only": true,
                    "operator_surface_only": true,
                    "grants_mutation_authority": false
                },
                "source_path": path.to_string_lossy()
            }));
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn l3_readiness_projection_path(&self) -> PathBuf {
        self.project_root_hint()
            .join("core/state/l3_readiness_projection.json")
    }

    fn project_root_hint(&self) -> PathBuf {
        if self.root.file_name().and_then(|name| name.to_str()) == Some("hermes")
            && self
                .root
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                == Some("data")
        {
            return self
                .root
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
        }
        self.root.clone()
    }

    pub(super) fn manwe_outbound_route(&self, msg: &OutboundMessage) -> Result<serde_json::Value> {
        let socket_path = std::env::var("ANNUNIMAS_MANWE_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/manwe/manwe.sock"));
        if !socket_path.exists() {
            return Err(ArdaError::Agent {
                agent: "manwe".to_string(),
                message: format!("manwe socket missing at {}", socket_path.display()),
            });
        }
        let payload = serde_json::json!({
            "agent_id": "hermes",
            "task_type": "chat",
            "priority": msg.priority,
            "messages": [{
                "role": "user",
                "content": format!("subject: {}\n\n{}", msg.subject, msg.body)
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
        self.send_manwe_ipc(&socket_path, "route", payload)
    }

    pub(super) fn emit_memory_event(
        &self,
        event_type: &str,
        content: &str,
        confidence_hint: Option<f64>,
        tags: Vec<String>,
    ) {
        if let Some(service) = &self.mnemosyne {
            let event = InformantEvent {
                informant_id: "hermes_mneme".to_string(),
                crate_name: "hermes".to_string(),
                event_type: event_type.to_string(),
                ts_utc: Utc::now().to_rfc3339(),
                content: content.to_string(),
                confidence_hint,
                tags,
            };
            if let Err(err) = service.encode(event) {
                tracing::debug!(error = %err, "failed to emit HERMES mnemosyne event");
            }
        }
    }
}
