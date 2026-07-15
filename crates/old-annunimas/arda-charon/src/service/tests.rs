#![allow(clippy::await_holding_lock)]

use super::status::PackageRuntimeSignals;
use super::{append_jsonl, CharonService};
use crate::service::route_policy::{
    derive_route_execution_profile, model_supports_request, provider_eligible, provider_score,
    resolve_hybrid_route_policy, HybridRoutePolicy, LaneFitnessSnapshot, RouteExecutionProfile,
    RouteSelectionCandidate,
};
use crate::types::{
    CharonRequestEnvelope, ModelState, ProviderState, RouteDecision, RouteGovernance,
    RouteLoveEquationGuard,
};
use arda_plutus::PlutusService;
use chrono::Utc;
use std::fs;
use std::sync::Mutex;
use tempfile::tempdir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn isolate_env_lock(lock: &Mutex<()>) -> std::sync::MutexGuard<()> {
    lock.lock().unwrap_or_else(|e| e.into_inner())
}

fn isolate_provider_sources(dir: &std::path::Path) {
    std::env::set_var(
        "ARDA_PROVIDER_INTELLIGENCE_PATH",
        dir.join("provider_intelligence.json"),
    );
    std::env::set_var(
        "ARDA_CHARON_PROVIDER_CONFIG",
        dir.join("charon.providers.toml"),
    );
    std::env::set_var(
        "ARDA_FLEET_BOOTSTRAP_STATE",
        dir.join("fleet_bootstrap.json"),
    );
}

fn isolate_test_provider_config(dir: &std::path::Path) {
    std::env::set_var(
        "ARDA_CHARON_PROVIDER_CONFIG",
        dir.join("missing-charon.providers.toml"),
    );
    std::env::set_var("MISTRAL_API_KEY", "test");
    std::env::set_var("ZAI_API_KEY", "test");
    std::env::set_var("ANTHROPIC_API_KEY", "test");
    std::env::set_var("NVIDIA_API_KEY", "test");
    std::env::set_var("CEREBRAS_API_KEY", "test");
    std::env::set_var("GROQ_API_KEY", "test");
    std::env::set_var("GEMINI_API_KEY", "test");
}

fn clear_provider_sources() {
    std::env::remove_var("ARDA_PROVIDER_INTELLIGENCE_PATH");
    std::env::remove_var("ARDA_CHARON_PROVIDER_CONFIG");
    std::env::remove_var("ARDA_FLEET_BOOTSTRAP_STATE");
}

#[tokio::test]
async fn route_selects_provider_and_writes_state() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let plutus_home = dir.path().join("plutus");
    std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"test"})],
        options: serde_json::json!({}),
    };
    let out = svc.route(req).await.expect("route");
    assert!(!out.provider_id.is_empty());
    let governance_events = svc.recent_governance_events(8);
    assert!(governance_events
        .iter()
        .any(|event| { event.get("event").and_then(|v| v.as_str()) == Some("route_selected") }));
    let status = svc.status().await.expect("status");
    assert!(status.providers_total >= 1);
    assert!(status
        .governance_events_path
        .ends_with("governance_events.jsonl"));
    let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
    let mut total = 0.0;
    for _ in 0..100 {
        total = plutus.status().await.expect("plutus status")["joulework"]["total"]
            .as_f64()
            .unwrap_or(0.0);
        if total > 0.0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    std::env::remove_var("ARDA_PLUTUS_HOME");
    clear_provider_sources();
    drop(_guard);
    assert!(total > 0.0);
}

#[tokio::test]
async fn tool_fit_observation_writes_sanitized_ledger_row() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "forge".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"edit file"})],
        options: serde_json::json!({}),
    };
    let decision = RouteDecision {
        provider_id: "local_fallback".to_string(),
        model_id: "Qwen3-8B-Q4_K_M".to_string(),
        reason: "test".to_string(),
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 8192,
        governance: RouteGovernance {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 1.0,
                attention: 1.0,
                reciprocity: 1.0,
                score: 1.0,
            },
            ..RouteGovernance::default()
        },
        route_id: "route-test".to_string(),
    };
    let body = serde_json::json!({
        "model": "Qwen3-8B-Q4_K_M",
        "messages": [
            {"role": "user", "content": "edit file"},
            {"role": "tool", "tool_call_id": "call_1", "content": "done"}
        ],
        "tools": [{"type":"function","function":{"name":"write_file"}}],
        "tool_choice": "auto",
        "response_format": {"type":"json_object"}
    });
    svc.record_tool_fit_observation(
        &decision,
        &req,
        &body,
        super::state_mutation::ToolFitOutcome {
            ok: true,
            latency_ms: Some(42),
            status_code: Some(200),
            outcome_class: "success".to_string(),
            error: None,
        },
    )
    .expect("record observation");

    let ledger = fs::read_to_string(dir.path().join("tool_fit_ledger.jsonl")).expect("ledger");
    let row: serde_json::Value =
        serde_json::from_str(ledger.lines().next().expect("row")).expect("json row");
    assert_eq!(row["provider_id"], "local_fallback");
    assert_eq!(row["model_id"], "Qwen3-8B-Q4_K_M");
    assert_eq!(row["tool_request"], true);
    assert_eq!(row["tool_schema_count"], 1);
    assert_eq!(row["tool_history_present"], true);
    assert_eq!(row["structured_output_request"], true);
    assert!(row.get("messages").is_none());
    let events = svc.recent_state_events(4);
    assert!(events.iter().any(|event| {
        event.get("event").and_then(|value| value.as_str()) == Some("tool_fit_observation")
    }));
    clear_provider_sources();
}

#[tokio::test]
async fn payload_capability_failure_writes_receipt_and_gates_model_tools() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "forge".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"call a tool"})],
        options: serde_json::json!({"tools":[{"type":"function","function":{"name":"lookup"}}]}),
    };
    let decision = RouteDecision {
        provider_id: "local_fallback".to_string(),
        model_id: "Qwen3-8B-Q4_K_M".to_string(),
        reason: "test".to_string(),
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 8192,
        governance: RouteGovernance {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 1.0,
                attention: 1.0,
                reciprocity: 1.0,
                score: 1.0,
            },
            ..RouteGovernance::default()
        },
        route_id: "route-test".to_string(),
    };
    let body = serde_json::json!({
        "model": "Qwen3-8B-Q4_K_M",
        "messages": [{"role": "user", "content": "call a tool"}],
        "tools": [{"type":"function","function":{"name":"lookup"}}],
        "tool_choice": "auto"
    });
    svc.record_tool_fit_observation(
        &decision,
        &req,
        &body,
        super::state_mutation::ToolFitOutcome {
            ok: false,
            latency_ms: Some(12),
            status_code: Some(400),
            outcome_class: "client_payload_error".to_string(),
            error: Some("tools not supported".to_string()),
        },
    )
    .expect("record observation");

    let providers = svc.providers_read().await;
    let provider = providers
        .iter()
        .find(|provider| provider.id == "local_fallback")
        .expect("provider");
    let model = provider
        .models
        .iter()
        .find(|model| model.id == "Qwen3-8B-Q4_K_M")
        .expect("model");
    assert_eq!(model.capabilities.tools, Some(false));
    assert!(!model_supports_request("local_fallback", model, Some(&req)));
    drop(providers);

    let receipts =
        fs::read_to_string(dir.path().join("provider_capability_receipts.json")).expect("receipts");
    let receipts: serde_json::Value = serde_json::from_str(&receipts).expect("receipt json");
    assert_eq!(
        receipts["receipts"]["local_fallback::Qwen3-8B-Q4_K_M"]["capabilities"]["tools"]["state"],
        "failed"
    );
    clear_provider_sources();
}

#[tokio::test]
async fn rate_limit_observation_does_not_poison_capability_receipts() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "forge".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"call a tool"})],
        options: serde_json::json!({"tools":[{"type":"function","function":{"name":"lookup"}}]}),
    };
    let decision = RouteDecision {
        provider_id: "local_fallback".to_string(),
        model_id: "Qwen3-8B-Q4_K_M".to_string(),
        reason: "test".to_string(),
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 8192,
        governance: RouteGovernance {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 1.0,
                attention: 1.0,
                reciprocity: 1.0,
                score: 1.0,
            },
            ..RouteGovernance::default()
        },
        route_id: "route-test".to_string(),
    };
    let body = serde_json::json!({
        "model": "Qwen3-8B-Q4_K_M",
        "messages": [{"role": "user", "content": "call a tool"}],
        "tools": [{"type":"function","function":{"name":"lookup"}}]
    });
    svc.record_tool_fit_observation(
        &decision,
        &req,
        &body,
        super::state_mutation::ToolFitOutcome {
            ok: false,
            latency_ms: Some(12),
            status_code: Some(429),
            outcome_class: "rate_or_retry_error".to_string(),
            error: Some("rate limit".to_string()),
        },
    )
    .expect("record observation");

    let providers = svc.providers_read().await;
    let model = providers
        .iter()
        .find(|provider| provider.id == "local_fallback")
        .and_then(|provider| {
            provider
                .models
                .iter()
                .find(|model| model.id == "Qwen3-8B-Q4_K_M")
        })
        .expect("model");
    assert_ne!(model.capabilities.tools, Some(false));
    assert!(!dir
        .path()
        .join("provider_capability_receipts.json")
        .exists());
    clear_provider_sources();
}

#[tokio::test]
async fn visible_reasoning_leak_marks_model_as_visible_reasoning_surface() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"hello"})],
        options: serde_json::json!({}),
    };
    let decision = RouteDecision {
        provider_id: "local_fallback".to_string(),
        model_id: "Qwen3-8B-Q4_K_M".to_string(),
        reason: "test".to_string(),
        route_class: "interactive".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: 8192,
        governance: RouteGovernance {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 1.0,
                attention: 1.0,
                reciprocity: 1.0,
                score: 1.0,
            },
            ..RouteGovernance::default()
        },
        route_id: "route-visible-reasoning".to_string(),
    };
    let body = serde_json::json!({
        "model": "Qwen3-8B-Q4_K_M",
        "messages": [{"role": "user", "content": "hello"}]
    });
    svc.record_tool_fit_observation(
        &decision,
        &req,
        &body,
        super::state_mutation::ToolFitOutcome {
            ok: false,
            latency_ms: Some(20),
            status_code: Some(200),
            outcome_class: "visible_reasoning_leak".to_string(),
            error: None,
        },
    )
    .expect("record observation");

    let providers = svc.providers_read().await;
    let model = providers
        .iter()
        .find(|provider| provider.id == "local_fallback")
        .and_then(|provider| {
            provider
                .models
                .iter()
                .find(|model| model.id == "Qwen3-8B-Q4_K_M")
        })
        .expect("model");
    assert_eq!(model.capabilities.visible_reasoning, Some(true));
    clear_provider_sources();
}

#[tokio::test]
async fn provider_result_trips_cooldown_after_failures() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(800),
        Some("timeout".to_string()),
    )
    .await
    .expect("result1");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(900),
        Some("timeout".to_string()),
    )
    .await
    .expect("result2");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(950),
        Some("timeout".to_string()),
    )
    .await
    .expect("result3");

    let providers = svc.providers().await;
    let local = providers
        .iter()
        .find(|p| p.id == "local_fallback")
        .expect("local provider");
    assert!(local.in_cooldown);
    assert!(local.error_count >= 3);
}

#[tokio::test]
async fn provider_success_clears_existing_cooldown() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(800),
        Some("timeout".to_string()),
    )
    .await
    .expect("result1");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(900),
        Some("timeout".to_string()),
    )
    .await
    .expect("result2");
    svc.mark_provider_result(
        "local_fallback",
        false,
        Some(950),
        Some("timeout".to_string()),
    )
    .await
    .expect("result3");

    let providers = svc.providers().await;
    let local = providers
        .iter()
        .find(|p| p.id == "local_fallback")
        .expect("local provider");
    assert!(local.in_cooldown);
    drop(providers);

    svc.mark_provider_result("local_fallback", true, Some(100), None)
        .await
        .expect("success result");
    let providers = svc.providers().await;
    let local = providers
        .iter()
        .find(|p| p.id == "local_fallback")
        .expect("local provider");
    assert!(!local.in_cooldown);
    assert!(local.cooldown_until_utc.is_none());
    assert_eq!(local.cooldown_backoff_seconds, 0);
    assert_eq!(local.consecutive_failures, 0);
}

#[tokio::test]
async fn half_open_route_preserves_failure_streak_until_confirmed_success() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    std::env::set_var("ARDA_CHARON_HALF_OPEN_PROBE_STRIDE", "1");
    let dir = tempdir().expect("tempdir");
    isolate_test_provider_config(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    {
        let mut providers = svc.providers.write().await;
        let provider = providers
            .iter_mut()
            .find(|provider| provider.id == "local_fallback")
            .expect("local_fallback provider");
        provider.consecutive_failures = 3;
        provider.consecutive_successes = 0;
        provider.in_cooldown = false;
        provider.cooldown_until_utc = None;
        provider.last_error = Some("timeout".to_string());
    }

    // local_fallback routes via the execution/tool lane; align the request task_type
    // and options so the model selector finds a compatible local fallback model.
    let req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "execution".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"probe recovery"})],
        options: serde_json::json!({
            "force_provider_id": "local_fallback",
            "execution_lane": "execution",
            "tool_use_required": false,
        }),
    };
    let (decision, _) = svc.route_and_resolve(req).await.expect("route");
    assert_eq!(decision.provider_id, "local_fallback");
    {
        let providers = svc.providers.read().await;
        let provider = providers
            .iter()
            .find(|provider| provider.id == "local_fallback")
            .expect("local_fallback provider");
        assert_eq!(provider.consecutive_failures, 3);
        assert!(provider.last_error.is_some());
    }

    svc.mark_provider_result("local_fallback", true, Some(100), None)
        .await
        .expect("result");
    let providers = svc.providers().await;
    let provider = providers
        .iter()
        .find(|provider| provider.id == "local_fallback")
        .expect("local_fallback provider");
    std::env::remove_var("ARDA_CHARON_HALF_OPEN_PROBE_STRIDE");
    drop(_guard);

    assert_eq!(provider.consecutive_failures, 0);
    assert!(!provider.in_cooldown);
}

#[tokio::test]
async fn per_agent_quota_blocks_only_exhausted_agent() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    std::env::set_var("ARDA_CHARON_AGENT_REQUESTS_PER_DAY", "1");
    let dir = tempdir().expect("tempdir");
    isolate_test_provider_config(dir.path());
    let svc = CharonService::new(dir.path()).expect("service");
    let req_for = |agent_id: &str| CharonRequestEnvelope {
        agent_id: agent_id.to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"quota test"})],
        options: serde_json::json!({"force_provider_id": "local_fallback"}),
    };

    let first = svc
        .route_and_resolve(req_for("agent_a"))
        .await
        .expect("first route");
    assert_eq!(first.0.provider_id, "local_fallback");

    let second = svc.route_and_resolve(req_for("agent_a")).await;
    assert!(second.is_err());

    let other_agent = svc
        .route_and_resolve(req_for("agent_b"))
        .await
        .expect("other agent route");
    std::env::remove_var("ARDA_CHARON_AGENT_REQUESTS_PER_DAY");
    drop(_guard);

    assert_eq!(other_agent.0.provider_id, "local_fallback");
}

#[tokio::test]
async fn bandit_records_provider_result_and_affects_score() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    std::env::set_var("ARDA_CHARON_BANDIT_MIN_OBSERVATIONS", "1");
    std::env::set_var("ARDA_CHARON_BANDIT_SCORE_WEIGHT", "8");
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    let tool_req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"bandit test"})],
        options: serde_json::json!({
            "force_provider_id": "local_fallback",
            "tools": [{
                "type": "function",
                "function": {"name": "lookup", "parameters": {"type": "object"}}
            }]
        }),
    };
    let plain_req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"bandit test"})],
        options: serde_json::json!({"force_provider_id": "local_fallback"}),
    };

    let provider_id = "local_fallback";
    let model_id = "Qwen3-8B-Q4_K_M";
    svc.record_bandit_route(&tool_req, provider_id, model_id);
    svc.mark_provider_result(provider_id, true, Some(25), None)
        .await
        .expect("result");
    let tool_bonus = svc.bandit_score_bonus(&tool_req, provider_id, model_id);
    let plain_bonus = svc.bandit_score_bonus(&plain_req, provider_id, model_id);
    std::env::remove_var("ARDA_CHARON_BANDIT_MIN_OBSERVATIONS");
    std::env::remove_var("ARDA_CHARON_BANDIT_SCORE_WEIGHT");
    drop(_guard);

    assert!(tool_bonus > 0.0);
    assert_eq!(plain_bonus, 0.0);
    assert!(dir.path().join("bandit.json").exists());
}

#[test]
fn credit_limited_402_is_treated_as_fallbackable() {
    let parsed = serde_json::json!({
        "error": {
            "code": 402,
            "message": "This request requires more credits, or fewer max_tokens. You requested up to 65536 tokens, but can only afford 21059."
        }
    });

    assert!(super::provider_error_should_fallback(402, &parsed));
}

#[test]
fn low_cost_orchestrator_prefers_free_context_fit_before_local() {
    let free = ProviderState {
        id: "openrouter_free".to_string(),
        name: "OpenRouter Free".to_string(),
        base_url: Some("https://openrouter.ai/api/v1".to_string()),
        enabled: true,
        healthy: true,
        api_key_env: Some("OPENROUTER_API_KEY".to_string()),
        has_api_key: true,
        models: vec![ModelState {
            aliases: vec![],
            id: "free-ctx".to_string(),
            capable_tasks: vec![
                "code".to_string(),
                "research".to_string(),
                "reasoning".to_string(),
            ],
            context_window: 200_000,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::types::ModelCapabilities::default(),
            streaming_validated: None,
        }],
        access_tier: "free_cloud".to_string(),
        quality_band: "high".to_string(),
        requests_per_minute: Some(20),
        requests_per_day: Some(50),
        requests_used_minute: 0,
        requests_used_day: 0,
        active_connections: 0,
        avg_latency_ms: Some(1800),
        error_count: 0,
        last_error: None,
        in_cooldown: false,
        cooldown_until_utc: None,
        minute_window_started_utc: None,
        day_window_started_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_reservation_utc: None,
        supports_tools: true,
        supports_structured_output: true,
        driver: "openai_compat".to_string(),
        hermes_bin: None,
        hermes_provider: None,
        hermes_toolsets: None,
        cooldown_backoff_seconds: 120,
        intelligence_refreshed_at_utc: None,
        probe_model: None,
        probe_profile: None,
    };
    let local = ProviderState {
        id: "edge_backbone".to_string(),
        name: "3080".to_string(),
        base_url: Some("http://127.0.0.1:1234".to_string()),
        enabled: true,
        healthy: true,
        api_key_env: None,
        has_api_key: true,
        models: vec![ModelState {
            aliases: vec![],
            id: "qwen3.5-9b".to_string(),
            capable_tasks: vec!["code".to_string(), "research".to_string()],
            context_window: 32_768,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::types::ModelCapabilities::default(),
            streaming_validated: None,
        }],
        access_tier: "local".to_string(),
        quality_band: "high".to_string(),
        requests_per_minute: Some(60),
        requests_per_day: None,
        requests_used_minute: 0,
        requests_used_day: 0,
        active_connections: 0,
        avg_latency_ms: Some(120),
        error_count: 0,
        last_error: None,
        in_cooldown: false,
        cooldown_until_utc: None,
        minute_window_started_utc: None,
        day_window_started_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_reservation_utc: None,
        supports_tools: true,
        supports_structured_output: true,
        driver: "openai_compat".to_string(),
        hermes_bin: None,
        hermes_provider: None,
        hermes_toolsets: None,
        cooldown_backoff_seconds: 120,
        intelligence_refreshed_at_utc: None,
        probe_model: None,
        probe_profile: None,
    };
    let providers = vec![free, local];
    let mut candidates = vec![
        RouteSelectionCandidate {
            provider_index: 0,
            model: providers[0].models[0].clone(),
            score: 100.0,
        },
        RouteSelectionCandidate {
            provider_index: 1,
            model: providers[1].models[0].clone(),
            score: 200.0,
        },
    ];

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let route_profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };

    CharonService::retain_cost_tier_orchestrator_candidates(
        &mut candidates,
        &providers,
        &policy,
        &route_profile,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        providers[candidates[0].provider_index].id,
        "openrouter_free"
    );
}

#[test]
fn orchestrator_rejects_under_context_local_fallbacks() {
    let providers = [
        ProviderState {
            id: "edge_backbone".to_string(),
            name: "edge".to_string(),
            base_url: Some("http://edge".to_string()),
            api_key_env: None,
            access_tier: "local".to_string(),
            quality_band: "high".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                aliases: vec![],
                id: "qwen3.5-9b".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 32_768,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: Some(250),
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        },
        ProviderState {
            id: "edge_guardhouse".to_string(),
            name: "guardhouse".to_string(),
            base_url: Some("http://guardhouse".to_string()),
            api_key_env: None,
            access_tier: "local".to_string(),
            quality_band: "low".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                aliases: vec![],
                id: "Qwen3.5-4B-Q4_K_M.gguf".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 4_096,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: Some(500),
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        },
    ];
    let req = CharonRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"audit config"})],
        options: serde_json::json!({
            "workload_role": "orchestrator",
            "context_priority": "high",
            "context_window_target": 128_000,
            "cost_policy": "free_first",
            "quality_priority": "high",
            "privacy_requirement": "internal",
        }),
    };
    let priority = "normal".to_string();
    let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
    let route_profile = derive_route_execution_profile(&req, &priority);
    let package_runtime = PackageRuntimeSignals {
        generated_at_utc: Utc::now().to_rfc3339(),
        llmfit_backend: "optional_signal_absent".to_string(),
        llmfit_recommendation_count: 0,
        llmfit_local_max_params_b: None,
        llmfit_top_model_names: vec![],
        nanoclaw_binary_present: false,
        nanoclaw_runtime_ready: false,
        nanoclaw_probe_state: "not_configured".to_string(),
    };
    let lane_fitness = LaneFitnessSnapshot::default();

    let mut candidates = providers
        .iter()
        .enumerate()
        .filter(|(_, provider)| provider_eligible(provider, &priority, false))
        .filter_map(|(idx, provider)| {
            let model = super::route_policy::select_model(&provider.models, &req.task_type, None)?;
            Some(RouteSelectionCandidate {
                provider_index: idx,
                model: model.clone(),
                score: provider_score(
                    provider,
                    &model,
                    &priority,
                    &policy,
                    &route_profile,
                    &package_runtime,
                    &lane_fitness,
                ),
            })
        })
        .collect::<Vec<_>>();

    CharonService::retain_orchestrator_context_fit_candidates(&mut candidates, &route_profile);

    assert!(candidates.is_empty());
}

#[test]
fn unrelated_402_is_not_treated_as_fallbackable() {
    let parsed = serde_json::json!({
        "error": {
            "code": 402,
            "message": "request could not be processed"
        }
    });

    assert!(!super::provider_error_should_fallback(402, &parsed));
}

#[test]
fn mesh_provider_with_healthy_alternate_model_stays_routable() {
    let provider = ProviderState {
        id: "edge_backbone".to_string(),
        name: "Backbone".to_string(),
        base_url: Some("http://127.0.0.1:9337/v1".to_string()),
        api_key_env: None,
        access_tier: "local".to_string(),
        quality_band: "high".to_string(),
        intelligence_refreshed_at_utc: None,
        probe_model: None,
        probe_profile: None,
        enabled: true,
        has_api_key: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        cooldown_backoff_seconds: 120,
        requests_per_minute: None,
        requests_used_minute: 0,
        minute_window_started_utc: None,
        requests_per_day: None,
        requests_used_day: 0,
        day_window_started_utc: None,
        models: vec![
            ModelState {
                aliases: vec![],
                id: "MiniMax-M2.5-Q4_K_M".to_string(),
                capable_tasks: vec!["code".to_string(), "research".to_string()],
                context_window: 200_000,
                is_default: false,
                healthy: false,
                in_cooldown: true,
                cooldown_until_utc: Some(Utc::now().to_rfc3339()),
                consecutive_failures: 1,
                consecutive_successes: 0,
                last_error: Some("timeout".to_string()),
                avg_latency_ms: Some(30_000),
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            },
            ModelState {
                aliases: vec![],
                id: "Qwen3-8B-Q4_K_M".to_string(),
                capable_tasks: vec!["code".to_string(), "research".to_string()],
                context_window: 131_072,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 2,
                last_error: None,
                avg_latency_ms: Some(1_500),
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            },
        ],
        error_count: 0,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: None,
        active_connections: 0,
        last_reservation_utc: None,
        supports_tools: true,
        supports_structured_output: true,
        driver: "openai_compat".to_string(),
        hermes_bin: None,
        hermes_provider: None,
        hermes_toolsets: None,
    };

    assert!(super::provider_has_alternate_routable_model(
        &provider,
        "MiniMax-M2.5-Q4_K_M",
        "code",
        None,
    ));
}

#[test]
fn mesh_tunnel_error_is_treated_as_fallbackable() {
    let parsed = serde_json::json!({
        "error": {
            "message": "all 1 tunnel(s) to hosts for None failed (mesh request)"
        }
    });

    assert!(super::provider_error_should_fallback(400, &parsed));
}

#[test]
fn context_overflow_400_is_treated_as_fallbackable() {
    let parsed = serde_json::json!({
        "error": {
            "code": 400,
            "message": "request (18870 tokens) exceeds the available context size (16384 tokens)"
        }
    });

    assert!(super::provider_error_should_fallback(400, &parsed));
    assert!(super::is_context_overflow_error(400, &parsed));
}

#[test]
fn context_overflow_500_is_classified_as_context_overflow() {
    let parsed = serde_json::json!({
        "error": {
            "code": 500,
            "message": "Context size has been exceeded.",
            "type": "server_error"
        }
    });

    assert!(super::provider_error_should_fallback(500, &parsed));
    assert!(super::is_context_overflow_error(500, &parsed));
}

#[test]
fn nvidia_function_not_found_404_is_treated_as_fallbackable() {
    let parsed = serde_json::json!({
        "detail": "Function '7dfc10a8-3cc4-448e-97c1-2213308dc222': Not found for account 'acct_123'",
        "status": 404,
        "title": "Not Found"
    });

    assert!(super::provider_error_should_fallback(404, &parsed));
}

#[test]
fn nvidia_function_not_found_404_marks_model_unavailable() {
    let parsed = serde_json::json!({
        "detail": "Function '7dfc10a8-3cc4-448e-97c1-2213308dc222': Not found for account 'acct_123'",
        "status": 404,
        "title": "Not Found"
    });

    assert!(super::model_error_should_mark_unavailable(404, &parsed));
}

#[test]
fn provider_model_not_found_404_marks_model_unavailable() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Model nvidia/llama-3.1-nemotron-ultra-253b-v1 was not found or is not available"
        }
    });

    assert!(super::provider_error_should_fallback(404, &parsed));
    assert!(super::model_error_should_mark_unavailable(404, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("nvidia", 404, &parsed),
        Some(900)
    );
}

#[test]
fn provider_model_not_supported_400_marks_model_unavailable() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Error code: 400 - {'detail': \"The 'gpt-5-codex' model is not supported when using Codex with a ChatGPT account.\"}"
        }
    });

    assert!(super::provider_error_should_fallback(400, &parsed));
    assert!(super::model_error_should_mark_unavailable(400, &parsed));
}

#[test]
fn billing_exhaustion_429_is_provider_scoped_and_long_cooldown() {
    let parsed = serde_json::json!({
        "error": {
            "code": "1113",
            "message": "Insufficient balance or no resource package. Please recharge."
        }
    });

    assert!(super::provider_error_should_fallback(429, &parsed));
    assert!(super::is_billing_or_credit_error(429, &parsed));
    assert!(!super::is_request_scoped_retry_error(429, &parsed));
    assert!(!super::model_error_should_mark_unavailable(429, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("zai", 429, &parsed),
        Some(86_400)
    );
}

#[test]
fn payment_required_402_is_fallbackable_billing_exhaustion() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Payment required: billing limit reached"
        }
    });

    assert!(super::provider_error_should_fallback(402, &parsed));
    assert!(super::is_billing_or_credit_error(402, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("cerebras", 402, &parsed),
        Some(86_400)
    );
}

#[test]
fn nvidia_function_not_found_404_triggers_cooldown() {
    let parsed = serde_json::json!({
        "detail": "Function '7dfc10a8-3cc4-448e-97c1-2213308dc222': Not found for account 'acct_123'",
        "status": 404,
        "title": "Not Found"
    });

    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("nvidia", 404, &parsed),
        Some(900)
    );
}

#[test]
fn proxy_attempts_scale_with_provider_pool() {
    assert_eq!(super::proxy_max_attempts(1), 5);
    assert_eq!(super::proxy_max_attempts(4), 5);
    assert_eq!(super::proxy_max_attempts(20), 14);
}

#[test]
fn local_transport_failure_triggers_cooldown() {
    assert!(super::transport_failure_should_trigger_cooldown(
            "local_fallback",
            "proxy request failed to local_fallback: error sending request for url (http://127.0.0.1:11434/v1/chat/completions)",
        ));
    assert!(super::transport_failure_should_trigger_cooldown(
        "edge_backbone",
        "proxy request failed to edge_backbone: connection refused",
    ));
    assert!(super::transport_failure_should_trigger_cooldown(
            "edge_backbone",
            "provider edge_backbone HTTP 400: {\"error\":{\"message\":\"all 1 tunnel(s) to hosts for None failed (mesh request)\"}}",
        ));
    assert!(!super::transport_failure_should_trigger_cooldown(
        "openrouter_free",
        "provider openrouter_free HTTP 429",
    ));
}

#[test]
fn local_orchestrator_timeout_is_shorter_than_cloud_default() {
    // local_fallback + non-backbone edge_* providers take the generic local
    // timeout path (30s orchestrator). edge_backbone (sovereign 35B model)
    // intentionally has longer timeouts (120s orchestrator) because of the
    // larger model and mesh-tunnel overhead.
    assert_eq!(
        super::proxy_timeout_for_provider("local_fallback", "orchestrator"),
        std::time::Duration::from_secs(30)
    );
    assert_eq!(
        super::proxy_timeout_for_provider("edge_worker_light", "execution"),
        std::time::Duration::from_secs(20)
    );
    assert_eq!(
        super::proxy_timeout_for_provider("openrouter", "orchestrator"),
        std::time::Duration::from_secs(45)
    );
    assert_eq!(
        super::proxy_timeout_for_provider("edge_backbone", "orchestrator"),
        std::time::Duration::from_secs(120)
    );
    assert_eq!(
        super::proxy_timeout_for_provider("edge_backbone_coder", "execution"),
        std::time::Duration::from_secs(360)
    );
    assert_eq!(
        super::proxy_timeout_for_provider("edge_backbone_coder", "orchestrator"),
        std::time::Duration::from_secs(420)
    );
}

#[test]
fn plain_cloud_429_triggers_short_provider_cooldown() {
    let parsed = serde_json::json!({
        "error": {
            "message": "rate limited"
        }
    });

    assert!(!super::is_request_scoped_retry_error(429, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("openrouter_free", 429, &parsed),
        Some(300)
    );
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("openai_sub", 429, &parsed),
        Some(300)
    );
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("edge_backbone", 429, &parsed),
        None
    );
}

#[test]
fn typed_rate_limited_429_triggers_short_provider_cooldown() {
    let parsed = serde_json::json!({
        "code": "1300",
        "message": "Rate limit exceeded",
        "object": "error",
        "raw_status_code": 429,
        "type": "rate_limited"
    });

    assert!(super::provider_error_should_fallback(429, &parsed));
    assert!(!super::is_request_scoped_retry_error(429, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("mistral", 429, &parsed),
        Some(300)
    );
}

#[test]
fn credit_limited_402_triggers_immediate_provider_cooldown() {
    let parsed = serde_json::json!({
        "error": {
            "code": 402,
            "message": "This request requires more credits, or fewer max_tokens."
        }
    });

    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("openrouter", 402, &parsed),
        Some(86_400)
    );
}

#[test]
fn opencode_insufficient_balance_401_triggers_fallback_and_cooldown() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Insufficient balance. Manage your billing here: https://opencode.ai/workspace/wrk_x/billing",
            "type": "CreditsError"
        },
        "type": "error"
    });

    assert!(super::provider_error_should_fallback(401, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("opencode", 401, &parsed),
        Some(86_400)
    );
}

#[test]
fn slim_local_attempt_body_trims_prompt_and_tool_schema() {
    let mut body = serde_json::json!({
        "messages": [
            {
                "role": "system",
                "content": "A".repeat(12000),
            },
            {
                "role": "user",
                "content": "hello"
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "B".repeat(400),
                    "parameters": {
                        "type": "object",
                        "description": "outer",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "full path"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }
        ]
    });

    super::slim_local_attempt_body(&mut body);

    let system = body["messages"][0]["content"]
        .as_str()
        .expect("system text");
    assert!(system.len() < 9000);
    assert!(system.contains("trimmed oversized prompt"));

    let description = body["tools"][0]["function"]["description"]
        .as_str()
        .expect("tool description");
    assert!(description.len() <= 180);
    assert!(body["tools"][0]["function"]["parameters"]["description"].is_null());
    assert!(
        body["tools"][0]["function"]["parameters"]["properties"]["path"]["description"].is_null()
    );
}

#[test]
fn slim_local_attempt_body_flattens_tool_history() {
    let mut body = serde_json::json!({
        "messages": [
            {
                "role": "assistant",
                "content": "I am checking files.",
                "tool_calls": [
                    {
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"/tmp/x\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "line1\nline2\nline3"
            }
        ]
    });

    super::slim_local_attempt_body(&mut body);

    assert!(body["messages"][0]["tool_calls"].is_null());
    assert!(body["messages"][0]["content"]
        .as_str()
        .expect("assistant content")
        .contains("Tool calls issued: read_file"));
    assert_eq!(body["messages"][1]["role"].as_str(), Some("user"));
    assert!(body["messages"][1]["content"]
        .as_str()
        .expect("tool content")
        .contains("[Tool result for call_1]"));
}

#[test]
fn local_payload_requires_structured_tool_history_for_agentic_continuations() {
    let body = serde_json::json!({
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "read_file"
                }
            }
        ],
        "tool_choice": "auto",
        "messages": [
            {
                "role": "assistant",
                "tool_calls": [
                    {
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"/tmp/x\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "line1"
            }
        ]
    });

    assert!(super::local_payload_requires_structured_tool_history(&body));
}

#[test]
fn local_payload_without_tools_can_still_be_slimmed() {
    let body = serde_json::json!({
        "messages": [
            {
                "role": "system",
                "content": "hello"
            },
            {
                "role": "user",
                "content": "world"
            }
        ]
    });

    assert!(!super::local_payload_requires_structured_tool_history(
        &body
    ));
}

#[test]
fn optional_tools_are_stripped_before_plain_chat_proxying() {
    let req = CharonRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "Summarize active queue health."
        })],
        options: serde_json::json!({
            "tools_available": true,
            "tool_schema_count": 47,
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto"
        }),
    };
    let mut body = serde_json::json!({
        "model": "auto",
        "messages": req.messages.clone(),
        "tools": [{"type":"function","function":{"name":"terminal"}}],
        "tool_choice": "auto"
    });

    super::strip_optional_tool_payload(&req, &mut body);

    assert!(body.get("tools").is_none());
    assert!(body.get("tool_choice").is_none());
}

#[test]
fn required_tools_are_preserved_before_agentic_proxying() {
    let req = CharonRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "Run tools and update queue evidence."
        })],
        options: serde_json::json!({
            "tool_use_required": true,
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto"
        }),
    };
    let mut body = serde_json::json!({
        "model": "auto",
        "messages": req.messages.clone(),
        "tools": [{"type":"function","function":{"name":"terminal"}}],
        "tool_choice": "auto"
    });

    super::strip_optional_tool_payload(&req, &mut body);

    assert!(body.get("tools").is_some());
    assert_eq!(body["tool_choice"], "auto");
}

#[test]
fn strip_internal_openai_routing_fields_removes_charon_metadata() {
    let mut payload = serde_json::json!({
        "model": "auto",
        "messages": [{"role":"user","content":"hello"}],
        "agent_id": "hermes",
        "source_agent": "hermes",
        "routing": {"agent_id": "hermes", "source_agent": "hermes"},
        "workload_role": "execution",
        "quality_priority": "high",
        "cost_policy": "free_first",
        "privacy_requirement": "internal",
        "inference_origin": "cloud",
        "origin_preference": "cloud",
        "context_window_target": 32000,
        "execution_lane": "execution",
        "force_provider_id": "groq",
        "force_model_id": "llama-3.3-70b-versatile",
        "exclude_provider_ids": ["edge_backbone"],
        "exclude_model_ids": ["broken/model"],
        "prefer_probe_model": true,
        "source_surface": "hermes_agent_gateway",
        "harness": "hermes-agent-charon",
        "session_id": "sess-1",
        "conversation_id": "conv-1",
        "turn_id": "turn-1",
        "trace_id": "trace-1",
        "receipt_id": "receipt-1",
        "skill": "repo-maintenance",
        "skills": ["repo-maintenance"],
        "toolset": "filesystem",
        "toolsets": ["filesystem", "terminal"],
        "tool_mode": "auto",
        "agent_mode": "agentic",
        "tool_use_required": true
    });

    super::strip_internal_openai_routing_fields(payload.as_object_mut().expect("payload object"));

    assert!(payload.get("agent_id").is_none());
    assert!(payload.get("source_agent").is_none());
    assert!(payload.get("routing").is_none());
    assert!(payload.get("workload_role").is_none());
    assert!(payload.get("quality_priority").is_none());
    assert!(payload.get("cost_policy").is_none());
    assert!(payload.get("privacy_requirement").is_none());
    assert!(payload.get("inference_origin").is_none());
    assert!(payload.get("origin_preference").is_none());
    assert!(payload.get("context_window_target").is_none());
    assert!(payload.get("execution_lane").is_none());
    assert!(payload.get("force_provider_id").is_none());
    assert!(payload.get("force_model_id").is_none());
    assert!(payload.get("exclude_provider_ids").is_none());
    assert!(payload.get("exclude_model_ids").is_none());
    assert!(payload.get("prefer_probe_model").is_none());
    assert!(payload.get("source_surface").is_none());
    assert!(payload.get("harness").is_none());
    assert!(payload.get("session_id").is_none());
    assert!(payload.get("conversation_id").is_none());
    assert!(payload.get("turn_id").is_none());
    assert!(payload.get("trace_id").is_none());
    assert!(payload.get("receipt_id").is_none());
    assert!(payload.get("skill").is_none());
    assert!(payload.get("skills").is_none());
    assert!(payload.get("toolset").is_none());
    assert!(payload.get("toolsets").is_none());
    assert!(payload.get("tool_mode").is_none());
    assert!(payload.get("agent_mode").is_none());
    assert!(payload.get("tool_use_required").is_none());
    assert_eq!(payload["model"], "auto");
    assert_eq!(payload["messages"][0]["content"], "hello");
}

#[test]
fn normalize_openai_request_payload_fills_missing_tool_call_type() {
    let mut payload = serde_json::json!({
        "model": "auto",
        "messages": [
            {
                "role": "assistant",
                "content": "Let me verify the audit.",
                "tool_calls": [
                    {
                        "id": "call_1",
                        "type": null,
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"/tmp/x\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "result"
            }
        ]
    });

    super::normalize_openai_request_payload(&mut payload);

    assert_eq!(
        payload["messages"][0]["tool_calls"][0]["type"].as_str(),
        Some("function")
    );
}

#[test]
fn normalize_openai_request_payload_rewrites_invalid_tool_call_ids_consistently() {
    let mut payload = serde_json::json!({
        "model": "auto",
        "messages": [
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "call_d9e9",
                        "type": "function",
                        "function": {
                            "name": "execute_code",
                            "arguments": "{\"code\":\"print(1)\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_d9e9",
                "content": "1"
            }
        ]
    });

    super::normalize_openai_request_payload(&mut payload);

    let normalized_id = payload["messages"][0]["tool_calls"][0]["id"]
        .as_str()
        .expect("normalized tool call id");
    assert_eq!(normalized_id.len(), 9);
    assert!(normalized_id.chars().all(|ch| ch.is_ascii_alphanumeric()));
    assert_eq!(
        payload["messages"][1]["tool_call_id"].as_str(),
        Some(normalized_id)
    );
}

#[test]
fn normalize_openai_request_payload_strips_reasoning_replay_fields() {
    let mut payload = serde_json::json!({
        "model": "auto",
        "messages": [
            {
                "role": "assistant",
                "content": "The audit is complete.",
                "reasoning_content": "internal chain of thought",
                "reasoning_details": [
                    {
                        "type": "reasoning.text",
                        "text": "internal chain of thought"
                    }
                ]
            }
        ]
    });

    super::normalize_openai_request_payload(&mut payload);

    assert!(payload["messages"][0].get("reasoning_content").is_none());
    assert!(payload["messages"][0].get("reasoning_details").is_none());
    assert_eq!(
        payload["messages"][0]["content"].as_str(),
        Some("The audit is complete.")
    );
}

#[test]
fn normalize_openai_request_payload_can_preserve_reasoning_replay_fields() {
    let mut payload = serde_json::json!({
        "model": "auto",
        "messages": [
            {
                "role": "assistant",
                "content": "The audit is complete.",
                "reasoning_content": "provider-required replay",
                "reasoning_details": [
                    {
                        "type": "reasoning.text",
                        "text": "provider-required replay"
                    }
                ]
            }
        ]
    });

    super::normalize_openai_request_payload_with_policy(&mut payload, true);

    assert_eq!(
        payload["messages"][0]["reasoning_content"].as_str(),
        Some("provider-required replay")
    );
    assert!(payload["messages"][0]["reasoning_details"].is_array());
}

#[test]
fn normalize_openai_response_promotes_reasoning_content_when_content_is_empty() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "",
                    "reasoning_content": "Use the terminal tool to run pwd."
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    assert_eq!(
        payload["choices"][0]["message"]["content"].as_str(),
        Some("Use the terminal tool to run pwd.")
    );
}

#[test]
fn normalize_openai_response_strips_visible_think_block() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "\n<think>\nReasoning text that should not be surfaced.\n</think>\nLFM_CORE_OK"
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    assert_eq!(
        payload["choices"][0]["message"]["content"].as_str(),
        Some("LFM_CORE_OK")
    );
}

#[test]
fn normalize_openai_response_quarantines_unclosed_think_block() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "<think>\npartial reasoning"
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    assert_eq!(
        payload["choices"][0]["message"]["content"].as_str(),
        Some("")
    );
}

#[test]
fn normalize_openai_response_repairs_legacy_function_call_shape() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "finish_reason": "function_call",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "function_call": {
                        "name": "read_file",
                        "arguments": {"path": "Cargo.toml"}
                    }
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    let tool_call = &payload["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(
        payload["choices"][0]["finish_reason"].as_str(),
        Some("tool_calls")
    );
    assert_eq!(tool_call["type"].as_str(), Some("function"));
    assert!(tool_call["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("call_charon_")));
    assert_eq!(tool_call["function"]["name"].as_str(), Some("read_file"));
    assert_eq!(
        tool_call["function"]["arguments"].as_str(),
        Some(r#"{"path":"Cargo.toml"}"#)
    );
    assert!(payload["choices"][0]["message"]
        .get("function_call")
        .is_none());
}

#[test]
fn normalize_openai_response_fills_missing_tool_call_fields() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "function": {
                                "name": "ack",
                                "arguments": {"ok": true}
                            }
                        }
                    ]
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    let tool_call = &payload["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(
        payload["choices"][0]["finish_reason"].as_str(),
        Some("tool_calls")
    );
    assert!(tool_call["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("call_charon_")));
    assert_eq!(tool_call["type"].as_str(), Some("function"));
    assert_eq!(
        tool_call["function"]["arguments"].as_str(),
        Some(r#"{"ok":true}"#)
    );
}

#[test]
fn normalize_openai_response_repairs_tool_name_with_embedded_arguments() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "function": {
                                "name": "terminal {\"command\":\"pwd\",\"workdir\":\"/tmp\"}",
                                "arguments": "{}"
                            }
                        }
                    ]
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    let tool_call = &payload["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(
        payload["choices"][0]["finish_reason"].as_str(),
        Some("tool_calls")
    );
    assert_eq!(tool_call["function"]["name"].as_str(), Some("terminal"));
    assert_eq!(
        tool_call["function"]["arguments"].as_str(),
        Some(r#"{"command":"pwd","workdir":"/tmp"}"#)
    );
}

#[test]
fn normalize_openai_response_repairs_tool_name_with_adjacent_arguments() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "function": {
                                "name": "terminal{\"command\":\"pwd\"}"
                            }
                        }
                    ]
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    let tool_call = &payload["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(tool_call["function"]["name"].as_str(), Some("terminal"));
    assert_eq!(
        tool_call["function"]["arguments"].as_str(),
        Some(r#"{"command":"pwd"}"#)
    );
}

#[test]
fn normalize_openai_response_repairs_legacy_function_name_with_embedded_arguments() {
    let mut payload = serde_json::json!({
        "choices": [
            {
                "finish_reason": "function_call",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "function_call": {
                        "name": "terminal {\"command\":\"pwd\"}"
                    }
                }
            }
        ]
    });

    super::normalize_openai_response(&mut payload);

    let tool_call = &payload["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(tool_call["function"]["name"].as_str(), Some("terminal"));
    assert_eq!(
        tool_call["function"]["arguments"].as_str(),
        Some(r#"{"command":"pwd"}"#)
    );
}

#[test]
fn models_probe_classifies_opencode_credit_exhaustion_as_spend_blocked() {
    let (state, _, blocked, _) = super::classify_models_probe_status(
        "opencode",
        401,
        r#"{"error":{"message":"Insufficient balance","type":"CreditsError"}}"#,
        None,
    );

    assert_eq!(state, "spend_blocked");
    assert!(blocked);
}

#[test]
fn models_probe_classifies_generic_unauthorized_as_auth_failed() {
    let (state, _, blocked, _) =
        super::classify_models_probe_status("openai", 401, "Unauthorized", None);

    assert_eq!(state, "auth_failed");
    assert!(blocked);
}

#[test]
fn models_probe_classifies_success_as_ready() {
    let (state, reason, blocked, _) = super::classify_models_probe_status(
        "mistral",
        200,
        r#"{"data":[{"id":"mistral-medium"}]}"#,
        Some(1),
    );

    assert_eq!(state, "ready");
    assert!(!blocked);
    assert!(reason.contains("1 models visible"));
}

#[test]
fn provider_error_should_fallback_for_tpm_413_failures() {
    let parsed = serde_json::json!({
        "error": {
            "code": "rate_limit_exceeded",
            "message": "Request too large for model `llama-3.3-70b-versatile` on tokens per minute (TPM): Limit 12000, Requested 15695, please reduce your input."
        }
    });

    assert!(super::provider_error_should_fallback(413, &parsed));
    assert!(super::is_request_scoped_retry_error(413, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("groq", 413, &parsed),
        Some(900)
    );
}

#[test]
fn token_quota_429_is_request_scoped_with_short_cloud_cooldown() {
    let parsed = serde_json::json!({
        "code": "token_quota_exceeded",
        "message": "Tokens per minute limit exceeded - too many tokens processed.",
        "param": "quota",
        "type": "too_many_tokens"
    });

    assert!(super::provider_error_should_fallback(429, &parsed));
    assert!(super::is_request_scoped_retry_error(429, &parsed));
    assert!(!super::is_billing_or_credit_error(429, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("cerebras", 429, &parsed),
        Some(300)
    );
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("edge_core", 429, &parsed),
        None
    );
}

#[test]
fn reasoning_replay_required_error_is_request_scoped_payload_dialect_retry() {
    let parsed = serde_json::json!({
        "error": {
            "code": "invalid_request_error",
            "message": "Error from provider: The `reasoning_content` in the thinking mode must be passed back to the API.",
            "type": "invalid_request_error"
        }
    });

    assert!(super::is_reasoning_replay_required_error(400, &parsed));
    assert!(super::provider_error_should_fallback(400, &parsed));
    assert!(super::is_request_scoped_retry_error(400, &parsed));
    assert!(!super::is_client_payload_error(400, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("opencode", 400, &parsed),
        None
    );
}

#[test]
fn generic_413_falls_back_and_cools_down_cloud_provider() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Payload Too Large"
        }
    });

    assert!(super::provider_error_should_fallback(413, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("groq", 413, &parsed),
        Some(900)
    );
}

#[test]
fn transient_cloud_5xx_falls_back_and_cools_down_provider() {
    let parsed = serde_json::json!({
        "error": {
            "message": "Bad Gateway"
        }
    });

    assert!(super::provider_error_should_fallback(502, &parsed));
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("mistral", 502, &parsed),
        Some(120)
    );
    assert_eq!(
        super::provider_error_immediate_cooldown_seconds("edge_core", 502, &parsed),
        None
    );
}

#[test]
fn google_high_demand_503_marks_model_unavailable_for_in_provider_fallback() {
    let parsed = serde_json::json!([
        {
            "error": {
                "code": 503,
                "message": "This model is currently experiencing high demand. Spikes in demand are usually temporary. Please try again later.",
                "status": "UNAVAILABLE"
            }
        }
    ]);

    assert!(super::provider_error_should_fallback(503, &parsed));
    assert!(super::model_error_should_mark_unavailable(503, &parsed));
}

#[tokio::test]
async fn restricted_privacy_prefers_local_provider() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    std::env::remove_var("GROQ_API_KEY");
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"test"})],
        options: serde_json::json!({
            "privacy_tier": "restricted"
        }),
    };
    let out = svc.route(req).await.expect("route");
    assert_eq!(out.provider_id, "local_fallback");
    assert_eq!(out.execution_lane, "interactive");
    clear_provider_sources();
}

#[tokio::test]
async fn cloud_preference_can_select_cloud_provider() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    std::env::set_var("GROQ_API_KEY", "test-key");
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"test"})],
        options: serde_json::json!({
            "inference_origin": "cloud",
            "privacy_tier": "public",
            "quality_tier": "high"
        }),
    };
    let out = svc.route(req).await.expect("route");
    assert_ne!(out.provider_id, "local_fallback");
}

#[tokio::test]
async fn code_routes_expose_execution_lane_and_context_target() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    std::env::set_var("GROQ_API_KEY", "test-key");
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "apollo".to_string(),
        task_type: "code".to_string(),
        priority: "high".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"test"})],
        options: serde_json::json!({
            "tools": [{"name":"shell"}],
            "tool_choice": "auto"
        }),
    };
    let out = svc.route(req).await.expect("route");
    assert_eq!(out.execution_lane, "execution");
    assert_eq!(out.route_class, "tool_oriented");
    assert_eq!(out.context_window_target, 32_000);
    assert_ne!(out.provider_id, "edge_backbone");
    assert!(!matches!(
        out.provider_id.as_str(),
        "mistral" | "zai" | "nvidia"
    ));
}

#[tokio::test]
async fn governance_ignores_system_prompt_risk_keywords_for_routing() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    let svc = CharonService::new(dir.path()).expect("service");
    let req = CharonRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({
                "role":"system",
                "content":"Do not overwrite the .env file or delete the production database."
            }),
            serde_json::json!({
                "role":"assistant",
                "content":"I'll call the terminal tool."
            }),
            serde_json::json!({
                "role":"tool",
                "content":"/var/home/mythos"
            }),
            serde_json::json!({
                "role":"user",
                "content":"Reply with exactly OK."
            }),
        ],
        options: serde_json::json!({}),
    };
    let out = svc.route(req).await.expect("route");
    assert_eq!(out.execution_lane, "interactive");
}

#[tokio::test]
async fn status_reports_malformed_state_events() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    let cache_path = dir.path().join("runtime_build_cache.json");
    fs::write(
        &cache_path,
        serde_json::json!({
            "generated_at_utc": "2026-03-11T00:00:00Z",
            "authority": "test",
            "build_root": "/tmp/ARDA-build",
            "target_dir": "/tmp/ARDA-build/target",
            "observed_bytes": 123,
            "target_bytes": 100,
            "removed_bytes": 23,
            "status": "ok"
        })
        .to_string(),
    )
    .expect("cache write");
    std::env::set_var("ARDA_RUNTIME_BUILD_CACHE_STATE_PATH", &cache_path);
    std::env::set_var("ARDA_RUNTIME_BUILD_CACHE_AUTORUN", "0");
    let svc = CharonService::new(dir.path()).expect("service");
    fs::write(
            dir.path().join("state.jsonl"),
            "{\"ts\":\"2026-03-10T00:00:00Z\",\"event\":\"route_selected\",\"payload\":{\"provider_id\":\"local_fallback\"}}\n{\"ts\":\"2026-03-10T00:01:00Z\",\"event\":\"route_failed\",\"payload\":{}}\n{bad\n",
        )
        .expect("state write");
    let status = svc.status().await.expect("status");
    assert_eq!(status.malformed_state_events, 1);
    assert_eq!(status.recent_route_successes, 1);
    assert_eq!(status.recent_route_failures, 1);
    assert_eq!(status.recent_local_fallback_routes, 1);
    assert!(!status.llmfit_backend.is_empty());
    assert!(!status.nanoclaw_probe_state.is_empty());
    assert_eq!(status.runtime_build_cache_status, "ok");
    assert_eq!(status.runtime_build_cache_observed_bytes, 123);
    assert_eq!(status.runtime_build_cache_removed_bytes, 23);
    std::env::remove_var("ARDA_RUNTIME_BUILD_CACHE_STATE_PATH");
    std::env::remove_var("ARDA_RUNTIME_BUILD_CACHE_AUTORUN");
    clear_provider_sources();
}

#[tokio::test]
async fn observability_separates_fallback_chains_from_legacy_route_failures() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    let dir = tempdir().expect("tempdir");
    isolate_provider_sources(dir.path());
    fs::write(
        dir.path().join("state.jsonl"),
        concat!(
            "{\"ts\":\"2026-03-10T00:00:00Z\",\"event\":\"route_failed\",\"payload\":{\"agent_id\":\"openai_shim\"}}\n",
            "{\"ts\":\"2026-03-10T00:01:00Z\",\"event\":\"route_fallback_chain\",\"payload\":{\"agent_id\":\"charon_probe\",\"attempt_count\":1,\"attempts\":[{\"provider_id\":\"openai_sub\",\"model_id\":\"gpt-5.5\",\"status_code\":429,\"outcome_class\":\"rate_or_retry_error\"}]}}\n",
            "{\"ts\":\"2026-03-10T00:02:00Z\",\"event\":\"route_cooldown_bypass\",\"payload\":{\"provider_id\":\"openai_sub\"}}\n"
        ),
    )
    .expect("state write");
    let svc = CharonService::new(dir.path()).expect("service");

    let rollup = svc.route_observability_rollup().await.expect("rollup");
    let chains = rollup["recent_fallback_chains"]
        .as_array()
        .expect("fallback chains");
    let legacy = rollup["recent_legacy_route_failures"]
        .as_array()
        .expect("legacy route failures");

    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0]["attempt_count"], 1);
    assert!(chains[0].get("legacy_payload").is_none());
    assert_eq!(legacy.len(), 2);
    assert_eq!(legacy[0]["reason"], "route_cooldown_bypass");
    assert_eq!(legacy[1]["reason"], "route_failed");
    clear_provider_sources();
}

#[test]
fn append_jsonl_serializes_concurrent_state_writers() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("state.jsonl");
    let mut threads = Vec::new();
    for idx in 0..12 {
        let path = path.clone();
        threads.push(std::thread::spawn(move || {
            append_jsonl(&path, &serde_json::json!({"idx": idx})).expect("append");
        }));
    }
    for thread in threads {
        thread.join().expect("thread join");
    }
    let content = fs::read_to_string(&path).expect("read");
    let lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(lines, 12);
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line).expect("valid json");
    }
}

#[test]
fn parses_model_size_hints_from_model_ids() {
    assert_eq!(
        super::route_policy::parse_model_params_billions("qwen2.5-coder:3b"),
        Some(3.0)
    );
    assert_eq!(
        super::route_policy::parse_model_params_billions("qwen3-30b-a3b-reasoning"),
        Some(30.0)
    );
    assert_eq!(
        super::route_policy::parse_model_params_billions("gemini-2.0-flash"),
        None
    );
}

#[test]
fn configured_provider_pool_preserves_default_fallbacks() {
    let configured = vec![super::ProviderState {
        id: "edge_worker_light".to_string(),
        name: "Configured Worker".to_string(),
        base_url: Some("http://100.103.125.88:1234/v1".to_string()),
        api_key_env: None,
        access_tier: "local".to_string(),
        quality_band: "medium".to_string(),
        intelligence_refreshed_at_utc: None,
        probe_model: None,
        probe_profile: None,
        enabled: true,
        has_api_key: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        cooldown_backoff_seconds: 120,
        requests_per_minute: Some(60),
        requests_used_minute: 0,
        minute_window_started_utc: None,
        requests_per_day: None,
        requests_used_day: 0,
        day_window_started_utc: None,
        models: vec![ModelState {
            aliases: vec![],
            id: "qwen3.5-9b".to_string(),
            capable_tasks: vec!["chat".to_string()],
            context_window: 32768,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::types::ModelCapabilities::default(),
            streaming_validated: None,
        }],
        error_count: 0,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: None,
        active_connections: 0,
        last_reservation_utc: None,
        supports_tools: true,
        supports_structured_output: true,
        driver: "openai_compat".to_string(),
        hermes_bin: None,
        hermes_provider: None,
        hermes_toolsets: None,
    }];

    let merged = super::bootstrap::merge_with_default_providers(configured);
    let ids = merged
        .iter()
        .map(|provider| provider.id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"edge_worker_light"));
    assert!(ids.contains(&"groq"));
    assert!(ids.contains(&"google"));
    assert_eq!(
        ids.iter()
            .filter(|provider_id| **provider_id == "edge_worker_light")
            .count(),
        1
    );
}

#[test]
fn stale_fleet_bootstrap_does_not_override_provider_health() {
    let bootstrap = super::bootstrap::FleetBootstrapFile {
        generated_at_utc: Some("2026-03-20T00:00:00Z".to_string()),
        targets: Vec::new(),
    };
    std::env::set_var("ARDA_FLEET_BOOTSTRAP_MAX_AGE_SECONDS", "60");
    assert!(!super::bootstrap::fleet_bootstrap_is_fresh(&bootstrap));
    std::env::remove_var("ARDA_FLEET_BOOTSTRAP_MAX_AGE_SECONDS");
}

#[test]
fn attach_charon_route_metadata_stamps_response_payload() {
    let mut response = serde_json::json!({"id":"chatcmpl-test","choices":[]});
    let decision = RouteDecision {
        provider_id: "edge_backbone".to_string(),
        model_id: "MiniMax-M2.5-Q4_K_M".to_string(),
        reason: "test".to_string(),
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
        governance: RouteGovernance {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 1.0,
                attention: 1.0,
                reciprocity: 1.0,
                score: 1.0,
            },
            ..RouteGovernance::default()
        },
        route_id: "route-test".to_string(),
    };

    super::attach_charon_route_metadata(&mut response, &decision, "edge_backbone", 42);

    let route = response
        .get("_charon_route")
        .and_then(|value| value.as_object())
        .expect("route");
    assert_eq!(
        route.get("provider_id").and_then(|value| value.as_str()),
        Some("edge_backbone")
    );
    assert_eq!(
        route.get("model_id").and_then(|value| value.as_str()),
        Some("MiniMax-M2.5-Q4_K_M")
    );
    assert_eq!(
        route.get("latency_ms").and_then(|value| value.as_u64()),
        Some(42)
    );
}
