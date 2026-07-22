use super::super::route_scoring::*;
use super::super::status::PackageRuntimeSignals;
use super::*;
use crate::adaptive::service::types::{ManweRequestEnvelope, ModelState, ProviderState};
use chrono::Utc;
use std::collections::BTreeMap;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn provider(id: &str) -> ProviderState {
    ProviderState {
        id: id.to_string(),
        name: id.to_string(),
        base_url: Some(format!("http://{}:{}/v1", "127.0.0.1", 1234)),
        api_key_env: None,
        access_tier: "mixed".to_string(),
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
        minute_window_started_utc: Some(Utc::now().to_rfc3339()),
        requests_per_day: Some(1_000),
        requests_used_day: 0,
        day_window_started_utc: Some(Utc::now().to_rfc3339()),
        error_count: 0,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(1_200),
        active_connections: 0,
        last_reservation_utc: None,
        supports_tools: true,
        supports_structured_output: true,
        driver: "openai_compat".to_string(),
        hermes_bin: None,
        hermes_provider: None,
        hermes_toolsets: None,
        models: vec![ModelState {
            aliases: vec![],
            id: "qwen2.5-coder:3b".to_string(),
            capable_tasks: vec!["chat".to_string(), "code".to_string()],
            context_window: 32_000,
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
            capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
            streaming_validated: None,
        }],
    }
}

#[test]
fn cooldown_bypass_allows_short_transient_cooldown() {
    let mut p = provider("transient");
    p.in_cooldown = true;
    p.cooldown_backoff_seconds = 30;
    p.last_error = Some("temporary upstream 502".to_string());

    assert!(provider_eligible_ignoring_cooldown(&p, "normal"));
}

#[test]
fn cooldown_bypass_blocks_opencode_billing_cooldown() {
    let mut p = provider("opencode");
    p.in_cooldown = true;
    p.cooldown_backoff_seconds = 86_400;
    p.last_error =
        Some("provider opencode HTTP 401: Insufficient balance. Manage your billing".to_string());

    assert!(!provider_cooldown_bypass_allowed(&p));
    assert!(provider_eligible_ignoring_cooldown(&p, "normal"));
}

#[test]
fn cooldown_bypass_blocks_nvidia_function_not_found_cooldown() {
    let mut p = provider("nvidia");
    p.in_cooldown = true;
    p.cooldown_backoff_seconds = 900;
    p.last_error = Some(
        "provider nvidia HTTP 404: Function '84bf12ff' Not found for account 'acct_123'"
            .to_string(),
    );

    assert!(!provider_cooldown_bypass_allowed(&p));
    assert!(provider_eligible_ignoring_cooldown(&p, "normal"));
}

#[test]
fn exclusion_helpers_accept_legacy_and_alias_keys() {
    let options = serde_json::json!({
        "excluded_provider_ids": ["openai_sub", "mistral"],
        "excluded_model_ids": ["bad-model"]
    });

    assert_eq!(
        excluded_provider_ids(&options),
        vec!["openai_sub".to_string(), "mistral".to_string()]
    );
    assert_eq!(excluded_model_ids(&options), vec!["bad-model".to_string()]);
}

#[test]
fn derive_execution_profile_respects_background_priority() {
    let req = ManweRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "chat".to_string(),
        priority: "background".to_string(),
        messages: vec![],
        options: serde_json::json!({}),
    };
    let profile = derive_route_execution_profile(&req, "background");
    assert_eq!(profile.execution_lane, "background");
    assert_eq!(profile.route_class, "background_maintenance");
    assert_eq!(profile.context_window_target, 16_000);
}

#[test]
fn derive_execution_profile_uses_health_probe_for_probe_requests() {
    let req = ManweRequestEnvelope {
        agent_id: "charon_probe".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "prefer_probe_model": true,
            "execution_lane": "interactive",
            "context_window_target": 1024
        }),
    };
    let profile = derive_route_execution_profile(&req, "normal");
    assert_eq!(profile.execution_lane, "interactive");
    assert_eq!(profile.route_class, "health_probe");
    assert_eq!(profile.context_window_target, 1024);
}

#[test]
fn route_governance_config_supports_single_bacon_method() {
    let default_chain = arda_governance::GovernanceChainConfig::default_triad();
    let config = route_governance_chain_config(
        &serde_json::json!({
            "governance_method": "single",
            "governance_philosopher": "bacon"
        }),
        &default_chain,
    );

    assert_eq!(config.chain_id, "single_bacon");
    assert_eq!(config.required_passes, Some(1));
    assert_eq!(config.lenses.len(), 1);
    assert_eq!(config.lenses[0].id, "bacon");
}

#[test]
fn route_governance_chain_method_requires_all_lenses_to_pass() {
    let default_chain = arda_governance::GovernanceChainConfig::default_triad();
    let config = route_governance_chain_config(
        &serde_json::json!({
            "governance_method": "chain",
            "governance_chain_id": "default_triad"
        }),
        &default_chain,
    );

    assert!(config.strict);
    assert_eq!(config.required_passes, Some(config.lenses.len() as u32));

    let mut task = arda_core::Task::new(
        "route request for agent=hermes task_type=chat prompt=generic route",
        "dispatch",
    );
    task.clarifications_resolved = 1;
    let result = evaluate_route_governance_chain(
        &task,
        &serde_json::json!({"governance_method": "chain"}),
        &default_chain,
    );

    assert!(!result.passed);
    assert_eq!(result.required_passes, 3);
}

#[test]
fn single_governance_method_is_boolean_pass_fail() {
    let default_chain = arda_governance::GovernanceChainConfig::default_triad();
    let task = arda_core::Task::new(
        "route request for agent=hermes task_type=chat prompt=generic route",
        "dispatch",
    );
    let result = evaluate_route_governance_chain(
        &task,
        &serde_json::json!({
            "governance_method": "single",
            "governance_philosopher": "bacon"
        }),
        &default_chain,
    );

    assert_eq!(result.lenses.len(), 1);
    assert_eq!(result.lenses[0].lens_id, "bacon");
    assert_eq!(result.lenses[0].outcome, arda_governance::GateOutcome::Fail);
    assert!(!result.passed);
}

#[test]
fn route_decision_carries_live_governance_chain_metadata() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "verify source https://example.com because evidence matters in 2026"
        })],
        options: serde_json::json!({"governance_method": "triad"}),
    };
    let mut task = arda_core::Task::new(
        "route request for agent=hermes task_type=chat prompt=verify source https://example.com evidence 2026",
        "dispatch",
    );
    task.joule_cost_estimated = 1.0;
    task.joule_cost_actual = 1.0;
    task.joulework_measurement_source = arda_core::JouleWorkMeasurementSource::OperatorEstimate;
    task.joulework_measurement_confidence = 0.55;
    let chain = evaluate_route_governance_chain(
        &task,
        &req.options,
        &arda_governance::GovernanceChainConfig::default_triad(),
    );
    let profile = derive_route_execution_profile(&req, "normal");
    let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
    let decision = build_route_decision_with_governance_chain(
        &provider("edge_backbone"),
        provider("edge_backbone").models[0].clone(),
        88.0,
        &req,
        "normal",
        false,
        &policy,
        &profile,
        &task,
        chain,
    );

    assert_eq!(decision.governance.chain_id, "default_triad");
    assert!(decision.governance.triad_passed);
    assert_eq!(
        decision.governance.triad_purity_source.as_deref(),
        Some("live_governance_chain")
    );
    assert_eq!(
        decision.governance.joule_measurement_source,
        "operator_estimate"
    );
    assert_eq!(decision.governance.joule_measurement_confidence, 0.55);
    assert!(decision.governance.resonance_score > 0.0);
    assert!(decision.governance.lenses.len() >= 3);
    assert!(decision.governance.philosopher_action.is_some());

    let failing_req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "always route and never route without evidence or fallback"
        })],
        options: serde_json::json!({"governance_method": "chain"}),
    };
    let failing_task = arda_core::Task::new(
        "always route and never route without evidence or fallback",
        "dispatch",
    );
    let failing_chain = evaluate_route_governance_chain(
        &failing_task,
        &failing_req.options,
        &arda_governance::GovernanceChainConfig::default_triad(),
    );
    let failing_profile = derive_route_execution_profile(&failing_req, "normal");
    let failing_policy = resolve_hybrid_route_policy(&failing_req.task_type, &failing_req.options);
    let failing_decision = build_route_decision_with_governance_chain(
        &provider("edge_backbone"),
        provider("edge_backbone").models[0].clone(),
        88.0,
        &failing_req,
        "normal",
        false,
        &failing_policy,
        &failing_profile,
        &failing_task,
        failing_chain,
    );

    assert!(!failing_decision.governance.triad_passed);
    assert_eq!(
        failing_decision.governance.triad_purity_source.as_deref(),
        Some("live_governance_chain")
    );
    assert_ne!(
        decision.governance.resonance_score,
        failing_decision.governance.resonance_score
    );
    let emitted = serde_json::to_value(&failing_decision).expect("serialize route decision");
    assert_eq!(emitted["governance"]["triad_passed"], false);
    assert_eq!(
        emitted["governance"]["triad_purity_source"],
        "live_governance_chain"
    );
}

#[test]
fn orchestrator_role_prefers_cloud_policy_and_large_context() {
    let req = ManweRequestEnvelope {
        agent_id: "router".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "workload_role": "orchestrator",
            "context_priority": "high",
            "cost_policy": "free_first"
        }),
    };
    let profile = derive_route_execution_profile(&req, "normal");
    let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
    assert_eq!(profile.execution_lane, "orchestrator");
    assert_eq!(profile.context_window_target, 128_000);
    assert_eq!(policy.origin_preference, "auto");
    assert_eq!(policy.cost_tier, "low");
}

#[test]
fn execution_role_uses_auto_origin_and_execution_lane() {
    let req = ManweRequestEnvelope {
        agent_id: "worker".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "workload_role": "execution",
            "context_priority": "medium"
        }),
    };
    let profile = derive_route_execution_profile(&req, "normal");
    let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
    assert_eq!(profile.execution_lane, "execution");
    assert_eq!(profile.context_window_target, 32_000);
    assert_eq!(policy.origin_preference, "auto");
}

#[test]
fn available_tool_schemas_do_not_promote_chat_to_execution_lane() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "What is the current active queue health?"
        })],
        options: serde_json::json!({
            "tools_available": true,
            "tool_schema_count": 47,
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto"
        }),
    };

    let profile = derive_route_execution_profile(&req, "normal");

    assert_ne!(profile.execution_lane, "execution");
    assert_ne!(profile.route_class, "tool_oriented");
}

#[test]
fn compression_role_uses_auto_origin_and_compression_lane() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "summary".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "compact this long context into durable state"
        })],
        options: serde_json::json!({
            "workload_role": "compression",
            "context_priority": "medium"
        }),
    };
    let profile = derive_route_execution_profile(&req, "normal");
    let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);

    assert_eq!(profile.execution_lane, "compression");
    assert_eq!(profile.route_class, "compression");
    assert_eq!(profile.context_window_target, 64_000);
    assert_eq!(policy.origin_preference, "auto");
}

#[test]
fn cost_target_aliases_map_to_cost_tiers() {
    let cheap_req = ManweRequestEnvelope {
        agent_id: "router".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({"cost_target": "cheap"}),
    };
    let premium_req = ManweRequestEnvelope {
        agent_id: "router".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({"cost_target": "premium"}),
    };

    assert_eq!(
        resolve_hybrid_route_policy(&cheap_req.task_type, &cheap_req.options).cost_tier,
        "low"
    );
    assert_eq!(
        resolve_hybrid_route_policy(&premium_req.task_type, &premium_req.options).cost_tier,
        "high"
    );
}

#[test]
fn interactive_profile_uses_message_estimate_not_oversized_default() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"hello"})],
        options: serde_json::json!({}),
    };

    let profile = derive_route_execution_profile(&req, "normal");

    assert_eq!(profile.execution_lane, "interactive");
    assert_eq!(profile.context_window_target, 16_000);
}

#[test]
fn model_supports_request_rejects_models_below_context_target() {
    let mut small = provider("edge_backbone").models.remove(0);
    small.context_window = 32_768;
    let mut large = small.clone();
    large.id = "large-context".to_string();
    large.context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "athena".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"analyze github repo"})],
        options: serde_json::json!({
            "context_window_target": 64_000
        }),
    };

    assert!(!model_supports_request("edge_backbone", &small, Some(&req)));
    assert!(model_supports_request("cloud", &large, Some(&req)));
}

#[test]
fn model_supports_request_enforces_execution_lane_context_floor() {
    let mut small = provider("edge_backbone").models.remove(0);
    small.context_window = 32_000;
    small.capabilities.tools = Some(true);
    let mut large = small.clone();
    large.id = "execution-large-context".to_string();
    large.context_window = tool_execution_min_context_window();
    let req = ManweRequestEnvelope {
        agent_id: "apollo".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"run the local task"})],
        options: serde_json::json!({
            "workload_role": "execution",
            "context_window_target": 32_000
        }),
    };

    assert_eq!(
        derive_route_execution_profile(&req, "normal").execution_lane,
        "execution"
    );
    assert!(!model_supports_request("edge_backbone", &small, Some(&req)));
    assert!(model_supports_request("cloud", &large, Some(&req)));
}

#[test]
fn model_supports_request_enforces_compression_context_floor() {
    let mut small = provider("edge_backbone").models.remove(0);
    small.context_window = 64_000;
    let mut large = small.clone();
    large.id = "compression-large-context".to_string();
    large.context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "summary".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"compress this context"})],
        options: serde_json::json!({
            "workload_role": "compression",
            "context_window_target": 64_000
        }),
    };

    assert_eq!(
        derive_route_execution_profile(&req, "normal").execution_lane,
        "compression"
    );
    assert!(!model_supports_request("edge_backbone", &small, Some(&req)));
    assert!(model_supports_request("cloud", &large, Some(&req)));
}

#[test]
fn model_supports_request_rejects_visible_reasoning_models_for_code_tools() {
    let mut thinking = provider("dynamic_catalog").models.remove(0);
    thinking.id = "provider/new-thinking-model".to_string();
    thinking.capable_tasks = vec!["chat".to_string(), "code".to_string()];
    thinking.context_window = 128_000;
    thinking.capabilities.tools = Some(true);
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"edit the file"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "apply_patch",
                    "parameters": {"type": "object"}
                }
            }],
            "tool_use_required": true
        }),
    };

    assert!(model_has_visible_reasoning_surface(&thinking));
    assert!(!model_supports_request(
        "dynamic_catalog",
        &thinking,
        Some(&req)
    ));
}

#[test]
fn model_supports_request_rejects_visible_reasoning_models_for_orchestration() {
    let mut thinking = provider("dynamic_catalog").models.remove(0);
    thinking.id = "provider/context-thinking-model".to_string();
    thinking.capable_tasks = vec!["chat".to_string(), "research".to_string()];
    thinking.context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "plan the next remediation pass"
        })],
        options: serde_json::json!({
            "workload_role": "orchestrator",
            "context_priority": "high"
        }),
    };

    assert!(model_has_visible_reasoning_surface(&thinking));
    assert!(!model_supports_request(
        "dynamic_catalog",
        &thinking,
        Some(&req)
    ));
}

#[test]
fn model_supports_request_rejects_prompt_guard_generation_models() {
    let mut guard_model = provider("groq").models.remove(0);
    guard_model.id = "meta-llama/llama-prompt-guard-2-22m".to_string();
    guard_model.capable_tasks = vec!["chat".to_string(), "summary".to_string()];
    guard_model.context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "summarize this routing policy"
        })],
        options: serde_json::json!({
            "workload_role": "orchestrator",
            "context_priority": "high"
        }),
    };

    assert!(!model_supports_request("groq", &guard_model, Some(&req)));
}

#[test]
fn model_supports_request_does_not_treat_reasoning_capability_as_visible_reasoning() {
    let mut model = provider("openai_sub").models.remove(0);
    model.id = "gpt-5.5".to_string();
    model.capable_tasks = vec![
        "code".to_string(),
        "reasoning".to_string(),
        "chat".to_string(),
    ];
    model.context_window = 1_050_000;
    model.capabilities.tools = Some(true);
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"call a tool"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {"name": "charon_probe_tool", "parameters": {"type": "object"}}
            }]
        }),
    };

    assert!(!model_has_visible_reasoning_surface(&model));
    assert!(model_supports_request("openai_sub", &model, Some(&req)));
}

#[test]
fn model_supports_request_allows_visible_reasoning_when_explicitly_requested() {
    let mut reasoning = provider("dynamic_catalog").models.remove(0);
    reasoning.id = "provider/fresh-reasoning-model".to_string();
    reasoning.capable_tasks = vec![
        "chat".to_string(),
        "code".to_string(),
        "reasoning".to_string(),
    ];
    reasoning.context_window = 128_000;
    reasoning.capabilities.tools = Some(true);
    let req = ManweRequestEnvelope {
        agent_id: "oracle".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"analyze tradeoffs"})],
        options: serde_json::json!({
            "tool_use_required": true,
            "allow_visible_reasoning": true
        }),
    };

    assert!(model_supports_request(
        "dynamic_catalog",
        &reasoning,
        Some(&req)
    ));
}

#[test]
fn select_model_skips_visible_reasoning_candidate_for_code_route() {
    let mut thinking = provider("dynamic_catalog").models.remove(0);
    thinking.id = "provider/high-context-reasoning".to_string();
    thinking.capable_tasks = vec!["code".to_string()];
    thinking.context_window = 262_000;
    thinking.is_default = true;
    thinking.capabilities.tools = Some(true);
    let mut coder = thinking.clone();
    coder.id = "provider/high-context-coder".to_string();
    coder.is_default = false;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"patch ingest.rs"})],
        options: serde_json::json!({"tool_use_required": true}),
    };

    let selected = select_model_for_request(
        "dynamic_catalog",
        &[thinking, coder],
        "code",
        None,
        Some(&req),
    )
    .expect("selected model");

    assert_eq!(selected.id, "provider/high-context-coder");
}

#[test]
fn tool_schema_bytes_count_toward_context_target() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "system",
            "content": "m".repeat(81_000)
        })],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "large_tool_schema",
                    "description": "t".repeat(52_000),
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }]
        }),
    };

    let profile = derive_route_execution_profile(&req, "normal");

    assert_eq!(profile.execution_lane, "execution");
    assert_eq!(profile.context_window_target, 64_000);
}

// Provider-level request admission is vendor-agnostic by design. Model
// eligibility is driven by catalog/config capability metadata plus request fit,
// not hardcoded provider/model family allowlists.
#[test]
fn provider_supports_request_blocks_local_fallback_for_high_context_audit() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "Do a deep audit working crate-by-crate and check service.rs plus tests."
        })],
        options: serde_json::json!({
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto",
            "tool_use_required": true,
            "workload_role": "orchestrator",
            "context_priority": "high",
            "context_window_target": 128000
        }),
    };

    assert!(!provider_supports_request(
        &provider("local_fallback"),
        &req
    ));
    assert!(provider_supports_request(&provider("opencode"), &req));
    assert!(provider_supports_request(&provider("openrouter"), &req));
}

#[test]
fn provider_capabilities_block_tool_required_continuations() {
    let mut provider = provider("text_only");
    provider.supports_tools = false;
    provider.models[0].capabilities.tools = Some(false);
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "file content"
        })],
        options: serde_json::json!({
            "tool_use_required": true
        }),
    };

    assert!(!provider_supports_request_capabilities(&provider, &req));
}

#[test]
fn provider_capabilities_allow_model_level_tool_truth() {
    let mut provider = provider("subscription_wrapper");
    provider.supports_tools = false;
    provider.models[0].capabilities.tools = Some(true);
    provider.models[0].context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "file content"
        })],
        options: serde_json::json!({
            "tool_use_required": true
        }),
    };

    assert!(provider_supports_request_capabilities(&provider, &req));
}

#[test]
fn provider_capabilities_block_implicit_code_execution_routes_without_tools_payload() {
    let mut provider = provider("text_only");
    provider.supports_tools = false;
    provider.models[0].capabilities.tools = Some(false);
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "inspect the queue and complete the next task"
        })],
        options: serde_json::json!({}),
    };

    assert!(!provider_supports_request_capabilities(&provider, &req));
}

#[test]
fn provider_supports_request_blocks_hermes_cli_for_streaming() {
    let mut provider = provider("openai_sub");
    provider.driver = "hermes_agent_cli".to_string();
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role": "user", "content": "hello"})],
        options: serde_json::json!({"stream": true}),
    };

    assert!(!provider_supports_request(&provider, &req));
}

#[test]
fn hermes_cli_driver_is_penalized_on_fast_interactive_lane() {
    let mut provider = provider("openai_sub");
    provider.driver = "hermes_agent_cli".to_string();
    provider.access_tier = "paid_cloud".to_string();
    let policy = HybridRoutePolicy {
        privacy_tier: "public".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let package_runtime = PackageRuntimeSignals {
        generated_at_utc: Utc::now().to_rfc3339(),
        llmfit_backend: "test".to_string(),
        llmfit_recommendation_count: 0,
        llmfit_local_max_params_b: None,
        llmfit_top_model_names: vec![],
        nanoclaw_binary_present: false,
        nanoclaw_runtime_ready: false,
        nanoclaw_probe_state: "not_configured".to_string(),
    };
    let lane_fitness = LaneFitnessSnapshot::default();
    let model = provider.models[0].clone();
    let interactive = RouteExecutionProfile {
        route_class: "operator_override".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: 1024,
    };
    let background = RouteExecutionProfile {
        route_class: "background".to_string(),
        execution_lane: "background".to_string(),
        context_window_target: 1024,
    };

    let interactive_score = provider_score(
        &provider,
        &model,
        "normal",
        &policy,
        &interactive,
        &package_runtime,
        &lane_fitness,
    );
    let background_score = provider_score(
        &provider,
        &model,
        "normal",
        &policy,
        &background,
        &package_runtime,
        &lane_fitness,
    );

    assert!(interactive_score + 40.0 < background_score);
}

#[test]
fn provider_supports_request_blocks_unresolved_base_url_templates() {
    let mut provider = provider("litellm_gateway");
    provider.base_url = Some("http://${LITELLM_PROXY_HOST}:${LITELLM_PROXY_PORT}/v1".to_string());
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"run tools"})],
        options: serde_json::json!({
            "stream": true,
            "tools": [{"type":"function","function":{"name":"shell","parameters":{"type":"object"}}}]
        }),
    };

    assert!(!provider_supports_request(&provider, &req));
}

#[test]
fn groq_large_tool_payload_stays_eligible_for_request_scoped_retry_handling() {
    let mut model = provider("groq").models.remove(0);
    model.id = "qwen/qwen3-32b".to_string();
    model.context_window = 131_072;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "system",
            "content": "m".repeat(81_000)
        })],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "large_tool_schema",
                    "description": "t".repeat(52_000),
                    "parameters": {"type": "object"}
                }
            }]
        }),
    };

    assert!(model_supports_request("groq", &model, Some(&req)));
}

#[test]
fn groq_small_tool_payload_stays_eligible() {
    let mut model = provider("groq").models.remove(0);
    model.id = "qwen/qwen3-32b".to_string();
    model.context_window = 131_072;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"small edit"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "shell",
                    "parameters": {"type": "object"}
                }
            }]
        }),
    };

    assert!(model_supports_request("groq", &model, Some(&req)));
}

#[test]
fn groq_compound_models_are_not_admitted_for_tool_calls_without_positive_metadata() {
    let mut model = provider("groq").models.remove(0);
    model.id = "groq/compound-mini".to_string();
    model.context_window = 131_072;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"inspect repo"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "shell",
                    "parameters": {"type": "object"}
                }
            }],
            "tool_use_required": true
        }),
    };

    assert!(!model_supports_request("groq", &model, Some(&req)));
    model.capabilities.tools = Some(true);
    assert!(model_supports_request("groq", &model, Some(&req)));
}

#[test]
fn configured_tool_incompatible_models_are_not_admitted_for_tool_calls() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var(
        "ARDA_CHARON_TOOL_INCOMPATIBLE_MODELS",
        "dynamic_catalog/fragile-tools",
    );
    let mut model = provider("dynamic_catalog").models.remove(0);
    model.id = "provider/fragile-tools-v1".to_string();
    model.capable_tasks = vec!["code".to_string()];
    model.context_window = 128_000;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"patch"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {"name": "apply_patch", "parameters": {"type": "object"}}
            }]
        }),
    };

    assert!(!model_supports_request(
        "dynamic_catalog",
        &model,
        Some(&req)
    ));
    std::env::remove_var("ARDA_CHARON_TOOL_INCOMPATIBLE_MODELS");
}

#[test]
fn cerebras_gpt_oss_is_not_admitted_for_tool_calls_without_positive_metadata() {
    let mut model = provider("cerebras").models.remove(0);
    model.id = "gpt-oss-120b".to_string();
    model.context_window = 131_072;
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"inspect repo"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "shell",
                    "parameters": {"type": "object"}
                }
            }],
            "tool_use_required": true
        }),
    };

    assert!(!model_supports_request("cerebras", &model, Some(&req)));
    model.capabilities.tools = Some(true);
    assert!(!model_supports_request("cerebras", &model, Some(&req)));
    let explicit_reasoning_req = ManweRequestEnvelope {
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "shell",
                    "parameters": {"type": "object"}
                }
            }],
            "tool_use_required": true,
            "allow_visible_reasoning": true
        }),
        ..req
    };
    assert!(model_supports_request(
        "cerebras",
        &model,
        Some(&explicit_reasoning_req)
    ));
}

#[test]
fn local_fallback_is_blocked_for_hermes_tool_routes_below_context_floor() {
    let model = ModelState {
        aliases: vec![],
        id: "Qwen3-8B-Q4_K_M".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 16_384,
        is_default: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(250),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    };
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({"role": "system", "content": "s".repeat(64_000)}),
            serde_json::json!({"role": "user", "content": "continue the tool workflow"}),
        ],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "terminal",
                    "description": "t".repeat(72_000),
                    "parameters": {"type": "object", "properties": {}}
                }
            }],
            "max_tokens": 2048
        }),
    };

    let profile = derive_route_execution_profile(&req, "normal");
    assert_eq!(profile.context_window_target, 64_000);
    assert!(!model_supports_request(
        "local_fallback",
        &model,
        Some(&req)
    ));
}

#[test]
fn declared_streaming_tool_model_survives_stale_streaming_validation() {
    let model = ModelState {
        aliases: vec![],
        id: "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 131_072,
        is_default: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(2_500),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::types::ModelCapabilities {
            tools: Some(true),
            streaming: Some(true),
            structured_output: Some(true),
            visible_reasoning: Some(false),
        },
        streaming_validated: Some(false),
    };
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({"role": "system", "content": "Hermes Agent Persona"}),
            serde_json::json!({"role": "user", "content": "continue the tool workflow"}),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"content\":\"queue_active compact task list\"}"
            }),
        ],
        options: serde_json::json!({
            "stream": true,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}
                }
            }],
            "max_tokens": 2048
        }),
    };

    assert!(model_supports_request(
        "edge_backbone_coder",
        &model,
        Some(&req)
    ));
}

#[test]
fn local_tool_routes_require_context_headroom_above_floor() {
    let model = ModelState {
        aliases: vec![],
        id: "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 65_536,
        is_default: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(2_500),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::types::ModelCapabilities {
            tools: Some(true),
            streaming: Some(true),
            structured_output: Some(true),
            visible_reasoning: Some(false),
        },
        streaming_validated: Some(true),
    };
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({"role": "user", "content": "continue the tool workflow"}),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": "{\"content\":\"queue_active compact task list\"}"
            }),
        ],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}
                }
            }],
            "max_tokens": 2048
        }),
    };

    assert!(!model_supports_request(
        "edge_backbone_coder",
        &model,
        Some(&req)
    ));
}

#[test]
fn structured_output_demotion_does_not_block_tool_routes() {
    let model = ModelState {
        aliases: vec![],
        id: "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 65_536,
        is_default: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(2_500),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::types::ModelCapabilities {
            tools: Some(true),
            streaming: Some(true),
            structured_output: Some(false),
            visible_reasoning: Some(false),
        },
        streaming_validated: Some(true),
    };
    let schema_req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role": "user", "content": "emit json"})],
        options: serde_json::json!({
            "response_format": {"type": "json_object"},
            "max_tokens": 512
        }),
    };
    let tool_req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role": "user", "content": "read a file"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}
                }
            }],
            "max_tokens": 512
        }),
    };

    assert!(!model_supports_request(
        "edge_backbone_coder",
        &model,
        Some(&schema_req)
    ));
    assert!(model_supports_request(
        "edge_backbone_coder",
        &model,
        Some(&tool_req)
    ));
}

#[test]
fn explicit_emergency_flag_allows_low_context_tool_fallback() {
    let model = ModelState {
        aliases: vec![],
        id: "Qwen3-8B-Q4_K_M".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 16_384,
        is_default: true,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(250),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::types::ModelCapabilities {
            tools: Some(true),
            ..Default::default()
        },
        streaming_validated: None,
    };
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({"role": "system", "content": "s".repeat(64_000)}),
            serde_json::json!({
                "role": "assistant",
                "content": "I need to inspect a file.",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "read_file", "arguments": "{\"path\":\"/tmp/x\"}"}
                }]
            }),
            serde_json::json!({"role": "tool", "tool_call_id": "call_1", "content": "line1\nline2"}),
            serde_json::json!({"role": "user", "content": "continue"}),
        ],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "terminal",
                    "description": "t".repeat(72_000),
                    "parameters": {"type": "object", "properties": {}}
                }
            }],
            "max_tokens": 2048,
            "allow_low_context_tool_fallback": true
        }),
    };

    let profile = derive_route_execution_profile(&req, "normal");
    assert_eq!(profile.context_window_target, 64_000);
    assert!(model_supports_request("local_fallback", &model, Some(&req)));
}

#[test]
fn agentic_tool_requests_block_models_marked_without_tool_support() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "tools": [{"type":"function","function":{"name":"read_file"}}],
            "tool_choice": "auto",
            "tool_use_required": true
        }),
    };
    let incompatible = ModelState {
        aliases: vec![],
        id: "z-ai/glm-4.5-air:free".to_string(),
        capable_tasks: vec!["code".to_string()],
        context_window: 131_072,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: None,
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::types::ModelCapabilities {
            tools: Some(false),
            ..Default::default()
        },
        streaming_validated: None,
    };
    let compatible = ModelState {
        aliases: vec![],
        id: "nvidia/nemotron-3-super-120b-a12b:free".to_string(),
        capable_tasks: vec!["code".to_string()],
        context_window: 262_000,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: None,
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    };

    assert!(!model_supports_request(
        "openrouter_free",
        &incompatible,
        Some(&req)
    ));
    assert!(model_supports_request(
        "openrouter_free",
        &compatible,
        Some(&req)
    ));
}

#[test]
fn implicit_code_execution_routes_block_models_marked_without_tool_support() {
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "work the task queue and edit files as needed"
        })],
        options: serde_json::json!({}),
    };
    let mut text_only = provider("edge_core").models.remove(0);
    text_only.context_window = 128_000;
    text_only.capabilities.tools = Some(false);
    let mut tool_unknown = text_only.clone();
    tool_unknown.id = "tool-capability-unknown".to_string();
    tool_unknown.capabilities.tools = None;

    assert!(!model_supports_request("edge_core", &text_only, Some(&req)));
    assert!(model_supports_request(
        "edge_core",
        &tool_unknown,
        Some(&req)
    ));
}

#[test]
fn audit_style_requests_use_audit_stability_route_class() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({
            "role": "user",
            "content": "Do a deep audit working crate-by-crate and check service.rs plus tests."
        })],
        options: serde_json::json!({
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto",
            "tool_use_required": true,
            "workload_role": "orchestrator",
            "context_priority": "high",
            "context_window_target": 128000
        }),
    };

    let profile = derive_route_execution_profile(&req, "normal");
    assert_eq!(profile.execution_lane, "orchestrator");
    assert_eq!(profile.route_class, "audit_stability");
}

#[test]
fn nvidia_agentic_tool_requests_use_catalog_metadata_not_model_allowlist() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto",
            "tool_use_required": true,
            "workload_role": "orchestrator",
            "context_priority": "high",
            "context_window_target": 128000
        }),
    };

    let catalog_live = ModelState {
        aliases: vec![],
        id: "meta/llama-3.3-70b-instruct".to_string(),
        capable_tasks: vec!["code".to_string()],
        context_window: 256_000,
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
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    };
    let mut incompatible = catalog_live.clone();
    incompatible.id = "nvidia/text-only-model".to_string();
    incompatible.capabilities.tools = Some(false);

    assert!(model_supports_request("nvidia", &catalog_live, Some(&req)));
    assert!(!model_supports_request("nvidia", &incompatible, Some(&req)));
}

#[test]
fn provider_model_pool_keeps_catalog_live_nvidia_models_outside_old_allowlist() {
    let mut p = provider("nvidia");
    p.models.clear();
    p.models.push(ModelState {
        aliases: vec![],
        id: "meta/llama-3.3-70b-instruct".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 128_000,
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
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    });
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![],
        options: serde_json::json!({
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto",
            "tool_use_required": true,
            "context_window_target": 128000
        }),
    };

    let candidates = candidate_models_for_provider_request(&p, "code", None, Some(&req));

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "meta/llama-3.3-70b-instruct");
}

#[test]
fn free_cloud_scores_above_paid_cloud_for_orchestrator_free_first() {
    let mut free = provider("groq");
    free.access_tier = "free_cloud".to_string();
    free.quality_band = "high".to_string();
    let mut paid = provider("openai");
    paid.access_tier = "paid_cloud".to_string();
    paid.quality_band = "high".to_string();
    let model = free.models[0].clone();
    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "cloud".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let free_score = provider_score(
        &free,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let paid_score = provider_score(
        &paid,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(free_score > paid_score);
}

#[test]
fn cheap_policy_prefers_lower_declared_model_cost() {
    let provider = provider("openrouter");
    let mut cheap_model = provider.models[0].clone();
    cheap_model.cost_per_million_tokens_in = Some(0.10);
    cheap_model.cost_per_million_tokens_out = Some(0.20);
    let mut expensive_model = cheap_model.clone();
    expensive_model.cost_per_million_tokens_in = Some(8.0);
    expensive_model.cost_per_million_tokens_out = Some(24.0);
    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "balanced".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "interactive".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: 16_000,
    };
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

    let cheap_score = provider_score(
        &provider,
        &cheap_model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let expensive_score = provider_score(
        &provider,
        &expensive_model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(cheap_score > expensive_score);
}

#[test]
fn viable_edge_backbone_beats_mixed_cloud_for_large_context_orchestrator() {
    let mut mesh = provider("edge_backbone");
    mesh.access_tier = "local".to_string();
    mesh.quality_band = "high".to_string();
    mesh.avg_latency_ms = Some(4_200);
    mesh.models[0].id = "MiniMax-M2.5-Q4_K_M".to_string();
    mesh.models[0].context_window = 200_000;

    let mut mixed = provider("openrouter");
    mixed.access_tier = "mixed".to_string();
    mixed.quality_band = "high".to_string();
    mixed.avg_latency_ms = Some(1_500);
    mixed.models[0].id = "openrouter/auto".to_string();
    mixed.models[0].context_window = 128_000;

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let mesh_score = provider_score(
        &mesh,
        &mesh.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let mixed_score = provider_score(
        &mixed,
        &mixed.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(mesh_score > mixed_score);
}

#[test]
fn overloaded_edge_backbone_still_loses_to_fast_cloud_for_orchestrator() {
    let mut mesh = provider("edge_backbone");
    mesh.access_tier = "local".to_string();
    mesh.quality_band = "high".to_string();
    mesh.avg_latency_ms = Some(48_000);
    mesh.active_connections = 5;
    mesh.models[0].id = "MiniMax-M2.5-Q4_K_M".to_string();
    mesh.models[0].context_window = 200_000;

    let mut cloud = provider("openrouter_free");
    cloud.access_tier = "free_cloud".to_string();
    cloud.quality_band = "high".to_string();
    cloud.avg_latency_ms = Some(2_200);
    cloud.models[0].id = "openrouter/free".to_string();
    cloud.models[0].context_window = 1_048_576;

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
    let profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let mesh_score = provider_score(
        &mesh,
        &mesh.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let cloud_score = provider_score(
        &cloud,
        &cloud.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(cloud_score > mesh_score);
}

#[test]
fn edge_backbone_scores_higher_under_restricted_policy() {
    let mut local = provider("edge_backbone");
    local.access_tier = "local".to_string();
    local.quality_band = "high".to_string();
    let cloud = provider("groq");
    let model = local.models[0].clone();
    let policy = HybridRoutePolicy {
        privacy_tier: "restricted".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "balanced".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: true,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "interactive".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: 32_000,
    };
    let package_runtime = PackageRuntimeSignals {
        generated_at_utc: Utc::now().to_rfc3339(),
        llmfit_backend: "test".to_string(),
        llmfit_recommendation_count: 1,
        llmfit_local_max_params_b: Some(7.0),
        llmfit_top_model_names: vec!["qwen2.5-coder:3b".to_string()],
        nanoclaw_binary_present: true,
        nanoclaw_runtime_ready: true,
        nanoclaw_probe_state: "ready".to_string(),
    };
    let lane_fitness = LaneFitnessSnapshot::default();

    let local_score = provider_score(
        &local,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let cloud_score = provider_score(
        &cloud,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(local_score > cloud_score);
}

#[test]
fn soft_lane_caps_drop_overloaded_candidates_when_alternatives_exist() {
    let providers = vec![
        ProviderState {
            // Bumped above the post-collapse execution-lane soft cap (6) for
            // edge_backbone so the test still exercises the drop path.
            active_connections: 7,
            ..provider("edge_backbone")
        },
        ProviderState {
            active_connections: 1,
            ..provider("edge_worker_light")
        },
    ];
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 32_000,
    };
    let mut candidates = vec![
        RouteSelectionCandidate {
            provider_index: 0,
            model: providers[0].models[0].clone(),
            score: 90.0,
        },
        RouteSelectionCandidate {
            provider_index: 1,
            model: providers[1].models[0].clone(),
            score: 88.0,
        },
    ];

    apply_soft_lane_caps(&mut candidates, &providers, &profile);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].provider_index, 1);
}

#[test]
fn edge_backbone_scores_above_worker_for_execution_when_mesh_is_preferred() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("ARDA_LOCAL_INFERENCE_SURFACE", "mesh");
    let mut mesh = provider("edge_backbone");
    mesh.access_tier = "local".to_string();
    mesh.quality_band = "high".to_string();
    mesh.models[0].context_window = 131_072;

    let mut worker = provider("edge_worker_light");
    worker.access_tier = "local".to_string();
    worker.quality_band = "medium".to_string();

    let model = mesh.models[0].clone();
    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "local".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 32_000,
    };
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

    let mesh_score = provider_score(
        &mesh,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let worker_score = provider_score(
        &worker,
        &worker.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    std::env::remove_var("ARDA_LOCAL_INFERENCE_SURFACE");
    assert!(mesh_score > worker_score);
}

#[test]
fn slow_mesh_coder_loses_to_fast_cloud_for_execution_lane() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("ARDA_LOCAL_INFERENCE_SURFACE", "mesh");

    let mut local = provider("edge_backbone_coder");
    local.access_tier = "local".to_string();
    local.quality_band = "high".to_string();
    local.avg_latency_ms = Some(45_000);
    local.models[0].id = "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string();
    local.models[0].context_window = 131_072;

    let mut cloud = provider("groq");
    cloud.access_tier = "free_cloud".to_string();
    cloud.quality_band = "high".to_string();
    cloud.avg_latency_ms = Some(1_500);
    cloud.models[0].id = "llama-3.3-70b-versatile".to_string();
    cloud.models[0].context_window = 131_072;

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 64_000,
    };
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
    let mut execution_lane = BTreeMap::new();
    execution_lane.insert(
        local.id.clone(),
        LaneFitnessState {
            avg_latency_ms: Some(45_000),
            success_count: 4,
            failure_count: 0,
            last_result_utc: Some(Utc::now().to_rfc3339()),
        },
    );
    execution_lane.insert(
        cloud.id.clone(),
        LaneFitnessState {
            avg_latency_ms: Some(1_500),
            success_count: 4,
            failure_count: 0,
            last_result_utc: Some(Utc::now().to_rfc3339()),
        },
    );
    let lane_fitness = LaneFitnessSnapshot {
        generated_at_utc: Utc::now().to_rfc3339(),
        lanes: BTreeMap::from([("execution".to_string(), execution_lane)]),
    };

    let local_score = provider_score(
        &local,
        &local.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let cloud_score = provider_score(
        &cloud,
        &cloud.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    std::env::remove_var("ARDA_LOCAL_INFERENCE_SURFACE");
    assert!(
        cloud_score > local_score,
        "execution routing should react to observed latency instead of overvaluing slow local coder"
    );
}

#[test]
fn local_device_pressure_penalizes_local_execution_when_origin_is_auto() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::remove_var("ARDA_LOCAL_DEVICE_PRESSURE");
    std::env::remove_var("ARDA_CHARON_LOCAL_DEVICE_PRESSURE");

    let mut local = provider("edge_backbone_coder");
    local.access_tier = "local".to_string();
    local.quality_band = "high".to_string();
    local.models[0].context_window = 65_536;

    let model = local.models[0].clone();
    let policy = HybridRoutePolicy {
        privacy_tier: "public".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 32_000,
    };
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

    let baseline = provider_score(
        &local,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    std::env::set_var("ARDA_CHARON_LOCAL_DEVICE_PRESSURE", "0.95");
    let pressured = provider_score(
        &local,
        &model,
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    std::env::remove_var("ARDA_CHARON_LOCAL_DEVICE_PRESSURE");

    assert!(
        pressured <= baseline - 80.0,
        "expected high local pressure to materially demote local routes; baseline={baseline} pressured={pressured}"
    );
}

#[test]
fn slow_execution_model_loses_even_when_provider_average_is_fast() {
    let mut slow_model_provider = provider("openrouter");
    slow_model_provider.access_tier = "mixed".to_string();
    slow_model_provider.quality_band = "high".to_string();
    slow_model_provider.avg_latency_ms = Some(1_500);
    slow_model_provider.models[0].id = "provider/large-tool-model-500b".to_string();
    slow_model_provider.models[0].context_window = 131_072;
    slow_model_provider.models[0].avg_latency_ms = Some(36_000);

    let mut fast_provider = provider("groq");
    fast_provider.access_tier = "free_cloud".to_string();
    fast_provider.quality_band = "high".to_string();
    fast_provider.avg_latency_ms = Some(1_800);
    fast_provider.models[0].id = "provider/responsive-tool-model-32b".to_string();
    fast_provider.models[0].context_window = 131_072;
    fast_provider.models[0].avg_latency_ms = Some(1_800);

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 64_000,
    };
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
    let lane_fitness = LaneFitnessSnapshot {
        generated_at_utc: Utc::now().to_rfc3339(),
        lanes: BTreeMap::new(),
    };

    let slow_score = provider_score(
        &slow_model_provider,
        &slow_model_provider.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let fast_score = provider_score(
        &fast_provider,
        &fast_provider.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(
        fast_score > slow_score,
        "execution scoring should honor model-level latency, not only provider averages"
    );
}

#[test]
fn responsive_mesh_coder_remains_preferred_for_execution_lane() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("ARDA_LOCAL_INFERENCE_SURFACE", "mesh");

    let mut local = provider("edge_backbone_coder");
    local.access_tier = "local".to_string();
    local.quality_band = "high".to_string();
    local.avg_latency_ms = Some(900);
    local.models[0].id = "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string();
    local.models[0].context_window = 131_072;

    let mut cloud = provider("groq");
    cloud.access_tier = "free_cloud".to_string();
    cloud.quality_band = "high".to_string();
    cloud.avg_latency_ms = Some(1_500);
    cloud.models[0].id = "llama-3.3-70b-versatile".to_string();
    cloud.models[0].context_window = 131_072;

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "balanced".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "tool_oriented".to_string(),
        execution_lane: "execution".to_string(),
        context_window_target: 64_000,
    };
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
    let mut execution_lane = BTreeMap::new();
    execution_lane.insert(
        local.id.clone(),
        LaneFitnessState {
            avg_latency_ms: Some(900),
            success_count: 4,
            failure_count: 0,
            last_result_utc: Some(Utc::now().to_rfc3339()),
        },
    );
    execution_lane.insert(
        cloud.id.clone(),
        LaneFitnessState {
            avg_latency_ms: Some(1_500),
            success_count: 4,
            failure_count: 0,
            last_result_utc: Some(Utc::now().to_rfc3339()),
        },
    );
    let lane_fitness = LaneFitnessSnapshot {
        generated_at_utc: Utc::now().to_rfc3339(),
        lanes: BTreeMap::from([("execution".to_string(), execution_lane)]),
    };

    let local_score = provider_score(
        &local,
        &local.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let cloud_score = provider_score(
        &cloud,
        &cloud.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    std::env::remove_var("ARDA_LOCAL_INFERENCE_SURFACE");
    assert!(
        local_score > cloud_score,
        "local coder should remain preferred when it is actually responsive"
    );
}

#[test]
fn orchestrator_prefers_backbone_when_origin_is_local_and_cost_is_low() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("ARDA_LOCAL_INFERENCE_SURFACE", "mesh");

    let mut backbone = provider("edge_backbone");
    backbone.access_tier = "local".to_string();
    backbone.quality_band = "high".to_string();
    backbone.models[0].context_window = 131_072;

    let mut free_cloud = provider("openrouter_free");
    free_cloud.access_tier = "free_cloud".to_string();
    free_cloud.quality_band = "high".to_string();
    free_cloud.models[0].context_window = 262_144;

    let policy = HybridRoutePolicy {
        privacy_tier: "internal".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "high".to_string(),
        origin_preference: "local".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let backbone_score = provider_score(
        &backbone,
        &backbone.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let free_cloud_score = provider_score(
        &free_cloud,
        &free_cloud.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    std::env::remove_var("ARDA_LOCAL_INFERENCE_SURFACE");
    assert!(backbone_score > free_cloud_score);
}

#[test]
fn orchestrator_free_cloud_pool_prefers_fast_healthy_metadata_over_provider_name() {
    let mut opencode = provider("opencode");
    opencode.access_tier = "free_cloud".to_string();
    opencode.quality_band = "high".to_string();
    opencode.avg_latency_ms = Some(28_000);
    opencode.consecutive_failures = 2;
    opencode.models[0].id = "deepseek-v4-flash-free".to_string();
    opencode.models[0].context_window = 128_000;

    let mut groq = provider("groq");
    groq.access_tier = "free_cloud".to_string();
    groq.quality_band = "high".to_string();
    groq.avg_latency_ms = Some(1_200);
    groq.consecutive_successes = 4;
    groq.probe_model = Some("llama-3.1-8b-instant".to_string());
    groq.models[0].id = "llama-3.1-8b-instant-free".to_string();
    groq.models[0].context_window = 128_000;

    let mut openrouter = provider("openrouter");
    openrouter.access_tier = "free_cloud".to_string();
    openrouter.quality_band = "high".to_string();
    openrouter.avg_latency_ms = Some(4_500);
    openrouter.models[0].id = "qwen/qwen3-coder:free".to_string();
    openrouter.models[0].context_window = 200_000;

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
    let profile = RouteExecutionProfile {
        route_class: "context_heavy".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let opencode_score = provider_score(
        &opencode,
        &opencode.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let groq_score = provider_score(
        &groq,
        &groq.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let openrouter_score = provider_score(
        &openrouter,
        &openrouter.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(
        groq_score > opencode_score,
        "free pool should favor current health and latency over provider id"
    );
    assert!(
        openrouter_score > opencode_score,
        "OpenRouter stays competitive when metadata is valid"
    );
}

#[test]
fn multi_turn_chat_uses_estimated_context_without_forcing_128k() {
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![
            serde_json::json!({"role": "system", "content": "execute the task"}),
            serde_json::json!({"role": "user", "content": "continue"}),
            serde_json::json!({"role": "assistant", "content": "working"}),
            serde_json::json!({"role": "user", "content": "continue"}),
            serde_json::json!({"role": "assistant", "content": "reading files"}),
            serde_json::json!({"role": "user", "content": "finish"}),
        ],
        options: serde_json::json!({}),
    };

    let profile = derive_route_execution_profile(&req, "normal");

    assert_eq!(profile.route_class, "context_heavy");
    assert_eq!(profile.execution_lane, "planning");
    assert_eq!(profile.context_window_target, 64_000);
}

#[test]
fn openrouter_free_models_are_not_agentic_tool_candidates_by_default() {
    let req = ManweRequestEnvelope {
        agent_id: "openai_shim".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role": "user", "content": "read a file"})],
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object"}
                }
            }]
        }),
    };
    let mut free_model = provider("openrouter").models.remove(0);
    free_model.id = "nvidia/nemotron-3-ultra-550b-a55b:free".to_string();
    free_model.context_window = 262_144;

    assert!(!model_supports_request(
        "openrouter",
        &free_model,
        Some(&req)
    ));

    free_model.capabilities.tools = Some(true);
    assert!(!model_supports_request(
        "openrouter",
        &free_model,
        Some(&req)
    ));

    let explicit_free_req = ManweRequestEnvelope {
        options: serde_json::json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object"}
                }
            }],
            "tool_pool_strategy": "free_first"
        }),
        ..req
    };
    assert!(model_supports_request(
        "openrouter",
        &free_model,
        Some(&explicit_free_req)
    ));
}

#[test]
fn named_free_cloud_pool_providers_remain_eligible_when_metadata_is_valid() {
    for provider_id in [
        "nvidia",
        "openrouter",
        "groq",
        "cerebras",
        "google",
        "opencode",
    ] {
        let mut candidate = provider(provider_id);
        candidate.access_tier = "free_cloud".to_string();
        candidate.quality_band = "high".to_string();
        candidate.probe_model = Some(candidate.models[0].id.clone());
        candidate.models[0].id = format!("{provider_id}/healthy-free");
        candidate.models[0].context_window = 128_000;
        candidate.cooldown_until_utc = None;
        candidate.in_cooldown = false;

        assert!(
            provider_eligible(&candidate, "normal", false),
            "{provider_id} should stay in the volatile free-provider pool when its freeze cooldowns are clear",
        );
        assert!(
            provider_supports_request(
                &candidate,
                &ManweRequestEnvelope {
                    agent_id: "router".to_string(),
                    task_type: "chat".to_string(),
                    priority: "normal".to_string(),
                    messages: vec![],
                    options: serde_json::json!({
                        "workload_role": "orchestrator",
                        "context_priority": "high",
                        "cost_policy": "free_first"
                    }),
                }
            ),
            "{provider_id} should not need hardcoded provider-id admission"
        );
    }
}

#[test]
fn audit_stability_uses_free_cloud_health_instead_of_openrouter_free_penalty() {
    let mut openrouter = provider("openrouter");
    openrouter.access_tier = "free_cloud".to_string();
    openrouter.quality_band = "high".to_string();
    openrouter.avg_latency_ms = Some(1_800);
    openrouter.consecutive_successes = 5;
    openrouter.probe_model = Some("qwen/qwen3-coder:free".to_string());
    openrouter.models[0].id = "qwen/qwen3-coder:free".to_string();
    openrouter.models[0].context_window = 200_000;

    let mut unhealthy_free = provider("opencode");
    unhealthy_free.access_tier = "free_cloud".to_string();
    unhealthy_free.quality_band = "high".to_string();
    unhealthy_free.avg_latency_ms = Some(32_000);
    unhealthy_free.consecutive_failures = 3;
    unhealthy_free.models[0].id = "deepseek-v4-flash-free".to_string();
    unhealthy_free.models[0].context_window = 128_000;

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
    let profile = RouteExecutionProfile {
        route_class: "audit_stability".to_string(),
        execution_lane: "orchestrator".to_string(),
        context_window_target: 128_000,
    };
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

    let openrouter_score = provider_score(
        &openrouter,
        &openrouter.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let unhealthy_score = provider_score(
        &unhealthy_free,
        &unhealthy_free.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(
        openrouter_score > unhealthy_score,
        "audit stability should not carry the old openrouter_free name penalty"
    );
}

#[test]
fn select_model_skips_unhealthy_high_context_candidate() {
    let models = vec![
        ModelState {
            aliases: vec![],
            id: "MiniMax-M2.5-Q4_K_M".to_string(),
            capable_tasks: vec!["chat".to_string(), "research".to_string()],
            context_window: 200_000,
            is_default: false,
            healthy: false,
            in_cooldown: true,
            cooldown_until_utc: Some(Utc::now().to_rfc3339()),
            consecutive_failures: 3,
            consecutive_successes: 0,
            last_error: Some("not currently available".to_string()),
            avg_latency_ms: Some(20_000),
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
            streaming_validated: None,
        },
        ModelState {
            aliases: vec![],
            id: "Qwen3-8B-Q4_K_M".to_string(),
            capable_tasks: vec!["chat".to_string(), "research".to_string()],
            context_window: 131_072,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 1,
            last_error: None,
            avg_latency_ms: Some(1_500),
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
            streaming_validated: None,
        },
    ];

    let selected = select_model(&models, "research", None).expect("selected model");
    assert_eq!(selected.id, "Qwen3-8B-Q4_K_M");
}

#[test]
fn select_model_honors_request_scoped_model_exclusions() {
    let models = vec![
        ModelState {
            aliases: vec!["primary-alias".to_string()],
            id: "provider/primary".to_string(),
            capable_tasks: vec!["chat".to_string(), "code".to_string()],
            context_window: 128_000,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 1,
            last_error: None,
            avg_latency_ms: Some(900),
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
            streaming_validated: None,
        },
        ModelState {
            aliases: vec![],
            id: "provider/alternate".to_string(),
            capable_tasks: vec!["chat".to_string(), "code".to_string()],
            context_window: 64_000,
            is_default: false,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: Some(1_100),
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
            streaming_validated: None,
        },
    ];
    let req = ManweRequestEnvelope {
        agent_id: "test".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"hello"})],
        options: serde_json::json!({"exclude_model_ids": ["primary-alias"]}),
    };

    let selected =
        select_model_for_request("provider", &models, "chat", None, Some(&req)).expect("selected");
    assert_eq!(selected.id, "provider/alternate");
}

#[test]
fn prefer_probe_model_selects_provider_probe_model_before_default() {
    let mut p = provider("openrouter");
    p.models[0].id = "provider/reasoning-ultra-120b".to_string();
    p.models[0].is_default = true;
    p.models.push(ModelState {
        aliases: vec![],
        id: "provider/nano-probe-free".to_string(),
        capable_tasks: vec!["chat".to_string()],
        context_window: 8_192,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(250),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    });
    p.probe_model = Some("provider/nano-probe-free".to_string());
    p.probe_profile = Some("low_latency_terse".to_string());
    let req = ManweRequestEnvelope {
        agent_id: "charon_probe".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"Health probe."})],
        options: serde_json::json!({"prefer_probe_model": true, "context_window_target": 1024}),
    };

    let selected =
        select_model_for_provider_request(&p, "chat", None, Some(&req)).expect("selected");

    assert_eq!(selected.id, "provider/nano-probe-free");
}

#[test]
fn provider_model_pool_keeps_alternate_after_dead_nvidia_model_excluded() {
    let mut p = provider("nvidia");
    p.models[0].id = "nvidia/llama-3.1-nemotron-ultra-253b-v1".to_string();
    p.models[0].capable_tasks = vec!["code".to_string(), "chat".to_string()];
    p.models[0].context_window = 131_072;
    p.models.push(ModelState {
        aliases: vec![],
        id: "qwen/qwen3-coder-480b-a35b-instruct".to_string(),
        capable_tasks: vec!["code".to_string(), "chat".to_string()],
        context_window: 262_144,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(1_100),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    });
    let req = ManweRequestEnvelope {
        agent_id: "hermes".to_string(),
        task_type: "code".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"continue"})],
        options: serde_json::json!({
            "exclude_model_ids": ["nvidia/llama-3.1-nemotron-ultra-253b-v1"],
            "tools": [{"type":"function","function":{"name":"terminal"}}],
            "tool_choice": "auto",
            "context_window_target": 128000
        }),
    };

    let candidates = candidate_models_for_provider_request(&p, "code", None, Some(&req));

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "qwen/qwen3-coder-480b-a35b-instruct");
}

#[test]
fn provider_model_pool_includes_probe_model_and_default_for_fallback() {
    let mut p = provider("openrouter");
    p.models[0].id = "openrouter/auto".to_string();
    p.models[0].context_window = 131_072;
    p.models.push(ModelState {
        aliases: vec![],
        id: "openrouter/nano-probe-free".to_string(),
        capable_tasks: vec!["chat".to_string()],
        context_window: 8_192,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(250),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    });
    p.probe_model = Some("openrouter/nano-probe-free".to_string());
    let req = ManweRequestEnvelope {
        agent_id: "charon_probe".to_string(),
        task_type: "chat".to_string(),
        priority: "normal".to_string(),
        messages: vec![serde_json::json!({"role":"user","content":"Health probe."})],
        options: serde_json::json!({"prefer_probe_model": true, "context_window_target": 1024}),
    };

    let candidates = candidate_models_for_provider_request(&p, "chat", None, Some(&req));

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "openrouter/nano-probe-free");
    assert!(candidates.iter().any(|model| model.id == "openrouter/auto"));
}

#[test]
fn health_probe_scoring_prefers_probe_model_over_large_production_model() {
    let mut p = provider("openrouter");
    p.models[0].id = "provider/reasoning-ultra-120b".to_string();
    p.models[0].is_default = true;
    p.models[0].avg_latency_ms = Some(4_500);
    p.models.push(ModelState {
        aliases: vec![],
        id: "provider/nano-probe-free".to_string(),
        capable_tasks: vec!["chat".to_string()],
        context_window: 8_192,
        is_default: false,
        healthy: true,
        in_cooldown: false,
        cooldown_until_utc: None,
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_error: None,
        avg_latency_ms: Some(250),
        cost_per_million_tokens_in: None,
        cost_per_million_tokens_out: None,
        capabilities: crate::adaptive::service::types::ModelCapabilities::default(),
        streaming_validated: None,
    });
    p.probe_model = Some("provider/nano-probe-free".to_string());
    p.probe_profile = Some("low_latency_terse".to_string());
    let policy = HybridRoutePolicy {
        privacy_tier: "public".to_string(),
        cost_tier: "low".to_string(),
        quality_tier: "low".to_string(),
        origin_preference: "auto".to_string(),
        latency_sla_ms: None,
        require_local: false,
        spread_score_band: 0.05,
        spread_top_cap: 4,
    };
    let profile = RouteExecutionProfile {
        route_class: "health_probe".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: 1024,
    };
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

    let production_score = provider_score(
        &p,
        &p.models[0],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );
    let probe_score = provider_score(
        &p,
        &p.models[1],
        "normal",
        &policy,
        &profile,
        &package_runtime,
        &lane_fitness,
    );

    assert!(probe_score > production_score);
}

#[test]
fn half_open_probe_gate_allows_only_probe_roll() {
    let mut p = provider("recovering");
    p.consecutive_failures = 3;
    p.consecutive_successes = 0;
    p.in_cooldown = false;
    p.cooldown_until_utc = None;

    assert!(provider_in_half_open(&p));
    assert!(provider_half_open_probe_allowed_for_roll(&p, 0));
    assert!(!provider_half_open_probe_allowed_for_roll(&p, 1));
}
