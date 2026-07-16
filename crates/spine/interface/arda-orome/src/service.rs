// sigil: REPAIR
use crate::intent::classify_message;
use crate::mcp::McpMessage;
use crate::provider::{DispatchReceipt, ProviderRuntime};
use crate::types::{
    BoardroomCharonRouteEvidence, BoardroomOracleLink, BoardroomPost, BoardroomQuorumDecision,
    BoardroomQuorumPacket, BoardroomTriadScores, CharonRouteHint, CommsEvent, CommsEventRisk,
    CommsEventType, CommsEventVisibility, CouncilCommandSeat, CouncilDiscussionNote,
    CouncilDiscussionProjection, CouncilDiscussionPromotion, InboundMessage, IntentResult,
    InterruptionDisposition, InterruptionMessage, LocalCouncilSummaryFallbackMetadata,
    LocalCouncilSummaryRoute, OperatingRoomEvent, OperatingRoomEventKind, OutboundMessage,
    PromotionState, SubagentCompletionPacket, SubagentCompletionProjection, TaskApprovalPacket,
    TaskApprovalProjection, TaskApprovalProposal,
};
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::task::Task;
use arda_core::{spawn_bounded_background, try_run_bounded_async};
use arda_governance::{record_bacon_lite, triad_validate, TriadConfig};
use arda_vaire::{InformantEvent, MnemosyneService};
use arda_plutus::{JouleWorkUnit, PlutusService};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::Mutex;

mod classification;
mod comms_event;
mod council;
mod decision;
mod inbound;
mod interrupts;
mod outbound;
mod queue_state;
mod runtime;
mod semantic_channel;
mod status;
mod subagent_completion;
mod support;
mod task_approval;
use decision::DecisionExecutionContext;
pub use decision::{DecisionOption, DecisionPrompt};
use outbound::count_outbound_queue_pending;
use queue_state::default_task_queue_path;
pub use semantic_channel::{
    ArdaHudProjection, ArdaHudProjectionContract, ArdaHudProjectionStateReceipt,
    DiscordChannelDryRunReceipt, DiscordChannelPermissionSummary, DiscordChannelPlan,
    DiscordChannelPlanEntry, SemanticChannel, SemanticChannelResolution,
};
pub use status::{AgentActivity, HermesStatus, HermesSubcomponent, MessageStats};
use support::*;

#[derive(Clone)]
pub struct HermesService {
    root: PathBuf,
    messages_path: PathBuf,
    boardroom_path: PathBuf,
    outbound_queue_path: PathBuf,
    interruptions_path: PathBuf,
    reroute_deferred_path: PathBuf,
    reroute_dlq_path: PathBuf,
    reroute_metrics_path: PathBuf,
    reroute_acks_path: PathBuf,
    decision_prompts_path: PathBuf,
    decision_responses_path: PathBuf,
    decision_metrics_path: PathBuf,
    comms_events_path: PathBuf,
    calendar_cache_path: PathBuf,
    council_sessions_path: PathBuf,
    mnemosyne: Option<MnemosyneService>,
    providers: Arc<ProviderRuntime>,
    seen_inbound_ids: Arc<Mutex<HashSet<String>>>,
    reroute_timestamps: Arc<StdMutex<VecDeque<std::time::Instant>>>,
}

impl HermesService {
    pub fn providers(&self) -> Vec<String> {
        self.providers.configured_provider_ids()
    }

    pub fn classify(&self, msg: InboundMessage) -> Result<IntentResult> {
        classification::classify(self, msg)
    }

    pub fn recent_interruptions(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.interruptions_path, limit)
    }

    pub fn recent_decision_metrics(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.decision_metrics_path, limit)
    }

    pub fn recent_comms_events(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.comms_events_path, limit)
    }
}
#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{append_jsonl, DecisionExecutionContext, HermesService};
    use crate::discord_health::DiscordBridgeReadinessState;
    use crate::mcp::{McpChannel, McpChannelError, McpMessage};
    use crate::provider::{ProviderConfig, ProviderRuntime, ProviderType};
    use crate::types::{
        BoardroomPost, CommsEventRisk, CommsEventType, CommsEventVisibility, InboundMessage,
        InterruptionMessage, OperatingRoomEventKind, OutboundMessage, PromotionState,
    };
    use arda_core::try_run_bounded_async;
    use arda_plutus::PlutusService;
    use async_trait::async_trait;
    use std::fs;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn l3_readiness_projection_loads_from_project_root_surface() {
        let dir = tempdir().expect("tempdir");
        let core_state = dir.path().join("core/state");
        fs::create_dir_all(&core_state).expect("core state");
        fs::create_dir_all(dir.path().join("data")).expect("data dir");
        fs::write(
            core_state.join("l3_readiness_projection.json"),
            r#"{
              "schema_version": "annunimas.l3-readiness-projection.v1",
              "status": {
                "level": "l3_safe_local_harness_proven_projection_only",
                "bounded_mutation_ready": true,
                "broad_mutation_authorized": false
              },
              "projection_policy": {
                "read_only": true,
                "grants_mutation_authority": false
              }
            }"#,
        )
        .expect("projection");
        let service = HermesService::new(dir.path().join("data/hermes")).expect("service");

        let projection = service.l3_readiness_projection().expect("projection");

        assert_eq!(
            projection.pointer("/status/bounded_mutation_ready"),
            Some(&serde_json::json!(true))
        );
        assert_eq!(
            projection.pointer("/status/broad_mutation_authorized"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            projection.pointer("/projection_policy/grants_mutation_authority"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            service.l3_readiness_projection_path(),
            core_state.join("l3_readiness_projection.json")
        );
    }

    #[derive(Default)]
    struct FakeDiscordChannel {
        sent: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl McpChannel for FakeDiscordChannel {
        async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError> {
            self.sent
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push((recipient.to_string(), message.to_string()));
            Ok(())
        }

        async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError> {
            Ok(Vec::new())
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn classify_send_and_status() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        std::env::set_var("ANNUNIMAS_PLUTUS_HOME", &plutus_home);
        let service = HermesService::new(dir.path()).expect("service");

        let _ = service
            .classify(InboundMessage::new("discord", "illuvatar", "status"))
            .expect("classify");
        let _ = service
            .send(OutboundMessage::new(
                "discord",
                "boardroom",
                "Test",
                "hello world",
            ))
            .await
            .expect("send");
        service
            .boardroom_post(BoardroomPost::new(
                "prometheus",
                "audit",
                "Decision",
                "routing complete",
            ))
            .expect("boardroom");
        std::thread::sleep(Duration::from_millis(150));

        let status = service.status().await.expect("status");
        assert!(status.messages_today.inbound >= 1);
        assert!(status.messages_today.outbound >= 1);
        assert!(status.boardroom_active);
        let mut plutus_status = serde_json::json!({});
        for _ in 0..20 {
            plutus_status = PlutusService::from_home(&plutus_home)
                .expect("plutus service")
                .status()
                .await
                .expect("plutus status");
            if plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default()
                >= 3
                && plutus_status["joulework"]["total"]
                    .as_f64()
                    .unwrap_or_default()
                    > 0.0
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default()
                >= 3
        );
        assert!(
            plutus_status["joulework"]["total"]
                .as_f64()
                .unwrap_or_default()
                > 0.0
        );
        std::env::remove_var("ANNUNIMAS_PLUTUS_HOME");
    }

    #[test]
    fn append_jsonl_serializes_concurrent_writers() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("messages.jsonl");
        let mut threads = Vec::new();
        for idx in 0..12usize {
            let path = path.clone();
            threads.push(thread::spawn(move || {
                for seq in 0..20usize {
                    append_jsonl(&path, &serde_json::json!({"idx": idx, "seq": seq}))
                        .expect("append");
                }
            }));
        }
        for handle in threads {
            handle.join().expect("join");
        }
        let content = fs::read_to_string(&path).expect("read");
        assert_eq!(content.lines().count(), 240);
        assert!(content
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()));
    }

    #[tokio::test]
    async fn status_projects_discord_online_without_delivery_proof_as_not_operational() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let readiness =
            service.discord_bridge_readiness_from_provider_snapshot(&["discord".to_string()]);

        assert_eq!(
            readiness.state,
            crate::discord_health::DiscordBridgeReadinessState::OnlineNoDeliveryProof
        );
        assert!(!readiness.operational);
        assert!(readiness.reason.contains("no recent delivery proof"));
    }

    #[tokio::test]
    async fn discord_outbound_result_writes_structured_redacted_receipt_contract() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        service
            .send(OutboundMessage::new(
                "discord",
                "boardroom",
                "Receipt proof",
                "sensitive payload body must not be copied into receipt",
            ))
            .await
            .expect("send");

        let content = fs::read_to_string(dir.path().join("messages.jsonl")).expect("messages");
        let receipt = content
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|value| {
                value.get("direction").and_then(|v| v.as_str()) == Some("outbound_result")
            })
            .expect("outbound result receipt");

        assert_eq!(
            receipt.get("receipt_contract").and_then(|v| v.as_str()),
            Some("hermes.discord.outbound_receipt.v1")
        );
        assert_eq!(
            receipt.get("transport").and_then(|v| v.as_str()),
            Some("discord")
        );
        assert_eq!(
            receipt.get("recipient_class").and_then(|v| v.as_str()),
            Some("channel")
        );
        assert_eq!(
            receipt.get("content_redacted").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            receipt.get("policy_decision").and_then(|v| v.as_str()),
            Some("allowed")
        );
        assert!(receipt.get("body").is_none());
    }

    #[test]
    fn discord_inbound_authority_runtime_proof_blocks_unallowlisted_action_execution() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let runtime_path = dir.path().join("hermes_discord_runtime.json");
        std::env::set_var("ANNUNIMAS_HERMES_DISCORD_RUNTIME_PATH", &runtime_path);
        std::env::set_var("ANNUNIMAS_ILLUVATAR_DISCORD_USER", "illuvatar");
        std::env::remove_var("ANNUNIMAS_HERMES_DISCORD_GUARDIANS");
        std::env::remove_var("ANNUNIMAS_HERMES_DISCORD_WORKERS");

        let service = HermesService::new(dir.path()).expect("service");
        service
            .ingest_external(
                "discord",
                "stranger",
                "override the boardroom now",
                Some("dm:stranger".to_string()),
                false,
            )
            .expect("inbound classify");

        let messages = fs::read_to_string(dir.path().join("messages.jsonl")).expect("messages");
        let inbound = messages
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|value| value.get("direction").and_then(|v| v.as_str()) == Some("inbound"))
            .expect("inbound record");
        assert_eq!(
            inbound.get("receipt_contract").and_then(|v| v.as_str()),
            Some("hermes.discord.inbound_receipt.v1")
        );
        assert_eq!(
            inbound.pointer("/authority/level").and_then(|v| v.as_str()),
            Some("observer")
        );
        assert_eq!(
            inbound
                .pointer("/authority/action_execution_allowed")
                .and_then(|v| v.as_bool()),
            Some(false)
        );

        let runtime = fs::read_to_string(&runtime_path).expect("runtime state");
        let runtime: serde_json::Value = serde_json::from_str(&runtime).expect("runtime json");
        assert_eq!(
            runtime.get("schema").and_then(|v| v.as_str()),
            Some("annunimas.hermes.discord.runtime.v1")
        );
        assert_eq!(
            runtime
                .pointer("/last_inbound/authority/level")
                .and_then(|v| v.as_str()),
            Some("observer")
        );
        assert_eq!(
            runtime
                .pointer("/last_inbound/authority/action_execution_allowed")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn discord_bridge_simulated_e2e_proves_policy_dispatch_receipt_and_readiness() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let config_dir = dir.path().join("config");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::write(
            config_dir.join("federated_comms.toml"),
            "[discord]\npolicy_guard_required = true\n",
        )
        .expect("policy config");
        std::env::remove_var("DISCORD_BOT_TOKEN");
        std::env::remove_var("DISCORD_CHANNEL_ID");

        let mut service = HermesService::new(dir.path()).expect("service");
        let fake = Arc::new(FakeDiscordChannel::default());
        service.providers = Arc::new(ProviderRuntime::from_test_channels(
            vec![ProviderConfig {
                id: "discord".to_string(),
                kind: ProviderType::Discord,
                enabled: true,
                persistent: true,
                fallback_to_direct_api: false,
            }],
            vec![("discord".to_string(), fake.clone())],
        ));

        let result = service
            .send(OutboundMessage::new(
                "discord",
                "boardroom",
                "Simulated bridge proof",
                "Hermes Discord bridge simulation succeeded. Receipt: simulated-discord-e2e. Cause: CI bridge proof. Next action: keep bridge send-only healthy until inbound proof exists.",
            ))
            .await
            .expect("simulated send");

        assert_eq!(result["queued"].as_bool(), Some(true));
        assert_eq!(result["dispatched"].as_bool(), Some(true));
        assert_eq!(result["attempts"].as_u64(), Some(1));
        assert_eq!(result["chunks_sent"].as_u64(), Some(1));

        let sent = fake
            .sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "boardroom");
        assert!(sent[0]
            .1
            .contains("Hermes Discord bridge simulation succeeded"));
        assert!(!sent[0].1.contains("DISCORD_BOT_TOKEN"));
        drop(sent);

        let content = fs::read_to_string(dir.path().join("messages.jsonl")).expect("messages");
        let receipts = content
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|value| {
                value.get("direction").and_then(|v| v.as_str()) == Some("outbound_result")
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 1);
        let receipt = &receipts[0];
        assert_eq!(
            receipt.get("receipt_contract").and_then(|v| v.as_str()),
            Some("hermes.discord.outbound_receipt.v1")
        );
        assert_eq!(
            receipt.get("transport").and_then(|v| v.as_str()),
            Some("discord")
        );
        assert_eq!(
            receipt.get("dispatched").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            receipt.get("policy_decision").and_then(|v| v.as_str()),
            Some("allowed")
        );
        assert_eq!(
            receipt.get("content_redacted").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(receipt.get("body").is_none());

        let status = service.status().await.expect("status");
        assert_eq!(
            status.discord_bridge.state,
            DiscordBridgeReadinessState::SendOnlyHealthy
        );
        assert!(status.discord_bridge.operational);
    }

    #[tokio::test]
    async fn status_reports_malformed_record_counts() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        fs::write(
            dir.path().join("messages.jsonl"),
            "{\"ok\":true}\n{\"broken\":\n",
        )
        .expect("messages");
        fs::write(
            dir.path().join("boardroom.jsonl"),
            "{\"ok\":true}\nnot-json\n",
        )
        .expect("boardroom");
        fs::write(
            dir.path().join("outbound_queue.jsonl"),
            "{\"ok\":true}\n{\"missing\"\n",
        )
        .expect("queue");
        fs::write(dir.path().join("interruptions.jsonl"), "{\"ok\":true}\n]\n")
            .expect("interruptions");

        let status = service.status().await.expect("status");
        assert_eq!(status.malformed_message_records, 1);
        assert_eq!(status.malformed_boardroom_records, 1);
        assert_eq!(status.malformed_queue_records, 1);
        assert_eq!(status.malformed_interrupt_records, 1);
    }

    #[tokio::test]
    async fn send_sheds_excess_burst_work() {
        let _send_gate_guard = crate::HERMES_PROVIDER_SEND_TEST_LOCK
            .lock()
            .expect("send gate test lock");
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        std::env::set_var("ANNUNIMAS_HERMES_SEND_MAX_CONCURRENCY", "1");
        let service = HermesService::new(dir.path()).expect("service");
        let acquired = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());

        let holder_acquired = Arc::clone(&acquired);
        let holder_release = Arc::clone(&release);
        let holder = tokio::spawn(async move {
            let _ = try_run_bounded_async("hermes_provider_send", 1, || async move {
                holder_acquired.notify_waiters();
                holder_release.notified().await;
            })
            .await;
        });
        acquired.notified().await;

        let out = service
            .send(OutboundMessage::new(
                "discord",
                "boardroom",
                "Burst",
                "should shed",
            ))
            .await
            .expect("send");
        assert_eq!(out["queued"].as_bool(), Some(true));
        assert_eq!(out["dispatched"].as_bool(), Some(false));
        assert_eq!(out["attempts"].as_u64(), Some(0));
        assert_eq!(
            out["error"].as_str(),
            Some("provider send concurrency gate saturated")
        );

        release.notify_waiters();
        holder.await.expect("holder");
        std::env::remove_var("ANNUNIMAS_HERMES_SEND_MAX_CONCURRENCY");
    }

    #[tokio::test]
    async fn auto_provider_resolves_to_fallback_transport_with_charon_metadata() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let missing_socket = dir.path().join("missing-charon.sock");
        std::env::set_var("ANNUNIMAS_CHARON_SOCKET", &missing_socket);
        std::env::set_var("ANNUNIMAS_HERMES_AUTO_TRANSPORT", "discord");

        let service = HermesService::new(dir.path()).expect("service");
        let routed = service
            .resolve_outbound_message(OutboundMessage::new(
                "auto",
                "boardroom",
                "Route test",
                "use charon when available",
            ))
            .await;

        assert_eq!(routed.requested_provider, "auto");
        assert_eq!(routed.resolved_transport, "discord");
        assert_eq!(routed.msg.provider, "discord");
        assert!(routed.charon_route.is_none());

        std::env::remove_var("ANNUNIMAS_CHARON_SOCKET");
        std::env::remove_var("ANNUNIMAS_HERMES_AUTO_TRANSPORT");
    }

    #[test]
    fn council_flow_writes_events() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let opened = service
            .council_open(
                "WayVR integration",
                vec!["athena".to_string(), "hades".to_string()],
            )
            .expect("open");
        let session_id = opened
            .get("session_id")
            .and_then(|v| v.as_str())
            .expect("id")
            .to_string();
        let _ = service
            .council_report(&session_id, "athena", "corpus depth is high")
            .expect("report");
        let _ = service
            .council_close(&session_id, "proceed with delegation")
            .expect("close");
        let boardroom = service.boardroom_recent(10).expect("recent");
        assert!(boardroom.len() >= 3);
    }

    #[test]
    fn local_council_summary_route_rejects_missing_task_reference_as_non_promotable() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let route = service
            .route_local_council_summary(
                "council_gate6",
                "Summarized discussion: keep Discord council notes low authority.",
                None,
                None,
            )
            .expect("summary route");

        assert_eq!(
            route.schema_version,
            "annunimas.hermes.local_council_summary_route.v1"
        );
        assert_eq!(route.semantic_channel, "council");
        assert_eq!(route.output_classification, "low_risk_summary");
        assert_eq!(route.source_task, None);
        assert!(!route.promotable);
        assert!(!route.is_authoritative);
        assert!(route
            .canonical_refs
            .contains(&"council_session:council_gate6".to_string()));
        assert!(route.fallback_metadata.fallback_used);
        assert_eq!(route.fallback_metadata.provider, "charon-prod-default");
    }

    #[test]
    fn local_council_summary_route_records_charon_hint_without_authority() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let route = service
            .route_local_council_summary(
                "council_gate6",
                "Summarized discussion: model suggests documenting fallback routing.",
                Some("task:discord-gate-6"),
                Some(crate::types::CharonRouteHint {
                    provider: Some("edge_core".to_string()),
                    model: Some("Qwen3.5-9B-Q4_K_M".to_string()),
                    route_evidence: Some(
                        "local provider healthy; low cost; low latency".to_string(),
                    ),
                    latency_ms: Some(150),
                    estimated_input_tokens: Some(42),
                    estimated_output_tokens: Some(24),
                    fallback_used: false,
                    fallback_reason: None,
                }),
            )
            .expect("summary route");

        assert_eq!(route.source_task.as_deref(), Some("task:discord-gate-6"));
        assert!(route.promotable);
        assert!(!route.is_authoritative);
        assert_eq!(route.provider_used.as_deref(), Some("edge_core"));
        assert_eq!(route.model_used.as_deref(), Some("Qwen3.5-9B-Q4_K_M"));
        assert_eq!(route.latency_ms, Some(150));
        assert_eq!(route.estimated_tokens, Some(66));
        assert!(!route.fallback_metadata.fallback_used);
        assert!(route
            .canonical_refs
            .contains(&"task:discord-gate-6".to_string()));
    }

    #[test]
    fn local_council_summary_route_uses_semantic_fallback_metadata() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        std::env::remove_var("DISCORD_CHANNEL_COUNCIL");
        let service = HermesService::new(dir.path()).expect("service");

        let route = service
            .route_local_council_summary(
                "council_gate6",
                "Summarized discussion: no local route is available, so preserve safe fallback metadata.",
                Some("discord-thread:1234"),
                Some(crate::types::CharonRouteHint {
                    provider: None,
                    model: None,
                    route_evidence: None,
                    latency_ms: None,
                    estimated_input_tokens: None,
                    estimated_output_tokens: None,
                    fallback_used: true,
                    fallback_reason: Some("charon route unavailable".to_string()),
                }),
            )
            .expect("summary route");

        assert_eq!(route.dispatch_channel, "boardroom");
        assert!(!route.promotable);
        assert_eq!(route.source_task, None);
        assert!(route.fallback_metadata.fallback_used);
        assert_eq!(
            route.fallback_metadata.reason.as_deref(),
            Some("charon route unavailable")
        );
        assert_eq!(route.provider_used.as_deref(), Some("charon-prod-default"));
        assert_eq!(route.model_used, None);
        assert!(!route.is_authoritative);
    }

    #[tokio::test]
    async fn decision_action_marks_queued_task_completed() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        fs::write(
            &queue_path,
            "{\"task_id\":\"tsk_test_1\",\"status\":\"queued\",\"title\":\"Test Task\"}\n",
        )
        .expect("queue write");
        std::env::set_var("ANNUNIMAS_TASK_QUEUE_PATH", &queue_path);

        let service = HermesService::new(dir.path()).expect("service");
        let mut msg = InboundMessage::new("discord", "illuvatar", "a");
        msg.channel = Some("discord".to_string());
        let ctx = DecisionExecutionContext {
            prompt_id: "dpr_test".to_string(),
            choice: "a".to_string(),
            selected_action: "execute queued task tsk_test_1".to_string(),
            selected_label: "Execute".to_string(),
        };

        service
            .execute_decision_action("discord", &msg, &ctx)
            .await
            .expect("execute");

        let updated = fs::read_to_string(&queue_path).expect("queue read");
        assert!(updated.contains("\"status\":\"completed\""));
        assert!(updated.contains("\"completion_source\":\"hermes_decision\""));
    }

    #[tokio::test]
    async fn decision_action_drains_queued_tasks_with_limit() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        fs::write(
            &queue_path,
            concat!(
                "{\"task_id\":\"tsk_test_1\",\"status\":\"queued\",\"title\":\"One\"}\n",
                "{\"task_id\":\"tsk_test_2\",\"status\":\"queued\",\"title\":\"Two\"}\n",
                "{\"task_id\":\"tsk_test_3\",\"status\":\"queued\",\"title\":\"Three\"}\n",
            ),
        )
        .expect("queue write");
        std::env::set_var("ANNUNIMAS_TASK_QUEUE_PATH", &queue_path);
        std::env::set_var("ANNUNIMAS_HERMES_DECISION_DRAIN_LIMIT", "2");

        let service = HermesService::new(dir.path()).expect("service");
        let mut msg = InboundMessage::new("discord", "illuvatar", "a");
        msg.channel = Some("discord".to_string());
        let ctx = DecisionExecutionContext {
            prompt_id: "dpr_test".to_string(),
            choice: "a".to_string(),
            selected_action: "drain queued tasks".to_string(),
            selected_label: "Drain".to_string(),
        };

        service
            .execute_decision_action("discord", &msg, &ctx)
            .await
            .expect("execute");

        let updated = fs::read_to_string(&queue_path).expect("queue read");
        assert_eq!(updated.matches("\"status\":\"completed\"").count(), 2);
        let third = updated
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|value| value.get("task_id").and_then(|v| v.as_str()) == Some("tsk_test_3"))
            .expect("third task");
        assert_eq!(third.get("status").and_then(|v| v.as_str()), Some("queued"));
    }

    #[test]
    fn arda_hud_projection_contract_uses_semantic_state_not_discord_identity() {
        let _guard = env_guard();
        std::env::remove_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH");
        std::env::remove_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH");
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let projection = service
            .render_arda_hud_channel_projection("council", Some("discord-thread-777"))
            .expect("arda projection");

        assert_eq!(
            projection.schema_version,
            "annunimas.hermes.arda_hud_projection.v1"
        );
        assert_eq!(projection.surface, "arda");
        assert_eq!(projection.semantic_channel, "council");
        assert_eq!(projection.panel, "boardroom");
        assert_eq!(projection.state_key, "hermes.semantic_channel.council");
        assert_eq!(projection.ui_identity, "arda:boardroom:council");
        assert_eq!(projection.risk_class, "low_risk_projection");
        assert_eq!(
            projection.trust_boundary,
            "semantic_projection_only_external_identity_noncanonical"
        );
        assert_eq!(
            projection.source_map_path,
            "core/state/arda_source_map.json"
        );
        assert_eq!(
            projection.triage_registry_path,
            "core/state/knowledge_triage_registry.jsonl"
        );
        assert_ne!(projection.ui_identity, "discord-thread-777");
        assert!(projection.subscribable);
        assert_eq!(
            projection
                .external_refs
                .get("discord_thread_id")
                .and_then(|value| value.as_str()),
            Some("discord-thread-777")
        );
        assert!(!projection
            .canonical_refs
            .contains(&"discord-thread-777".to_string()));
    }

    #[test]
    fn arda_hud_projection_contract_exposes_panel_subscriptions_for_semantic_channels() {
        let _guard = env_guard();
        std::env::remove_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH");
        std::env::remove_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH");
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let contract = service.arda_hud_projection_contract();

        assert_eq!(
            contract.schema_version,
            "annunimas.hermes.arda_hud_projection_contract.v1"
        );
        assert_eq!(contract.surface, "arda");
        assert!(contract
            .subscriptions
            .iter()
            .any(|subscription| subscription.panel == "boardroom"
                && subscription.semantic_channel == "council"));
        assert!(contract
            .subscriptions
            .iter()
            .any(|subscription| subscription.panel == "workstation"
                && subscription.semantic_channel == "tasks"));
        assert!(contract
            .subscriptions
            .iter()
            .any(|subscription| subscription.panel == "world"
                && subscription.semantic_channel == "general"));
        assert!(contract
            .subscriptions
            .iter()
            .all(|subscription| subscription
                .state_key
                .starts_with("hermes.semantic_channel.")));
        assert!(contract
            .subscriptions
            .iter()
            .all(|subscription| !subscription.ui_identity.starts_with("discord")));
    }

    #[test]
    fn arda_hud_projection_contract_persists_state_backed_lifecycle_without_discord_canonicality() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let source_map_path = dir.path().join("core/state/arda_source_map.json");
        let triage_registry_path = dir
            .path()
            .join("core/state/knowledge_triage_registry.jsonl");
        let contract_path = dir
            .path()
            .join("core/state/hermes_arda_projection_contract.json");
        std::fs::create_dir_all(source_map_path.parent().expect("source map parent"))
            .expect("state dir");
        std::fs::write(
            &source_map_path,
            serde_json::json!({
                "schema_version": "annunimas.core.state.v1",
                "sections": []
            })
            .to_string(),
        )
        .expect("source map seed");

        std::env::set_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH", &source_map_path);
        std::env::set_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH", &triage_registry_path);
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::set_var(
            "ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH",
            &contract_path,
        );

        let service = HermesService::new(dir.path().join("data/hermes")).expect("service");
        let receipt = service
            .persist_arda_hud_projection_contract()
            .expect("persist projection contract");

        assert_eq!(receipt.risk_class, "low_risk_projection");
        assert_eq!(receipt.contract_path, contract_path.to_string_lossy());
        assert_eq!(receipt.source_map_path, source_map_path.to_string_lossy());
        assert_eq!(
            receipt.triage_registry_path,
            triage_registry_path.to_string_lossy()
        );
        assert!(receipt.subscription_count >= 7);

        let contract: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&contract_path).expect("contract read"))
                .expect("contract json");
        assert_eq!(
            contract.get("risk_class").and_then(|value| value.as_str()),
            Some("low_risk_projection")
        );
        assert!(contract
            .get("subscriptions")
            .and_then(|value| value.as_array())
            .map(
                |subscriptions| subscriptions.iter().all(|subscription| subscription
                    .get("ui_identity")
                    .and_then(|value| value.as_str())
                    .map(|identity| !identity.starts_with("discord"))
                    .unwrap_or(false))
            )
            .unwrap_or(false));

        let source_map: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&source_map_path).expect("source map read"),
        )
        .expect("source map json");
        assert!(source_map
            .get("sections")
            .and_then(|value| value.as_array())
            .map(|sections| sections
                .iter()
                .any(|section| section.get("id").and_then(|value| value.as_str())
                    == Some("hermes_arda_projection")))
            .unwrap_or(false));

        let registry =
            std::fs::read_to_string(&triage_registry_path).expect("triage registry read");
        assert!(registry.contains("hermes.semantic_channel.council"));
        assert!(!registry.contains("discord-thread"));

        std::env::remove_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH");
        std::env::remove_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH");
    }

    #[test]
    fn arda_hud_projection_contract_default_write_paths_are_service_root_scoped() {
        let _guard = env_guard();
        std::env::remove_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH");
        std::env::remove_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH");

        let cwd = std::env::current_dir().expect("current dir");
        let cwd_registry_path = cwd.join("core/state/knowledge_triage_registry.jsonl");
        let cwd_registry_before = std::fs::read_to_string(&cwd_registry_path).ok();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let receipt = service
            .persist_arda_hud_projection_contract()
            .expect("persist projection contract");

        let expected_state = dir.path().join("core/state");
        assert_eq!(
            receipt.triage_registry_path,
            expected_state
                .join("knowledge_triage_registry.jsonl")
                .to_string_lossy()
        );
        assert_eq!(
            receipt.source_map_path,
            expected_state
                .join("arda_source_map.json")
                .to_string_lossy()
        );
        assert_eq!(
            receipt.contract_path,
            expected_state
                .join("hermes_arda_projection_contract.json")
                .to_string_lossy()
        );
        assert!(expected_state
            .join("knowledge_triage_registry.jsonl")
            .exists());
        assert_eq!(
            std::fs::read_to_string(&cwd_registry_path).ok(),
            cwd_registry_before
        );

        std::env::remove_var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH");
        std::env::remove_var("ANNUNIMAS_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        std::env::remove_var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH");
    }

    #[test]
    fn task_approval_thread_model_keeps_discord_thread_metadata_noncanonical() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let proposal = service
            .create_task_approval_proposal(
                "discord_operating_room",
                CommsEventRisk::High,
                "Run gated Discord task implementation",
                Some("tsk_gate_3"),
                "illuvatar",
                Some("discord-thread-123"),
            )
            .expect("proposal");

        assert_eq!(proposal.task_ref, "task:tsk_gate_3");
        assert_eq!(proposal.scope, "discord_operating_room");
        assert_eq!(proposal.risk, CommsEventRisk::High);
        assert!(proposal
            .canonical_refs
            .contains(&"task:tsk_gate_3".to_string()));
        assert!(!proposal
            .canonical_refs
            .contains(&"discord-thread-123".to_string()));
        assert_eq!(
            proposal
                .delivery_metadata
                .get("discord_thread_id")
                .and_then(|value| value.as_str()),
            Some("discord-thread-123")
        );

        let discord = service.render_task_approval_projection(&proposal, "discord");
        let terminal = service.render_task_approval_projection(&proposal, "terminal");
        let arda = service.render_task_approval_projection(&proposal, "arda");

        assert_eq!(discord.task_ref, proposal.task_ref);
        assert_eq!(terminal.task_ref, proposal.task_ref);
        assert_eq!(arda.task_ref, proposal.task_ref);
        assert!(discord.body.contains("task:tsk_gate_3"));
        assert_eq!(
            discord
                .delivery_metadata
                .get("discord_thread_id")
                .and_then(|value| value.as_str()),
            Some("discord-thread-123")
        );
        assert!(!terminal.delivery_metadata.contains_key("discord_thread_id"));
        assert!(!arda.delivery_metadata.contains_key("discord_thread_id"));
    }

    #[test]
    fn task_approval_thread_model_records_scope_risk_summary_and_receipt() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let proposal = service
            .create_task_approval_proposal(
                "discord_operating_room",
                CommsEventRisk::Medium,
                "Approve bounded terminal-safe task projection",
                None,
                "operator",
                None,
            )
            .expect("proposal");

        let packet = service
            .record_task_approval_packet(&proposal, "operator", "receipt_abc123")
            .expect("approval packet");

        assert_eq!(packet.proposal_id, proposal.proposal_id);
        assert_eq!(packet.task_ref, proposal.task_ref);
        assert_eq!(packet.scope, "discord_operating_room");
        assert_eq!(packet.risk, CommsEventRisk::Medium);
        assert_eq!(
            packet.action_summary,
            "Approve bounded terminal-safe task projection"
        );
        assert_eq!(packet.receipt_id, "receipt_abc123");
        assert_eq!(packet.approved_by, "operator");
    }

    #[test]
    fn council_discussion_lane_marks_notes_discussion_only_until_promoted() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let note = service
            .record_council_discussion_note(
                "council_gate_5",
                "oracle",
                "Consider a bounded rollout; this is not final approval.",
                CommsEventRisk::Medium,
                "agent",
            )
            .expect("discussion note");

        assert_eq!(note.session_id, "council_gate_5");
        assert_eq!(note.agent, "oracle");
        assert_eq!(note.semantic_channel, "council");
        assert!(note.discussion_only);
        assert_eq!(note.promotion_state, PromotionState::Unpromoted);
        assert!(note.summary.contains("discussion-only"));
        assert!(note.canonical_refs.contains(&note.note_id));

        let discord = service.render_council_discussion_projection(&note, "discord");
        let terminal = service.render_council_discussion_projection(&note, "terminal");
        assert_eq!(discord.surface, "discord");
        assert_eq!(discord.semantic_channel, "council");
        assert!(matches!(
            discord.dispatch_channel.as_str(),
            "council" | "boardroom"
        ));
        assert!(discord.title.contains("discussion-only"));
        assert!(discord.body.contains("Not approved"));
        assert!(discord.body.len() <= 600);
        assert_eq!(terminal.note_id, note.note_id);

        let promotion = service
            .promote_council_discussion_to_task(&note, "task:tsk_gate_5_canonical")
            .expect("promotion requires canonical task ref");
        assert_eq!(promotion.note_id, note.note_id);
        assert_eq!(promotion.task_ref, "task:tsk_gate_5_canonical");
        assert_eq!(promotion.promotion_state, PromotionState::Projected);
        assert!(!promotion.is_authoritative);
        assert!(!promotion.canonical_write_authorized);
        assert!(!promotion.queue_mutated);
        assert!(promotion.requires_human_approval);
        assert_eq!(
            promotion.authority_boundary,
            "discord_projection_only_not_canonical_authority"
        );
    }

    #[test]
    fn council_command_seats_define_arandur_second_and_third_command() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let seats = service.council_command_seats();

        assert_eq!(seats.len(), 3);
        assert_eq!(seats[0].seat, "first");
        assert_eq!(seats[0].agent_id, "arandur");
        assert_eq!(seats[0].authority, "sovereign_direction");
        assert_eq!(seats[1].seat, "second");
        assert_eq!(seats[1].agent_id, "prometheus");
        assert_eq!(seats[1].authority, "execution_coordination");
        assert_eq!(seats[2].seat, "third");
        assert_eq!(seats[2].agent_id, "counsel_or_oracle");
        assert!(seats[2].role.contains("Counsel"));
        assert!(seats[2].role.contains("Oracle"));
    }

    #[test]
    fn council_discussion_lane_blocks_final_approval_and_high_risk_local_inference() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let final_claim = service.record_council_discussion_note(
            "council_gate_5",
            "warden",
            "Final approval granted; execute immediately.",
            CommsEventRisk::Medium,
            "agent",
        );
        assert!(final_claim.is_err());

        let high_risk_inference = service.record_council_discussion_note(
            "council_gate_5",
            "charon-local",
            "Local model recommends an authority decision.",
            CommsEventRisk::Medium,
            "local_inference",
        );
        assert!(high_risk_inference.is_err());

        let low_risk_inference = service
            .record_council_discussion_note(
                "council_gate_5",
                "charon-local",
                "Local model summary only: open questions remain.",
                CommsEventRisk::Low,
                "local_inference",
            )
            .expect("low-risk local summary");
        assert!(low_risk_inference.discussion_only);
        assert_eq!(low_risk_inference.risk, CommsEventRisk::Low);

        let rejected_promotion =
            service.promote_council_discussion_to_task(&low_risk_inference, "discord-thread-123");
        assert!(rejected_promotion.is_err());
    }

    #[test]
    fn council_approval_surface_records_promotion_and_note_decisions() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let low = service
            .record_council_discussion_note(
                "council_approval_1",
                "warden",
                "Requesting promotion for review.",
                CommsEventRisk::Low,
                "agent",
            )
            .expect("low risk note");
        let promotion = service
            .promote_council_discussion_to_task(&low, "task:discussion-200")
            .expect("promotion");
        let promoted_task = promotion.task_ref;

        let decision = service
            .approve_council_promotion("council_approval_1", &promoted_task, "operator-1")
            .expect("approve promotion");
        assert!(decision.approved);
        assert!(decision
            .canonical_refs
            .contains(&format!("council_session:council_approval_1")));
        assert!(decision
            .canonical_refs
            .iter()
            .any(|ref_| ref_.starts_with("council_approval_")));
        assert_eq!(decision.status, "approved");
        assert_eq!(decision.reason, "operator approval granted");

        let note_decision = service
            .approve_council_note("council_approval_1", &low.note_id, "operator-1")
            .expect("approve note");
        assert_eq!(note_decision.note_id, Some(low.note_id));
        assert_eq!(note_decision.session_id, "council_approval_1");
        assert_eq!(note_decision.status, "approved");
    }

    #[test]
    fn subagent_completion_lane_marks_unverified_claims_for_review() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let packet = service
            .record_subagent_completion_packet(
                "tsk_gate_4",
                "forge-mind",
                "Implemented the Discord completion lane",
                vec!["cargo test -p annunimas-hermes subagent_completion_lane".to_string()],
                vec!["crates/annunimas-hermes/src/service/subagent_completion.rs".to_string()],
                Vec::new(),
                CommsEventRisk::Medium,
                "review focused diff and promote",
                false,
            )
            .expect("completion packet");

        assert_eq!(packet.task_ref, "task:tsk_gate_4");
        assert_eq!(packet.agent, "forge-mind");
        assert_eq!(packet.status, "needs_review");
        assert!(packet.review_required);
        assert_eq!(packet.risk, CommsEventRisk::Medium);
        assert!(packet
            .canonical_refs
            .contains(&"task:tsk_gate_4".to_string()));
        assert!(packet.canonical_refs.contains(&packet.completion_id));
        assert_eq!(packet.changed_paths.len(), 1);
        assert_eq!(packet.blockers.len(), 0);

        let discord = service.render_subagent_completion_projection(&packet, "discord");
        let arda = service.render_subagent_completion_projection(&packet, "arda");
        let terminal = service.render_subagent_completion_projection(&packet, "terminal");

        assert_eq!(discord.surface, "discord");
        assert_eq!(discord.semantic_channel, "subagents");
        assert_eq!(discord.dispatch_channel, "tasks");
        assert!(discord.title.contains("needs_review"));
        assert!(discord.body.contains("forge-mind"));
        assert!(discord.body.contains("Changed paths: 1"));
        assert!(discord.body.contains("Verification: 1 item"));
        assert!(discord
            .body
            .contains("Next: review focused diff and promote"));
        assert!(discord.body.len() <= 600);
        assert_eq!(arda.task_ref, packet.task_ref);
        assert_eq!(terminal.task_ref, packet.task_ref);
    }

    #[test]
    fn subagent_completion_lane_verified_without_blockers_is_completed() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let packet = service
            .record_subagent_completion_packet(
                "task:tsk_gate_4_verified",
                "athena",
                "Validated documentation reconciliation",
                vec!["cargo test -p annunimas-hermes --lib".to_string()],
                Vec::new(),
                Vec::new(),
                CommsEventRisk::Low,
                "close gate",
                true,
            )
            .expect("completion packet");

        assert_eq!(packet.task_ref, "task:tsk_gate_4_verified");
        assert_eq!(packet.status, "completed");
        assert!(!packet.review_required);

        let events = service.recent_comms_events(5);
        let event = events
            .iter()
            .find(|entry| {
                entry
                    .get("canonical_refs")
                    .and_then(|value| value.as_array())
                    .map(|refs| {
                        refs.iter()
                            .any(|value| value.as_str() == Some(&packet.completion_id))
                    })
                    .unwrap_or(false)
            })
            .expect("completion comms event");
        assert_eq!(
            event.get("semantic_channel").and_then(|v| v.as_str()),
            Some("subagents")
        );
        assert_eq!(
            event.get("raw_content_redacted").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn interrupt_capture_writes_audit_and_context() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        let service = HermesService::new(dir.path()).expect("service");
        let warden_queue = dir.path().join("warden").join("informant_queue.jsonl");
        let apollo_hook = dir.path().join("apollo").join("interruptions.jsonl");
        let orders_path = dir.path().join("prometheus").join("orders.jsonl");
        fs::create_dir_all(orders_path.parent().expect("parent")).expect("mkdir");
        fs::write(
            &orders_path,
            "{\"task_id\":\"task_active_1\",\"status\":\"open\"}\n{\"task_id\":\"task_done_1\",\"status\":\"complete\"}\n",
        )
        .expect("orders write");
        std::env::set_var("ANNUNIMAS_WARDEN_QUEUE_PATH", &warden_queue);
        std::env::set_var("ANNUNIMAS_APOLLO_INTERRUPT_QUEUE_PATH", &apollo_hook);
        std::env::set_var("ANNUNIMAS_PROMETHEUS_ORDERS_PATH", &orders_path);
        std::env::set_var("ANNUNIMAS_PLUTUS_HOME", &plutus_home);

        let mut msg = InterruptionMessage::new("voice", "operator", "switch to queue cleanup");
        msg.channel = Some("discord".to_string());
        let out = service.interrupt(msg).expect("interrupt");
        assert!(out
            .get("captured")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        let disposition = out
            .get("disposition")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert_eq!(disposition, "reroute");
        let ctx_task_ids = out
            .get("context")
            .and_then(|v| v.get("task_ids"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!ctx_task_ids.is_empty());

        let interrupts =
            fs::read_to_string(dir.path().join("interruptions.jsonl")).expect("interruptions read");
        assert!(interrupts.contains("switch to queue cleanup"));
        let warden = fs::read_to_string(&warden_queue).expect("warden queue read");
        assert!(warden.contains("\"event_type\":\"interrupt_captured\""));
        let apollo = fs::read_to_string(&apollo_hook).expect("apollo hook read");
        assert!(apollo.contains("\"source\":\"hermes_interrupt\""));
        std::thread::sleep(Duration::from_millis(150));
        let mut plutus_status = serde_json::json!({});
        for _ in 0..20 {
            plutus_status = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(async {
                    PlutusService::from_home(&plutus_home)
                        .expect("plutus service")
                        .status()
                        .await
                        .expect("plutus status")
                });
            if plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );
        std::env::remove_var("ANNUNIMAS_PLUTUS_HOME");
    }

    #[test]
    fn interrupt_reroute_respects_backpressure_and_defers() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        std::env::set_var("ANNUNIMAS_HERMES_REROUTE_MAX_PER_SEC", "1");

        let msg1 = InterruptionMessage::new("voice", "operator", "switch to queue cleanup");
        let out1 = service.interrupt(msg1).expect("interrupt one");
        assert_eq!(
            out1.get("disposition").and_then(|v| v.as_str()),
            Some("reroute")
        );

        let msg2 = InterruptionMessage::new("voice", "operator", "reroute to maintenance");
        let out2 = service.interrupt(msg2).expect("interrupt two");
        let deferred = out2
            .get("reroute_result")
            .and_then(|v| v.get("deferred"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(deferred);

        let deferred_file =
            fs::read_to_string(dir.path().join("reroute_deferred.jsonl")).expect("deferred read");
        assert!(deferred_file.contains("\"reason\":\"reroute_rate_limited\""));
        let metrics_file =
            fs::read_to_string(dir.path().join("reroute_metrics.jsonl")).expect("metrics read");
        assert!(metrics_file.contains("\"event\":\"deferred\""));
    }

    #[test]
    fn reroute_failures_enter_dlq_and_can_be_retried() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        std::env::set_var("ANNUNIMAS_HERMES_REROUTE_MAX_PER_SEC", "5");

        let msg = InterruptionMessage::new("voice", "operator", "reroute to maintenance queue");
        let out = service.interrupt(msg).expect("interrupt");
        let dlq_path = out
            .get("reroute_result")
            .and_then(|v| v.get("dlq_path"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(!dlq_path.is_empty());

        let dlq_before =
            fs::read_to_string(dir.path().join("reroute_dlq.jsonl")).expect("dlq read");
        assert!(dlq_before.contains("\"status\":\"pending\""));

        let retry = service.retry_reroute_dlq(10).expect("retry");
        assert!(retry.get("processed").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
        assert!(retry.get("failed").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
        let dlq_after = fs::read_to_string(dir.path().join("reroute_dlq.jsonl")).expect("dlq read");
        assert!(dlq_after.contains("\"attempt\":2"));
    }

    #[test]
    fn interrupt_policy_gate_blocks_unauthorized_override() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let policy_path = dir.path().join("interrupt_policy.json");
        let escalations_path = dir.path().join("escalations.jsonl");
        fs::write(
            &policy_path,
            r#"{"default":{"allow":["note"]},"senders":{"operator":{"allow":["note"]}}}"#,
        )
        .expect("policy write");
        std::env::set_var("ANNUNIMAS_INTERRUPT_AUTH_POLICY_PATH", &policy_path);
        std::env::set_var("ANNUNIMAS_PROMETHEUS_ESCALATIONS_PATH", &escalations_path);

        let msg = InterruptionMessage::new("voice", "operator", "override and stop now");
        let out = service.interrupt(msg).expect("interrupt");
        assert_eq!(
            out.get("disposition").and_then(|v| v.as_str()),
            Some("override")
        );
        assert_eq!(
            out.get("policy_authorized").and_then(|v| v.as_bool()),
            Some(false)
        );
        let blocked = out
            .get("reroute_result")
            .and_then(|v| v.get("blocked"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(blocked);

        let escalations = fs::read_to_string(escalations_path).expect("escalations read");
        assert!(escalations.contains("interrupt_authority_policy.denied"));
    }

    #[tokio::test]
    async fn operating_room_event_records_canonical_state_without_discord_projection() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let event = service
            .record_operating_room_event(
                OperatingRoomEventKind::Status,
                "hermes_discord_operating_room",
                "Hermes Discord operating room status projection",
                "bridge online; delivery receipts pending",
                vec!["core/state/hermes_discord_runtime.json".to_string()],
                false,
            )
            .expect("event");

        assert_eq!(
            event.schema_version,
            "annunimas.hermes.operating_room_event.v1"
        );
        assert_eq!(event.kind, OperatingRoomEventKind::Status);
        assert_eq!(event.topic, "hermes_discord_operating_room");
        assert!(!event.discord_projection_permitted);
        assert_eq!(event.safety_state, "observe_only");
        assert!(event.event_id.starts_with("ore_"));

        let events = fs::read_to_string(dir.path().join("operating_room_events.jsonl"))
            .expect("events read");
        assert!(events.contains("annunimas.hermes.operating_room_event.v1"));
        assert!(events.contains("core/state/hermes_discord_runtime.json"));
        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.trim().is_empty());
    }

    #[test]
    fn operating_room_renderer_is_redacted_plain_language_and_discord_sized() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let event = service
            .record_operating_room_event(
                OperatingRoomEventKind::Alert,
                "provider_health",
                "Provider heartbeat degraded",
                "Discord token=TOKEN_SAMPLE_SHOULD_NOT_LEAK should never leak into projection",
                vec!["data/hermes/provider_heartbeat.jsonl".to_string()],
                false,
            )
            .expect("event");

        let rendered = service.render_operating_room_event_for_discord(&event);
        assert!(rendered.len() <= 1900);
        assert!(rendered.contains("HERMES operating room event"));
        assert!(rendered.contains("trace:"));
        assert!(rendered.contains("kind: alert"));
        assert!(rendered.contains("safety: observe_only"));
        assert!(rendered.contains("next action:"));
        assert!(rendered.contains("receipt:"));
        assert!(!rendered.contains("TOKEN_SAMPLE_SHOULD_NOT_LEAK"));
    }

    #[test]
    fn comms_event_contract_records_redacted_semantic_event_and_skips_malformed_history() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        fs::write(
            dir.path().join("comms_events.jsonl"),
            "{malformed historical record}\n",
        )
        .expect("malformed seed");

        let event = service
            .record_comms_event(
                CommsEventType::Inbound,
                "governance-audit",
                CommsEventVisibility::OperatorOnly,
                CommsEventRisk::High,
                "Discord-origin operator note with token=ghp_secret and password=hunter2",
                vec!["discord:message:123".to_string(), "receipt:abc".to_string()],
                PromotionState::Unpromoted,
                true,
            )
            .expect("comms event");

        assert_eq!(event.schema_version, "annunimas.hermes.comms_event.v1");
        assert!(event.event_id.starts_with("comms_"));
        assert_eq!(event.semantic_channel, "governance-audit");
        assert_eq!(event.event_type, CommsEventType::Inbound);
        assert_eq!(event.visibility, CommsEventVisibility::OperatorOnly);
        assert_eq!(event.risk, CommsEventRisk::High);
        assert_eq!(event.promotion_state, PromotionState::Unpromoted);
        assert_eq!(event.canonical_refs.len(), 2);
        assert!(event.raw_content_redacted);
        assert!(!event.summary.contains("ghp_secret"));
        assert!(!event.summary.contains("hunter2"));
        assert!(event.summary.contains("[REDACTED]"));

        let recent = service.recent_comms_events(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent[0]
                .get("schema_version")
                .and_then(|value| value.as_str()),
            Some("annunimas.hermes.comms_event.v1")
        );
    }

    #[test]
    fn comms_event_projection_from_operating_room_keeps_discord_surface_independent() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let operating_room = service
            .record_operating_room_event(
                OperatingRoomEventKind::Status,
                "bridge_receipt_status",
                "Receipt-backed Discord bridge status",
                "Discord token=ghp_abc123 should not leak beyond canonical summary",
                vec!["data/hermes/outbound_receipts.jsonl".to_string()],
                true,
            )
            .expect("operating room event");

        let comms = service
            .record_operating_room_comms_event(&operating_room, "tasks")
            .expect("comms event");

        assert_eq!(comms.event_type, CommsEventType::Status);
        assert_eq!(comms.semantic_channel, "tasks");
        assert_eq!(comms.visibility, CommsEventVisibility::OperatorVisible);
        assert_eq!(comms.risk, CommsEventRisk::Low);
        assert_eq!(comms.promotion_state, PromotionState::Projected);
        assert!(comms
            .canonical_refs
            .iter()
            .any(|reference| reference == &operating_room.event_id));
        assert!(!comms.summary.contains("ghp_abc123"));
        assert!(comms.summary.contains("[REDACTED]"));

        let events =
            fs::read_to_string(dir.path().join("comms_events.jsonl")).expect("comms events read");
        assert!(events.contains("annunimas.hermes.comms_event.v1"));
        assert!(!events.contains("ghp_abc123"));
    }

    #[test]
    fn semantic_channel_registry_defines_gate_one_taxonomy() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let registry = service.semantic_channel_registry();
        let names = registry
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "general",
                "work-stream",
                "tasks",
                "subagents",
                "council",
                "research-forge",
                "governance-audit"
            ]
        );
        assert!(registry.iter().any(|entry| entry.name == "governance-audit"
            && entry.env_key == "DISCORD_CHANNEL_GOVERNANCE_AUDIT"
            && entry.fallback == "boardroom"));
    }

    #[test]
    fn semantic_channel_resolution_uses_configured_alias_without_leaking_id() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        std::env::set_var("DISCORD_CHANNEL_GOVERNANCE_AUDIT", "123456789012345678");

        let resolution = service
            .resolve_semantic_discord_channel("governance_audit")
            .expect("resolution");

        assert_eq!(resolution.semantic_channel, "governance-audit");
        assert_eq!(resolution.env_key, "DISCORD_CHANNEL_GOVERNANCE_AUDIT");
        assert_eq!(resolution.discord_recipient, "governance-audit");
        assert!(resolution.configured);
        assert!(!resolution.fallback_used);

        std::env::remove_var("DISCORD_CHANNEL_GOVERNANCE_AUDIT");
    }

    #[test]
    fn semantic_channel_resolution_falls_back_and_normalizes_legacy_boardroom_aliases() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        std::env::remove_var("DISCORD_CHANNEL_TASKS");
        std::env::remove_var("DISCORD_CHANNEL_WORK_STREAM");
        std::env::remove_var("DISCORD_CHANNEL_SUBAGENTS");

        let work_stream_resolution = service
            .resolve_semantic_discord_channel("workstream")
            .expect("work stream resolution");
        assert_eq!(work_stream_resolution.semantic_channel, "work-stream");
        assert_eq!(
            work_stream_resolution.env_key,
            "DISCORD_CHANNEL_WORK_STREAM"
        );
        assert_eq!(work_stream_resolution.discord_recipient, "tasks");
        assert!(!work_stream_resolution.configured);
        assert!(work_stream_resolution.fallback_used);

        let task_resolution = service
            .resolve_semantic_discord_channel("ops-boardroom")
            .expect("task resolution");
        assert_eq!(task_resolution.semantic_channel, "tasks");
        assert_eq!(task_resolution.env_key, "DISCORD_CHANNEL_TASKS");
        assert_eq!(task_resolution.discord_recipient, "boardroom");
        assert!(!task_resolution.configured);
        assert!(task_resolution.fallback_used);

        let subagent_resolution = service
            .resolve_semantic_discord_channel("subagents")
            .expect("subagent resolution");
        assert_eq!(subagent_resolution.semantic_channel, "subagents");
        assert_eq!(subagent_resolution.env_key, "DISCORD_CHANNEL_SUBAGENTS");
        assert_eq!(subagent_resolution.discord_recipient, "tasks");
        assert!(!subagent_resolution.configured);
        assert!(subagent_resolution.fallback_used);
    }

    #[tokio::test]
    async fn operating_room_dispatch_blocks_unpermitted_or_unsafe_actions() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let event = service
            .record_operating_room_event(
                OperatingRoomEventKind::Command,
                "unsafe_restart_request",
                "Restart Charon from Discord",
                "restart annunimas-charon.service now",
                vec!["core/projects/tasks/queue.jsonl".to_string()],
                false,
            )
            .expect("event");

        let blocked = service
            .dispatch_operating_room_event_to_discord(&event.event_id, "ops-boardroom")
            .await
            .expect("dispatch check");
        assert_eq!(blocked.get("queued").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            blocked.get("blocked_reason").and_then(|v| v.as_str()),
            Some("discord_projection_not_permitted")
        );

        let dispatches = fs::read_to_string(dir.path().join("operating_room_dispatches.jsonl"))
            .expect("dispatches read");
        assert!(dispatches.contains("discord_projection_not_permitted"));
        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.trim().is_empty());
    }

    #[tokio::test]
    async fn operating_room_dispatch_queues_only_observe_events_through_outbound() {
        let _guard = env_guard();
        std::env::remove_var("DISCORD_CHANNEL_TASKS");
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let event = service
            .record_operating_room_event(
                OperatingRoomEventKind::Status,
                "bridge_receipt_status",
                "Receipt-backed Discord bridge status",
                "Hermes bridge has delivery proof from outbound_receipts.jsonl",
                vec!["data/hermes/outbound_receipts.jsonl".to_string()],
                true,
            )
            .expect("event");

        let out = service
            .dispatch_operating_room_event_to_discord(&event.event_id, "ops-boardroom")
            .await
            .expect("dispatch");
        assert_eq!(out.get("queued").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            out.get("dispatch_provider").and_then(|v| v.as_str()),
            Some("discord")
        );

        assert_eq!(
            out.get("semantic_channel").and_then(|v| v.as_str()),
            Some("tasks")
        );
        assert_eq!(
            out.get("dispatch_channel").and_then(|v| v.as_str()),
            Some("boardroom")
        );
        assert_eq!(
            out.get("semantic_channel_fallback_used")
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.contains("boardroom"));
        assert!(!outbound.contains("ops-boardroom"));
        assert!(outbound.contains("HERMES operating room event"));
        assert!(outbound.contains(&event.event_id));
    }

    #[test]
    fn boardroom_quorum_packet_defaults_to_review_required_without_dispatch() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let packet = service
            .project_boardroom_quorum_packet(
                "council_gate_36",
                "Gate 3.6 boardroom quorum",
                vec!["data/hermes/council_sessions.jsonl".to_string()],
                Some("oracle_missing".to_string()),
                Some(dir.path().join("oracle_verdicts.jsonl")),
                None,
                2,
                vec!["aurelius".to_string()],
            )
            .expect("packet");

        assert_eq!(
            packet.schema_version,
            "annunimas.hermes.boardroom_quorum.v1"
        );
        assert_eq!(packet.session_id, "council_gate_36");
        assert_eq!(packet.status, "review_required");
        assert!(packet.status_reason.contains("oracle_verdict_missing"));
        assert!(packet.status_reason.contains("quorum_threshold_unmet:1/2"));
        assert!(packet
            .status_reason
            .contains("charon_route_evidence_missing"));
        assert!(!packet.discord_projection_permitted);
        assert!(packet.operator_approval_required);
        assert!(!packet.operator_approved);
        assert_eq!(packet.oracle.query_id.as_deref(), Some("oracle_missing"));
        assert!(!packet.oracle.verdict_found);
        assert_eq!(packet.quorum.result, "review_required");

        let packets = fs::read_to_string(dir.path().join("boardroom_quorum_packets.jsonl"))
            .expect("packets read");
        assert!(packets.contains("\"schema_version\":\"annunimas.hermes.boardroom_quorum.v1\""));
        assert!(packets.contains("\"status\":\"review_required\""));
        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.trim().is_empty());
    }

    #[test]
    fn boardroom_quorum_packet_links_oracle_verdict_and_thresholds() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let verdicts = dir.path().join("oracle_verdicts.jsonl");
        fs::write(
            &verdicts,
            r#"{"query_id":"oracle_gate_36","outcome":"Pass","resonance_score":0.87,"gates":{"aurelius":{"score":0.91},"bacon":{"score":0.88},"sun_tzu":{"score":0.79}}}
"#,
        )
        .expect("verdict write");

        let packet = service
            .project_boardroom_quorum_packet(
                "council_gate_36",
                "Gate 3.6 boardroom quorum",
                vec![
                    "data/hermes/council_sessions.jsonl".to_string(),
                    verdicts.display().to_string(),
                ],
                Some("oracle_gate_36".to_string()),
                Some(verdicts.clone()),
                Some("edge_hub_3080:nous-hermes".to_string()),
                2,
                vec!["aurelius".to_string(), "bacon".to_string()],
            )
            .expect("packet");

        assert_eq!(packet.status, "passed");
        assert_eq!(
            packet.status_reason,
            "oracle_quorum_and_charon_route_verified"
        );
        assert!(packet.oracle.verdict_found);
        assert_eq!(
            packet.oracle.verdict_locator.as_deref(),
            Some(verdicts.to_str().unwrap_or_default())
        );
        assert_eq!(packet.oracle.outcome.as_deref(), Some("Pass"));
        assert_eq!(packet.oracle.triad_scores.aurelius, Some(0.91));
        assert_eq!(packet.oracle.triad_scores.bacon, Some(0.88));
        assert_eq!(packet.oracle.triad_scores.sun_tzu, Some(0.79));
        assert_eq!(packet.oracle.resonance_score, Some(0.87));
        assert_eq!(packet.quorum.threshold, 2);
        assert_eq!(packet.quorum.approvals.len(), 2);
        assert_eq!(packet.quorum.result, "passed");
        assert_eq!(
            packet.charon_route.selected_provider.as_deref(),
            Some("edge_hub_3080")
        );
        assert_eq!(
            packet.charon_route.selected_model.as_deref(),
            Some("nous-hermes")
        );
    }

    #[test]
    fn boardroom_quorum_renderer_is_pure_discord_sized_review_packet() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let packet = service
            .project_boardroom_quorum_packet(
                "council_gate_36",
                "Gate 3.6 boardroom quorum",
                vec!["data/hermes/council_sessions.jsonl".to_string()],
                Some("oracle_gate_36".to_string()),
                None,
                Some("edge_hub_3080:nous-hermes".to_string()),
                2,
                vec!["aurelius".to_string()],
            )
            .expect("packet");

        let rendered = service.render_boardroom_quorum_review_packet(&packet);
        assert!(rendered.len() <= 1900);
        assert!(rendered.contains("trace:"));
        assert!(rendered.contains("session: council_gate_36"));
        assert!(rendered.contains("quorum: review_required"));
        assert!(rendered.contains("reason: oracle_verdict_missing"));
        assert!(rendered.contains("oracle: oracle_gate_36"));
        assert!(rendered.contains("charon: edge_hub_3080 / nous-hermes"));
        assert!(rendered.contains("operator approval: required"));

        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.trim().is_empty());
    }

    #[tokio::test]
    async fn boardroom_quorum_dispatch_blocks_unapproved_or_unpassed_packets() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let packet = service
            .project_boardroom_quorum_packet(
                "council_gate_36",
                "Gate 3.6 boardroom quorum",
                vec!["data/hermes/council_sessions.jsonl".to_string()],
                Some("oracle_gate_36".to_string()),
                None,
                Some("edge_hub_3080:nous-hermes".to_string()),
                2,
                vec!["aurelius".to_string()],
            )
            .expect("packet");

        let blocked = service
            .dispatch_boardroom_quorum_packet(&packet.packet_id, "discord", "ops-boardroom", "")
            .await
            .expect("dispatch check");

        assert_eq!(blocked.get("queued").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            blocked.get("blocked_reason").and_then(|v| v.as_str()),
            Some("operator_approval_note_required")
        );
        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.trim().is_empty());

        let blocked = service
            .dispatch_boardroom_quorum_packet(
                &packet.packet_id,
                "discord",
                "ops-boardroom",
                "operator approved dispatch smoke",
            )
            .await
            .expect("dispatch check");
        assert_eq!(blocked.get("queued").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            blocked.get("blocked_reason").and_then(|v| v.as_str()),
            Some("packet_status_not_passed:review_required")
        );
    }

    #[tokio::test]
    async fn boardroom_quorum_dispatch_queues_approved_passed_packet_through_outbound() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let verdicts = dir.path().join("oracle_verdicts.jsonl");
        fs::write(
            &verdicts,
            r#"{"query_id":"oracle_gate_36","outcome":"Pass","resonance_score":0.87,"gates":{"aurelius":{"score":0.91},"bacon":{"score":0.88},"sun_tzu":{"score":0.79}}}
"#,
        )
        .expect("verdict write");
        let packet = service
            .project_boardroom_quorum_packet(
                "council_gate_36",
                "Gate 3.6 boardroom quorum",
                vec!["data/hermes/council_sessions.jsonl".to_string()],
                Some("oracle_gate_36".to_string()),
                Some(verdicts),
                Some("edge_hub_3080:nous-hermes".to_string()),
                2,
                vec!["aurelius".to_string(), "bacon".to_string()],
            )
            .expect("packet");

        let out = service
            .dispatch_boardroom_quorum_packet(
                &packet.packet_id,
                "discord",
                "ops-boardroom",
                "operator approved dispatch smoke",
            )
            .await
            .expect("dispatch");

        assert_eq!(out.get("queued").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            out.get("operator_approved").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            out.get("dispatch_provider").and_then(|v| v.as_str()),
            Some("discord")
        );
        let outbound =
            fs::read_to_string(dir.path().join("outbound_queue.jsonl")).expect("outbound read");
        assert!(outbound.contains("ops-boardroom"));
        assert!(outbound.contains("HERMES boardroom quorum review"));
        assert!(outbound.contains(&packet.packet_id));
        let dispatches = fs::read_to_string(dir.path().join("boardroom_quorum_dispatches.jsonl"))
            .expect("dispatch ledger read");
        assert!(dispatches.contains("operator approved dispatch smoke"));
        assert!(dispatches.contains("\"queued\":true"));
    }

    fn clear_discord_channel_planner_env() {
        for key in [
            "DISCORD_GUILD_ID",
            "DISCORD_CATEGORY_ID",
            "DISCORD_MANAGE_CHANNELS",
            "DISCORD_BOT_PERMISSIONS",
            "DISCORD_CHANNEL_GENERAL",
            "DISCORD_CHANNEL_WORK_STREAM",
            "DISCORD_CHANNEL_TASKS",
            "DISCORD_CHANNEL_SUBAGENTS",
            "DISCORD_CHANNEL_COUNCIL",
            "DISCORD_CHANNEL_RESEARCH_FORGE",
            "DISCORD_CHANNEL_GOVERNANCE_AUDIT",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn discord_channel_plan_projects_required_channels_without_leaking_ids() {
        let _guard = env_guard();
        clear_discord_channel_planner_env();
        std::env::set_var("DISCORD_GUILD_ID", "123456789012345678");
        std::env::set_var("DISCORD_CATEGORY_ID", "987654321098765432");
        std::env::set_var("DISCORD_CHANNEL_TASKS", "222222222222222222");
        std::env::set_var("DISCORD_MANAGE_CHANNELS", "true");
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let plan = service.discord_channel_plan();

        assert_eq!(
            plan.schema_version,
            "annunimas.hermes.discord_channel_plan.v1"
        );
        assert_eq!(plan.mode, "read_only_discovery");
        assert_eq!(plan.guild_id.as_deref(), Some("[REDACTED]"));
        assert_eq!(plan.category_id.as_deref(), Some("[REDACTED]"));
        assert!(plan.secrets_redacted);
        assert_eq!(plan.required_channels.len(), 7);
        assert_eq!(plan.existing_channel_count, 1);
        assert_eq!(plan.missing_channel_count, 6);
        assert!(plan.permission_summary.can_create_channels);
        let names = plan
            .required_channels
            .iter()
            .map(|entry| entry.required_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "annunimas-general",
                "annunimas-work-stream",
                "annunimas-tasks",
                "annunimas-subagents",
                "annunimas-council",
                "annunimas-research-forge",
                "annunimas-governance-audit",
            ]
        );
        let serialized = serde_json::to_string(&plan).expect("serialize plan");
        assert!(!serialized.contains("123456789012345678"));
        assert!(!serialized.contains("987654321098765432"));
        assert!(!serialized.contains("222222222222222222"));
        clear_discord_channel_planner_env();
    }

    #[test]
    fn discord_channel_dry_run_never_mutates_and_blocks_without_operator_approval() {
        let _guard = env_guard();
        clear_discord_channel_planner_env();
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let receipt = service.apply_discord_channel_plan_dry_run(false);

        assert!(receipt.dry_run);
        assert!(!receipt.approved);
        assert!(!receipt.mutation_performed);
        assert_eq!(
            receipt.blocked_reason.as_deref(),
            Some("operator_approval_required")
        );
        assert_eq!(receipt.would_create.len(), 7);
        assert!(!dir.path().join("discord_channels.jsonl").exists());
        clear_discord_channel_planner_env();
    }
}
