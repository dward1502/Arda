use arda_prometheus::autopilot::{
    AgentCapabilities, AgentRegistry, AutopilotConfig, CeoAutopilot, LearningStore,
};

fn registry() -> AgentRegistry {
    let mut registry = AgentRegistry::new();
    registry.register(AgentCapabilities {
        agent_id: "ceo".into(),
        task_types: vec!["ops".into()],
        max_concurrent: 4,
        current_load: 0,
        success_rate: 1.0,
    });
    registry.register(AgentCapabilities {
        agent_id: "warden".into(),
        task_types: vec!["monitor".into()],
        max_concurrent: 4,
        current_load: 0,
        success_rate: 1.0,
    });
    registry.register(AgentCapabilities {
        agent_id: "prometheus".into(),
        task_types: vec!["analysis".into()],
        max_concurrent: 4,
        current_load: 0,
        success_rate: 1.0,
    });
    registry
}

fn write_allow_readiness_artifacts(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("config")).unwrap();
    std::fs::write(
        root.join("config/autonomy_operating_loop.toml"),
        r#"
[[sovereign_crates]]
id = "governance"
crate = "arda-governance"
status = "contract_required"

[[sovereign_crates]]
id = "oracle"
crate = "arda-oracle"
status = "active_prototype"

[[sovereign_crates]]
id = "plutus"
crate = "arda-plutus"
status = "contract_required"

[[sovereign_crates]]
id = "human"
crate = "arda-human"
status = "contract_required"

[[sovereign_crates]]
id = "council"
crate = "arda-aule"
status = "active_subordinate"

[[sovereign_crates]]
id = "ceo"
role = "productized CEO/orchestration API surface; current runtime authority remains Prometheus ceo-autopilot"
status = "prometheus_cea_autopilot_only"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data/prometheus")).unwrap();
    std::fs::write(
        root.join("data/prometheus/autonomy_operating_loop_preflight.json"),
        serde_json::json!({
            "schema_version": "arda.autonomy_operating_loop_preflight.v1",
            "loop": {"missing_required_stages": []},
            "summary": {
                "lane_count": 12,
                "lane_configured_count": 12,
                "lane_incomplete_count": 0
            }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data/hades")).unwrap();
    std::fs::write(
        root.join("data/hades/autonomy_cleanup_approval_packets.json"),
        serde_json::json!({
            "schema_version": "arda.hades.cleanup_approval_packets.v1",
            "candidate_count": 0,
            "cleanup_authorized": false,
            "requires_operator_approval_for_mutation": true,
            "no_file_moves_or_deletes_performed": true,
            "packets": []
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data/athena")).unwrap();
    std::fs::write(
        root.join("data/athena/external_source_lane_ledger.jsonl"),
        r#"{"schema_version":"arda.athena.external_source_lane.v1","source_id":"web","task_promotion_allowed":true,"canonical_url":"https://example.invalid/source","verification_status":"source_receipted"}
"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.join("data/council")).unwrap();
    std::fs::write(root.join("data/council/agent_conversations.jsonl"), "").unwrap();
}

#[tokio::test]
async fn objective_cycle_executes_apollo_updates_learning_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-integration-*".into();
    write_allow_readiness_artifacts(dir.path());

    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-integration","review_required":false,"approval_packet":{"approval_id":"approval-reco-integration","status":"approved","approved_by":"operator","approved_at":"2026-05-22T00:00:00Z"},"candidate":{"id":"obj_integration","owner":"prometheus","priority":"high","title":"Refactor module x"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.objectives_processed, 1);
    assert_eq!(
        report.outcomes_ingested,
        report.plans[0].apollo_dispatches.len()
    );
    assert!(!report.plans[0].queued_task_ids.is_empty());
    assert!(!report.plans[0].apollo_dispatches.is_empty());
    assert!(
        report.plans[0].pipeline_submitted,
        "approved plan should submit through Pipeline before queue delegation"
    );
    assert!(report
        .report_path
        .as_deref()
        .unwrap_or("")
        .contains("daily_"));
    assert!(report
        .weekly_report_path
        .as_deref()
        .unwrap_or("")
        .contains("weekly_"));

    let queue = std::fs::read_to_string(&cfg.queue_path).unwrap();
    assert!(queue.contains("\"origin\":\"ceo_autopilot\""));
    assert!(queue.contains("\"status\":\"pending\""));
    assert!(queue.contains("\"status\":\"completed\""));
    assert_eq!(report.autonomy_readiness.decision, "allow");
    assert!(report.autonomy_readiness.task_promotion_allowed);
    assert!(queue.contains("\"autonomy_readiness_decision\":\"allow\""));
    assert!(std::fs::read_to_string(&cfg.objectives_path)
        .unwrap()
        .trim()
        .is_empty());

    let learning = LearningStore::new(&cfg.learning_path).load();
    assert!(
        learning
            .stats
            .values()
            .any(|stats| stats.attempts > 0 && stats.successes > 0),
        "Apollo completion records should be ingested into learning"
    );

    let cursor = std::fs::read_to_string(&cfg.outcome_cursor_path).unwrap();
    assert!(cursor.contains("byte_offset"));

    let daily_path = report.report_path.as_ref().unwrap();
    let weekly_path = report.weekly_report_path.as_ref().unwrap();
    assert!(std::fs::read_to_string(daily_path)
        .unwrap()
        .contains("CEO Autopilot Daily Summary"));
    assert!(std::fs::read_to_string(weekly_path)
        .unwrap()
        .contains("CEO Autopilot Weekly Summary"));
}

#[tokio::test]
async fn readiness_gate_holds_incomplete_lane_before_queue_promotion() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-readiness-hold-*".into();
    write_allow_readiness_artifacts(dir.path());
    std::fs::write(
        dir.path()
            .join("data/prometheus/autonomy_operating_loop_preflight.json"),
        serde_json::json!({
            "schema_version": "arda.autonomy_operating_loop_preflight.v1",
            "loop": {"missing_required_stages": []},
            "summary": {
                "lane_count": 12,
                "lane_configured_count": 11,
                "lane_incomplete_count": 1
            }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-readiness-hold","review_required":false,"approval_packet":{"approval_id":"approval-reco-readiness-hold","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_readiness_hold","owner":"prometheus","priority":"high","title":"Refactor local module"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.objectives_processed, 1);
    assert_eq!(report.autonomy_readiness.decision, "hold");
    assert!(!report.autonomy_readiness.task_promotion_allowed);
    assert!(report
        .autonomy_readiness
        .reasons
        .iter()
        .any(|reason| reason == "lane_health_incomplete:1"));
    assert!(report.plans[0].queued_task_ids.is_empty());
    assert_eq!(
        report.plans[0]
            .queue_operation
            .as_ref()
            .map(|operation| &operation.result_status),
        Some(&arda_prometheus::autopilot::QueueOperationStatus::BlockedAutonomyReadiness)
    );
    assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
}

#[tokio::test]
async fn readiness_gate_requires_human_for_hades_cleanup_approval_packets() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-readiness-human-*".into();
    write_allow_readiness_artifacts(dir.path());
    std::fs::write(
        dir.path()
            .join("data/hades/autonomy_cleanup_approval_packets.json"),
        serde_json::json!({
            "schema_version": "arda.hades.cleanup_approval_packets.v1",
            "candidate_count": 1,
            "cleanup_authorized": false,
            "requires_operator_approval_for_mutation": true,
            "no_file_moves_or_deletes_performed": true,
            "packets": [{"packet_id": "hades_cleanup_approval_demo", "mutation_allowed": false}]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-readiness-human","review_required":false,"approval_packet":{"approval_id":"approval-reco-readiness-human","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_readiness_human","owner":"prometheus","priority":"high","title":"Refactor local module"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.objectives_processed, 1);
    assert_eq!(report.autonomy_readiness.decision, "human_required");
    assert!(report.autonomy_readiness.human_required);
    assert!(report
        .autonomy_readiness
        .reasons
        .iter()
        .any(|reason| reason == "hades_cleanup_packets_need_operator_approval:1"));
    assert!(report.plans[0].queued_task_ids.is_empty());
    assert_eq!(
        report.plans[0]
            .queue_operation
            .as_ref()
            .map(|operation| &operation.result_status),
        Some(&arda_prometheus::autopilot::QueueOperationStatus::BlockedAutonomyReadiness)
    );
    assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
}

#[tokio::test]
async fn readiness_gate_holds_external_source_lanes_without_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-readiness-external-*".into();
    write_allow_readiness_artifacts(dir.path());
    std::fs::write(
        dir.path()
            .join("data/athena/external_source_lane_ledger.jsonl"),
        r#"{"schema_version":"arda.athena.external_source_lane.v1","source_id":"reddit","task_promotion_allowed":false,"verification_status":"not_ingested"}
"#,
    )
    .unwrap();
    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-readiness-external","review_required":false,"approval_packet":{"approval_id":"approval-reco-readiness-external","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_readiness_external","owner":"prometheus","priority":"high","title":"Refactor local module"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.autonomy_readiness.decision, "hold");
    assert!(report
        .autonomy_readiness
        .reasons
        .iter()
        .any(|reason| reason == "external_source_lanes_without_canonical_receipts:1"));
    assert!(report.plans[0].queued_task_ids.is_empty());
    assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
}

#[tokio::test]
async fn readiness_gate_requires_human_for_council_unresolved_escalation() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-readiness-council-*".into();
    write_allow_readiness_artifacts(dir.path());
    std::fs::write(
        dir.path().join("data/council/agent_conversations.jsonl"),
        r#"{"schema_version":"arda.council.agent_conversation.v1","conversation_id":"council-escalation","actionability":"gated_action","risk_lane":"human_gated","summary":"needs operator approval"}
"#,
    )
    .unwrap();
    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-readiness-council","review_required":false,"approval_packet":{"approval_id":"approval-reco-readiness-council","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_readiness_council","owner":"prometheus","priority":"high","title":"Refactor local module"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.autonomy_readiness.decision, "human_required");
    assert!(report
        .autonomy_readiness
        .reasons
        .iter()
        .any(|reason| reason == "council_unresolved_escalation:1"));
    assert!(report.plans[0].queued_task_ids.is_empty());
    assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
}

#[tokio::test]
async fn readiness_gate_holds_when_sovereign_adapter_contract_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-readiness-sovereign-*".into();
    write_allow_readiness_artifacts(dir.path());
    std::fs::remove_file(dir.path().join("config/autonomy_operating_loop.toml")).unwrap();
    std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.queue_path, "").unwrap();
    std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
    std::fs::write(&cfg.objectives_path, "").unwrap();
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-readiness-sovereign","review_required":false,"approval_packet":{"approval_id":"approval-reco-readiness-sovereign","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_readiness_sovereign","owner":"prometheus","priority":"high","title":"Refactor local module"}}
"#,
    )
    .unwrap();

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(report.autonomy_readiness.decision, "hold");
    assert!(report
        .autonomy_readiness
        .reasons
        .iter()
        .any(|reason| reason == "sovereign_adapter_projection_missing_config"));
    assert!(report.plans[0].queued_task_ids.is_empty());
    assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
}

#[tokio::test]
async fn read_only_cycle_reports_hades_introspection_without_mutating_runtime_surfaces() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.read_only = true;
    cfg.systemd_pattern = "arda-ceo-autopilot-read-only-hades-*".into();

    std::fs::create_dir_all(cfg.queue_path.parent().expect("queue parent")).expect("queue dir");
    std::fs::write(&cfg.queue_path, "").expect("queue seed");
    std::fs::create_dir_all(cfg.objectives_path.parent().expect("objectives parent"))
        .expect("objectives dir");
    std::fs::write(&cfg.objectives_path, "").expect("objectives seed");

    let hades_dir = dir.path().join("data/hades");
    std::fs::create_dir_all(&hades_dir).expect("hades dir");
    let policy_report_path = hades_dir.join("lifecycle_policy_automation_report.json");
    let review_queue_path = hades_dir.join("lifecycle_review_queue.jsonl");
    std::fs::write(
        &review_queue_path,
        r#"{"contract":"arda.hades.lifecycle_review_queue_projection.v1","review_id":"hlq_stale_plan","path":"docs/plans/stale.md","review_required":true,"destructive_allowed":false}
"#,
    )
    .expect("review queue");
    std::fs::write(
        &policy_report_path,
        serde_json::json!({
            "contract": "arda.hades.lifecycle_policy_automation_report.v1",
            "generated_at_utc": "2026-05-31T00:00:00Z",
            "source_findings_total": 1,
            "report_path": policy_report_path.display().to_string(),
            "policy_summary": {
                "findings_total": 1,
                "consistency_holds_total": 1,
                "by_disposition": {"hold": 1}
            },
            "review_queue_projection_recommended": true,
            "cleanup_authorized": false,
            "requires_operator_approval_for_mutation": true,
            "no_file_moves_or_deletes_performed": true
        })
        .to_string(),
    )
    .expect("policy report");

    let queue_before = std::fs::read_to_string(&cfg.queue_path).expect("queue before");
    let review_queue_before = std::fs::read_to_string(&review_queue_path).expect("review before");

    let mut autopilot = CeoAutopilot::new(cfg.clone(), registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(
        std::fs::read_to_string(&cfg.queue_path).expect("queue after"),
        queue_before
    );
    assert_eq!(
        std::fs::read_to_string(&review_queue_path).expect("review after"),
        review_queue_before
    );
    assert_eq!(report.objectives_processed, 0);
    assert_eq!(report.report_path, None);
    assert_eq!(report.weekly_report_path, None);
    assert_eq!(
        report.hades_introspection.contract,
        "arda.prometheus.hades_introspection_projection.v1"
    );
    assert_eq!(
        report.hades_introspection.source_contract.as_deref(),
        Some("arda.hades.lifecycle_policy_automation_report.v1")
    );
    assert_eq!(report.hades_introspection.source_findings_total, 1);
    assert_eq!(report.hades_introspection.consistency_holds_total, 1);
    assert_eq!(report.hades_introspection.review_queue_records, 1);
    assert!(!report.hades_introspection.cleanup_authorized);
    assert!(
        report
            .hades_introspection
            .requires_operator_approval_for_mutation
    );
    assert!(
        report
            .hades_introspection
            .no_file_moves_or_deletes_performed
    );
}

#[tokio::test]
async fn cycle_reports_sovereign_adapter_receipts_from_portable_loop_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.read_only = true;
    cfg.systemd_pattern = "arda-ceo-autopilot-sovereign-adapters-*".into();

    std::fs::create_dir_all(cfg.queue_path.parent().expect("queue parent")).expect("queue dir");
    std::fs::write(&cfg.queue_path, "").expect("queue seed");
    std::fs::create_dir_all(cfg.objectives_path.parent().expect("objectives parent"))
        .expect("objectives dir");
    std::fs::write(&cfg.objectives_path, "").expect("objectives seed");
    std::fs::create_dir_all(dir.path().join("config")).expect("config dir");
    std::fs::write(
        dir.path().join("config/autonomy_operating_loop.toml"),
        r#"
[[sovereign_crates]]
id = "governance"
crate = "arda-governance"
loop_stages = ["assess", "plan", "task"]
gate = "classify_action_and_required_authority_before_task_or_execute"
status = "contract_required"

[[sovereign_crates]]
id = "oracle"
crate = "arda-oracle"
loop_stages = ["assess", "review", "confirm"]
gate = "validate_before_high_risk_promotion_and_before_claiming_done"
status = "active_prototype"

[[sovereign_crates]]
id = "plutus"
crate = "arda-plutus"
loop_stages = ["plan", "execute", "audit"]
gate = "budget_receipt_before_delegation_and_review_after_execution"
status = "contract_required"

[[sovereign_crates]]
id = "human"
crate = "arda-human"
loop_stages = ["deliberate", "review", "confirm"]
gate = "human_required_action_classes_need_explicit_human_approval"
status = "contract_required"

[[sovereign_crates]]
id = "council"
crate = "arda-aule"
loop_stages = ["deliberate", "review", "replan"]
gate = "evidence_for_governance_not_approval_by_itself"
status = "active_subordinate"
"#,
    )
    .expect("loop config");
    let council_ledger = dir.path().join("data/council/agent_conversations.jsonl");
    std::fs::create_dir_all(council_ledger.parent().expect("council parent")).expect("council dir");
    std::fs::write(
        &council_ledger,
        r#"{"conversation_id":"c1","agent":"oracle","body":"needs review"}
"#,
    )
    .expect("council ledger");

    let mut autopilot = CeoAutopilot::new(cfg, registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(
        report.sovereign_adapters.contract,
        "arda.prometheus.sovereign_adapter_projection.v1"
    );
    assert!(report.sovereign_adapters.source_available);
    assert_eq!(report.sovereign_adapters.adapter_count, 5);
    assert_eq!(report.sovereign_adapters.active_runtime_adapter_count, 4);
    assert_eq!(report.sovereign_adapters.evidence_only_adapter_count, 1);
    assert_eq!(report.sovereign_adapters.missing_required_adapter_count, 0);
    assert!(report
        .sovereign_adapters
        .adapters
        .iter()
        .any(|adapter| adapter.id == "council"
            && adapter.gate_effect == "evidence_only"
            && adapter.source_records == 1));
}

#[tokio::test]
async fn mutating_cycle_appends_evidence_only_council_runtime_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = AutopilotConfig::from_root(dir.path());
    cfg.systemd_pattern = "arda-ceo-autopilot-council-runtime-*".into();
    write_allow_readiness_artifacts(dir.path());

    std::fs::create_dir_all(cfg.queue_path.parent().expect("queue parent")).expect("queue dir");
    std::fs::write(&cfg.queue_path, "").expect("queue seed");
    std::fs::create_dir_all(cfg.objectives_path.parent().expect("objectives parent"))
        .expect("objectives dir");
    std::fs::write(&cfg.objectives_path, "").expect("objectives seed");
    std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg.arandur_recommendations_path,
        r#"{"recommendation_id":"reco-council-runtime","review_required":false,"approval_packet":{"approval_id":"approval-reco-council-runtime","status":"approved","approved_by":"operator","approved_at":"2026-05-31T00:00:00Z"},"candidate":{"id":"obj_council_runtime","owner":"prometheus","priority":"high","title":"Review council runtime evidence wiring"}}
"#,
    )
    .expect("recommendation");

    let council_ledger = dir.path().join("data/council/agent_conversations.jsonl");
    let mut autopilot = CeoAutopilot::new(cfg, registry());
    let report = autopilot.run_cycle().await;

    assert_eq!(
        report.council_runtime.contract,
        "arda.prometheus.council_runtime_projection.v1"
    );
    assert_eq!(report.council_runtime.appended_record_count, 1);
    assert_eq!(report.council_runtime.existing_record_count, 1);
    assert!(report.council_runtime.evidence_only);
    assert!(!report.council_runtime.task_promotion_allowed);

    let ledger = std::fs::read_to_string(council_ledger).expect("council ledger");
    let value: serde_json::Value = serde_json::from_str(ledger.trim()).expect("council json");
    assert_eq!(
        value["schema_version"],
        "arda.council.agent_conversation.v1"
    );
    assert_eq!(value["speaker_agent"], "prometheus");
    assert_eq!(value["message_class"], "receipt");
    assert_eq!(value["actionability"], "completed_evidence");
    assert!(value["summary"]
        .as_str()
        .unwrap_or("")
        .contains("does not approve execution"));
}
