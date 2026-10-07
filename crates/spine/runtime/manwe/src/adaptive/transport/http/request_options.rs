//! OpenAI request metadata normalization; routing policy remains in the service.
use serde_json::Value;

pub(super) fn collect(body: &Value) -> Value {
    let mut options = serde_json::json!({
        "endpoint": "/chat/completions"
    });

    for key in [
        "max_tokens",
        "temperature",
        "top_p",
        "stream",
        "response_format",
        "tools",
        "tool_choice",
        "stop",
        "exclude_provider_ids",
        "excluded_provider_ids",
        "exclude_model_ids",
        "excluded_model_ids",
        "prefer_probe_model",
        "dry_run",
        "source_surface",
        "harness",
        "session_id",
        "conversation_id",
        "turn_id",
        "trace_id",
        "receipt_id",
        "skill",
        "skills",
        "toolset",
        "toolsets",
        "tool_mode",
        "tool_pool_strategy",
        "agent_mode",
        "allow_visible_reasoning",
        "allow_thinking_models",
        "require_reasoning_model",
    ] {
        if let Some(value) = body.get(key) {
            options[key] = value.clone();
        }
    }

    for key in [
        "workload_role",
        "context_priority",
        "quality_priority",
        "quality_tier",
        "cost_policy",
        "cost_tier",
        "privacy_requirement",
        "privacy_tier",
        "local_only",
        "inference_origin",
        "origin_preference",
        "context_window_target",
        "execution_lane",
        "force_provider_id",
        "force_model_id",
        "allow_forced_provider_fallback",
        "exclude_provider_ids",
        "excluded_provider_ids",
        "exclude_model_ids",
        "excluded_model_ids",
        "dry_run",
        "tool_use_required",
        "source_surface",
        "harness",
        "skill",
        "skills",
        "toolset",
        "toolsets",
        "tool_mode",
        "agent_mode",
        "allow_visible_reasoning",
        "allow_thinking_models",
        "require_reasoning_model",
    ] {
        if let Some(value) = body.get(key) {
            options[key] = value.clone();
        }
    }

    if let Some(routing) = body.get("routing").and_then(Value::as_object) {
        for key in [
            "workload_role",
            "context_priority",
            "quality_priority",
            "quality_tier",
            "cost_policy",
            "cost_tier",
            "privacy_requirement",
            "privacy_tier",
            "local_only",
            "inference_origin",
            "origin_preference",
            "context_window_target",
            "execution_lane",
            "force_provider_id",
            "force_model_id",
            "allow_forced_provider_fallback",
            "exclude_provider_ids",
            "excluded_provider_ids",
            "exclude_model_ids",
            "excluded_model_ids",
            "dry_run",
            "tool_use_required",
            "source_surface",
            "harness",
            "skill",
            "skills",
            "toolset",
            "toolsets",
            "tool_mode",
            "tool_pool_strategy",
            "agent_mode",
            "allow_visible_reasoning",
            "allow_thinking_models",
            "require_reasoning_model",
        ] {
            if let Some(value) = routing.get(key) {
                options[key] = value.clone();
            }
        }
    }

    if let Some(extra_body) = body.get("extra_body").and_then(Value::as_object) {
        for key in [
            "workload_role",
            "context_priority",
            "quality_priority",
            "quality_tier",
            "cost_policy",
            "cost_tier",
            "privacy_requirement",
            "privacy_tier",
            "local_only",
            "inference_origin",
            "origin_preference",
            "context_window_target",
            "execution_lane",
            "force_provider_id",
            "force_model_id",
            "allow_forced_provider_fallback",
            "dry_run",
            "tool_use_required",
            "source_surface",
            "harness",
            "session_id",
            "conversation_id",
            "turn_id",
            "trace_id",
            "receipt_id",
            "skill",
            "skills",
            "toolset",
            "toolsets",
            "tool_mode",
            "tool_pool_strategy",
            "agent_mode",
            "allow_visible_reasoning",
            "allow_thinking_models",
            "require_reasoning_model",
        ] {
            if let Some(value) = extra_body.get(key) {
                options[key] = value.clone();
            }
        }

        if let Some(routing) = extra_body.get("routing").and_then(Value::as_object) {
            for key in [
                "workload_role",
                "context_priority",
                "quality_priority",
                "quality_tier",
                "cost_policy",
                "cost_tier",
                "privacy_requirement",
                "privacy_tier",
                "local_only",
                "inference_origin",
                "origin_preference",
                "context_window_target",
                "execution_lane",
                "force_provider_id",
                "force_model_id",
                "allow_forced_provider_fallback",
                "exclude_provider_ids",
                "excluded_provider_ids",
                "exclude_model_ids",
                "excluded_model_ids",
                "dry_run",
                "tool_use_required",
                "source_surface",
                "harness",
                "skill",
                "skills",
                "toolset",
                "toolsets",
                "tool_mode",
                "tool_pool_strategy",
                "agent_mode",
                "allow_visible_reasoning",
                "allow_thinking_models",
                "require_reasoning_model",
                "governance_method",
                "philosopher_method",
                "governance_philosopher",
                "philosopher_lens",
                "philosopher",
                "governance_chain_id",
                "chain_id",
            ] {
                if let Some(value) = routing.get(key) {
                    options[key] = value.clone();
                }
            }
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::collect;
    use serde_json::json;

    #[test]
    fn privacy_survives_every_supported_body_location() {
        for local_only in [true, false] {
            let policy = json!({"local_only":local_only,"privacy_tier":"confidential"});
            for body in [
                policy.clone(),
                json!({"routing":policy.clone()}),
                json!({"extra_body":policy.clone()}),
                json!({"extra_body":{"routing":policy.clone()}}),
            ] {
                let options = collect(&body);
                assert_eq!(options["local_only"], local_only);
                assert_eq!(options["privacy_tier"], "confidential");
            }
        }
    }

    #[test]
    fn metadata_precedence_remains_body_then_routing_then_extra_body() {
        let options = collect(&json!({
            "force_provider_id":"body", "local_only":false,
            "routing":{"force_provider_id":"routing"},
            "extra_body":{"force_provider_id":"extra", "local_only":true,
                "routing":{"force_provider_id":"nested", "privacy_requirement":"restricted"}}
        }));
        assert_eq!(options["force_provider_id"], "nested");
        assert_eq!(options["local_only"], true);
        assert_eq!(options["privacy_requirement"], "restricted");
    }

    #[test]
    fn absent_privacy_is_not_replaced_with_a_weaker_default() {
        let options = collect(&json!({"model":"auto","messages":[]}));
        assert!(options.get("local_only").is_none());
        assert!(options.get("privacy_tier").is_none());
        assert!(options.get("privacy_requirement").is_none());
    }
}
