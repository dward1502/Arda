use super::super::*;

pub(crate) async fn handle(command: CharonCommands) -> anyhow::Result<()> {
    let service = CharonService::from_default_or_fallback()?;
    let default_socket_path =
        socket_path_from_env("ANNUNIMAS_CHARON_SOCKET", "data/charon/charon.sock");
    match command {
        CharonCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let socket_path = expand_home(&socket_path);
            let daemon = CharonDaemon::new(
                service.with_socket_path(&socket_path),
                CharonDaemonConfig {
                    socket_path,
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        CharonCommands::Status { json: _, compact } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || async { Ok(serde_json::to_value(service.status().await?)?) },
            )
            .await?;
            if compact {
                println!("{}", serde_json::to_string(&out)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
        CharonCommands::State => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "state",
                serde_json::json!({}),
                || async { Ok(service.state().await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Providers => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "providers",
                serde_json::json!({}),
                || async { Ok(serde_json::to_value(service.providers().await)?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::RouteAudit { limit } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "route_history",
                serde_json::json!({"limit": limit}),
                || async {
                    Ok(route_audit_summary(
                        service.route_history(limit).await,
                        "local_fallback",
                    ))
                },
            )
            .await?;
            let out = route_audit_summary_from_value(out, "ipc");
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Observability => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "observability",
                serde_json::json!({}),
                || async { Ok(service.route_observability_rollup().await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::OperatorSummary { compact } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "operator_summary",
                serde_json::json!({}),
                || async { Ok(service.operator_route_summary().await?) },
            )
            .await?;
            if compact {
                println!("{}", serde_json::to_string(&out)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
        CharonCommands::Eval { dry_run } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "eval",
                serde_json::json!({"dry_run": dry_run}),
                || async { Ok(service.charon_eval(dry_run).await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Route {
            agent_id,
            force_provider_id,
            exclude_provider_ids,
            task_type,
            prompt,
            priority,
        } => {
            let mut options = serde_json::json!({});
            if let Some(force_provider_id) = force_provider_id {
                options["force_provider_id"] = serde_json::json!(force_provider_id);
            }
            if !exclude_provider_ids.is_empty() {
                options["exclude_provider_ids"] = serde_json::json!(exclude_provider_ids);
            }
            let envelope = CharonRequestEnvelope {
                agent_id,
                task_type: task_type.clone(),
                priority,
                messages: vec![serde_json::json!({"role":"user","content":prompt})],
                options,
            };
            let out = charon_call_or_local_async(
                &default_socket_path,
                "route",
                serde_json::to_value(&envelope)?,
                || async { Ok(serde_json::to_value(service.route(envelope).await?)?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Proxy {
            agent_id,
            force_provider_id,
            exclude_provider_ids,
            task_type,
            prompt,
            priority,
            dry_run,
        } => {
            let mut options = serde_json::json!({
                "dry_run": dry_run
            });
            if let Some(force_provider_id) = force_provider_id {
                options["force_provider_id"] = serde_json::json!(force_provider_id);
            }
            if !exclude_provider_ids.is_empty() {
                options["exclude_provider_ids"] = serde_json::json!(exclude_provider_ids);
            }
            let envelope = CharonRequestEnvelope {
                agent_id,
                task_type,
                priority,
                messages: vec![serde_json::json!({"role":"user","content":prompt})],
                options,
            };
            let out = charon_call_or_local_async(
                &default_socket_path,
                "proxy",
                serde_json::to_value(&envelope)?,
                || async { Ok(service.proxy_openai(envelope).await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Cooldown {
            provider_id,
            seconds,
        } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "cooldown",
                serde_json::json!({
                    "provider_id": provider_id,
                    "seconds": seconds
                }),
                || async {
                    service
                        .mark_provider_cooldown(&provider_id, seconds)
                        .await?;
                    Ok(serde_json::json!({"ok": true}))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::ProviderResult {
            provider_id,
            ok,
            latency_ms,
            error,
        } => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "provider_result",
                serde_json::json!({
                    "provider_id": provider_id,
                    "ok": ok,
                    "latency_ms": latency_ms,
                    "error": error,
                }),
                || async {
                    service
                        .mark_provider_result(&provider_id, ok, latency_ms, error.clone())
                        .await?;
                    Ok(serde_json::json!({"ok": true}))
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::ReloadConfig => {
            let out = charon_call_or_local_async(
                &default_socket_path,
                "reload_config",
                serde_json::json!({}),
                || async { Ok(service.reload_provider_config().await?) },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        CharonCommands::Probe {
            only,
            no_tools,
            timeout_secs,
        } => {
            let providers = service.providers().await;
            let report = probe_providers(&providers, &only, no_tools, timeout_secs).await;
            if let Some(results) = report.get("results").and_then(|value| value.as_array()) {
                for result in results {
                    let Some(provider_id) = result.get("provider_id").and_then(|v| v.as_str())
                    else {
                        continue;
                    };
                    let Some(model_id) = result.get("model_id").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let Some(streaming_validated) =
                        result.get("streaming_validated").and_then(|v| v.as_bool())
                    else {
                        continue;
                    };
                    let error = result
                        .get("streaming_error")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                    let payload = serde_json::json!({
                        "provider_id": provider_id,
                        "model_id": model_id,
                        "streaming_validated": streaming_validated,
                        "error": error,
                    });
                    let service_for_update = service.clone();
                    let provider_id = provider_id.to_string();
                    let model_id = model_id.to_string();
                    let error_for_update = error.clone();
                    let _ = charon_call_or_local_async(
                        &default_socket_path,
                        "model_streaming_validation",
                        payload,
                        || async move {
                            service_for_update
                                .mark_model_streaming_validation(
                                    &provider_id,
                                    &model_id,
                                    streaming_validated,
                                    error_for_update,
                                )
                                .await?;
                            Ok(serde_json::json!({"ok": true}))
                        },
                    )
                    .await;
                }
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CharonCommands::Discover {
            only,
            timeout_secs,
            grep,
        } => {
            let providers = service.providers().await;
            let report = discover_providers(&providers, &only, timeout_secs, &grep).await;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        CharonCommands::Paths => {
            let out =
                charon_call_or_local(&default_socket_path, "paths", serde_json::json!({}), || {
                    Ok(service.paths())
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}

fn route_audit_summary_from_value(value: serde_json::Value, source: &str) -> serde_json::Value {
    if value.get("recent_routes").is_some() && value.get("route_count").is_some() {
        return value;
    }
    let routes = value
        .get("routes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    route_audit_summary_values(routes, source)
}

fn route_audit_summary(
    routes: Vec<annunimas_charon::service::RouteHistoryEntry>,
    source: &str,
) -> serde_json::Value {
    let values = routes
        .into_iter()
        .filter_map(|route| serde_json::to_value(route).ok())
        .collect::<Vec<_>>();
    route_audit_summary_values(values, source)
}

fn route_audit_summary_values(routes: Vec<serde_json::Value>, source: &str) -> serde_json::Value {
    let mut by_agent_task =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, usize>>::new();
    let mut by_provider_model =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, usize>>::new();
    for route in &routes {
        let agent_id = route
            .get("agent_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let task_type = route
            .get("task_type")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let provider_id = route
            .get("provider_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let model_id = route
            .get("model_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        *by_agent_task
            .entry(agent_id.to_string())
            .or_default()
            .entry(task_type.to_string())
            .or_default() += 1;
        *by_provider_model
            .entry(provider_id.to_string())
            .or_default()
            .entry(model_id.to_string())
            .or_default() += 1;
    }

    serde_json::json!({
        "ok": true,
        "source": source,
        "route_count": routes.len(),
        "by_agent_task": by_agent_task,
        "by_provider_model": by_provider_model,
        "recent_routes": routes,
    })
}

async fn probe_providers(
    providers: &[annunimas_charon::ProviderState],
    only: &[String],
    no_tools: bool,
    timeout_secs: u64,
) -> serde_json::Value {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut skipped = Vec::new();
    let mut probe_tasks = Vec::new();

    for provider in providers {
        if !provider.enabled {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|needle| provider.id.contains(needle)) {
            continue;
        }
        let Some(base_url) = provider.base_url.clone() else {
            skipped.push(serde_json::json!({
                "provider_id": provider.id,
                "skipped": "no base_url configured",
            }));
            continue;
        };
        let Some(model) = provider
            .models
            .iter()
            .find(|m| m.is_default)
            .or_else(|| provider.models.first())
            .cloned()
        else {
            skipped.push(serde_json::json!({
                "provider_id": provider.id,
                "skipped": "provider has no models",
            }));
            continue;
        };

        let provider_id = provider.id.clone();
        let api_key_env = provider.api_key_env.clone();
        let client = client.clone();
        let url = format!("{}/{}", base_url.trim_end_matches('/'), "chat/completions");

        let mut payload = serde_json::json!({
            "model": model.id,
            "messages": [{
                "role": "user",
                "content": "ping: respond with the single word PONG"
            }],
            "max_tokens": 16,
            "temperature": 0.0,
        });
        if !no_tools {
            payload["tools"] = serde_json::json!([{
                "type": "function",
                "function": {
                    "name": "ack",
                    "description": "Acknowledge probe receipt.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "ok": {"type": "boolean"}
                        },
                        "required": ["ok"]
                    }
                }
            }]);
            payload["tool_choice"] = serde_json::json!("auto");
        }
        let model_id_for_task = model.id.clone();

        probe_tasks.push(tokio::spawn(async move {
            let mut req = client.post(&url).json(&payload);
            if let Some(env_key) = api_key_env.as_deref() {
                if let Ok(key) = std::env::var(env_key) {
                    if !key.trim().is_empty() {
                        req = req.bearer_auth(key);
                    }
                }
            }

            let started = std::time::Instant::now();
            match req.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let latency_ms = started.elapsed().as_millis() as u64;
                    let body = response.text().await.unwrap_or_default();
                    let body_preview = body
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(240)
                        .collect::<String>();
                    let ok = (200..300).contains(&status);
                    eprintln!(
                        "  [{}] {} {} ({}ms)",
                        if ok { "OK " } else { "FAIL" },
                        provider_id,
                        status,
                        latency_ms
                    );
                    let mut result = serde_json::json!({
                        "provider_id": provider_id,
                        "model_id": model_id_for_task,
                        "url": url,
                        "status": status,
                        "ok": ok,
                        "latency_ms": latency_ms,
                        "body_preview": body_preview,
                    });
                    let streaming = probe_streaming_format(
                        &client,
                        &url,
                        api_key_env.as_deref(),
                        &model_id_for_task,
                    )
                    .await;
                    result["streaming_validated"] = serde_json::json!(streaming.ok);
                    result["streaming_chunks_validated"] =
                        serde_json::json!(streaming.chunks_validated);
                    if let Some(error) = streaming.error {
                        result["streaming_error"] = serde_json::json!(error);
                    }
                    result
                }
                Err(err) => {
                    let latency_ms = started.elapsed().as_millis() as u64;
                    eprintln!(
                        "  [FAIL] {} transport error ({}ms): {}",
                        provider_id, latency_ms, err
                    );
                    serde_json::json!({
                        "provider_id": provider_id,
                        "model_id": model_id_for_task,
                        "url": url,
                        "status": null,
                        "ok": false,
                        "latency_ms": latency_ms,
                        "transport_error": err.to_string(),
                        "streaming_validated": false,
                        "streaming_error": "plain chat probe failed before streaming probe",
                    })
                }
            }
        }));
    }

    let tested = probe_tasks.len();
    eprintln!(
        "probing {} providers in parallel (timeout={}s, tools={})",
        tested, timeout_secs, !no_tools
    );

    let mut results = skipped;
    for handle in probe_tasks {
        match handle.await {
            Ok(value) => results.push(value),
            Err(err) => results.push(serde_json::json!({
                "probe_error": err.to_string(),
            })),
        }
    }

    serde_json::json!({
        "tested": tested,
        "tools_in_payload": !no_tools,
        "timeout_secs": timeout_secs,
        "results": results,
    })
}

struct StreamingProbeResult {
    ok: bool,
    chunks_validated: usize,
    error: Option<String>,
}

async fn probe_streaming_format(
    client: &reqwest::Client,
    url: &str,
    api_key_env: Option<&str>,
    model_id: &str,
) -> StreamingProbeResult {
    let payload = serde_json::json!({
        "model": model_id,
        "messages": [{
            "role": "user",
            "content": "ping: stream five short tokens"
        }],
        "max_tokens": 16,
        "temperature": 0.0,
        "stream": true,
    });
    let mut req = client.post(url).json(&payload);
    if let Some(env_key) = api_key_env {
        if let Ok(key) = std::env::var(env_key) {
            if !key.trim().is_empty() {
                req = req.bearer_auth(key);
            }
        }
    }

    let response = match req.send().await {
        Ok(response) => response,
        Err(err) => {
            return StreamingProbeResult {
                ok: false,
                chunks_validated: 0,
                error: Some(format!("streaming probe transport error: {err}")),
            };
        }
    };
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return StreamingProbeResult {
            ok: false,
            chunks_validated: 0,
            error: Some(format!(
                "streaming probe HTTP {status}: {}",
                body.chars().take(240).collect::<String>()
            )),
        };
    }
    let text = match response.text().await {
        Ok(text) => text,
        Err(err) => {
            return StreamingProbeResult {
                ok: false,
                chunks_validated: 0,
                error: Some(format!("streaming probe body read failed: {err}")),
            };
        }
    };

    let mut chunks_validated = 0usize;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with(':') {
            chunks_validated += 1;
        } else if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" || serde_json::from_str::<serde_json::Value>(data).is_ok() {
                chunks_validated += 1;
            } else {
                return StreamingProbeResult {
                    ok: false,
                    chunks_validated,
                    error: Some(format!(
                        "streaming probe malformed data line: {}",
                        data.chars().take(160).collect::<String>()
                    )),
                };
            }
        } else {
            return StreamingProbeResult {
                ok: false,
                chunks_validated,
                error: Some(format!(
                    "streaming probe malformed SSE line: {}",
                    line.chars().take(160).collect::<String>()
                )),
            };
        }
        if chunks_validated >= 5 {
            return StreamingProbeResult {
                ok: true,
                chunks_validated,
                error: None,
            };
        }
    }

    StreamingProbeResult {
        ok: chunks_validated > 0,
        chunks_validated,
        error: if chunks_validated > 0 {
            None
        } else {
            Some("streaming probe returned no SSE chunks".to_string())
        },
    }
}

async fn discover_providers(
    providers: &[annunimas_charon::ProviderState],
    only: &[String],
    timeout_secs: u64,
    grep: &[String],
) -> serde_json::Value {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut skipped = Vec::new();
    let mut tasks = Vec::new();

    for provider in providers {
        if !provider.enabled {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|needle| provider.id.contains(needle)) {
            continue;
        }
        let Some(base_url) = provider.base_url.clone() else {
            skipped.push(serde_json::json!({
                "provider_id": provider.id,
                "skipped": "no base_url configured",
            }));
            continue;
        };

        let provider_id = provider.id.clone();
        let api_key_env = provider.api_key_env.clone();
        let client = client.clone();
        let grep_filters = grep.to_vec();
        let configured_ids: Vec<String> = provider.models.iter().map(|m| m.id.clone()).collect();

        let url = format!("{}/{}", base_url.trim_end_matches('/'), "models");

        tasks.push(tokio::spawn(async move {
            let mut req = client.get(&url);
            if let Some(env_key) = api_key_env.as_deref() {
                if let Ok(key) = std::env::var(env_key) {
                    if !key.trim().is_empty() {
                        req = req.bearer_auth(key);
                    }
                }
            }

            let started = std::time::Instant::now();
            let value = match req.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let latency_ms = started.elapsed().as_millis() as u64;
                    let body_text = response.text().await.unwrap_or_default();
                    if !(200..300).contains(&status) {
                        let preview = body_text.chars().take(240).collect::<String>();
                        eprintln!(
                            "  [FAIL] {} {} ({}ms)  {}",
                            provider_id,
                            status,
                            latency_ms,
                            preview
                        );
                        return serde_json::json!({
                            "provider_id": provider_id,
                            "url": url,
                            "status": status,
                            "ok": false,
                            "latency_ms": latency_ms,
                            "body_preview": preview,
                        });
                    }
                    let parsed: serde_json::Value =
                        serde_json::from_str(&body_text).unwrap_or_else(|_| {
                            serde_json::json!({ "raw": body_text.chars().take(500).collect::<String>() })
                        });
                    let mut model_ids = extract_model_ids(&parsed);
                    if !grep_filters.is_empty() {
                        model_ids.retain(|id| {
                            let lowered = id.to_ascii_lowercase();
                            grep_filters
                                .iter()
                                .any(|needle| lowered.contains(&needle.to_ascii_lowercase()))
                        });
                    }
                    model_ids.sort();
                    model_ids.dedup();

                    let missing_in_live: Vec<String> = configured_ids
                        .iter()
                        .filter(|cfg| !model_ids.contains(cfg))
                        .cloned()
                        .collect();
                    let not_in_config: Vec<String> = model_ids
                        .iter()
                        .filter(|live| !configured_ids.contains(live))
                        .cloned()
                        .collect();

                    eprintln!(
                        "  [OK ] {} returned {} models ({}ms)  stale_in_config={} new_live={}",
                        provider_id,
                        model_ids.len(),
                        latency_ms,
                        missing_in_live.len(),
                        not_in_config.len()
                    );
                    serde_json::json!({
                        "provider_id": provider_id,
                        "url": url,
                        "status": status,
                        "ok": true,
                        "latency_ms": latency_ms,
                        "available_models": model_ids,
                        "configured_models": configured_ids,
                        "stale_in_config": missing_in_live,
                        "not_in_config": not_in_config,
                    })
                }
                Err(err) => {
                    let latency_ms = started.elapsed().as_millis() as u64;
                    eprintln!(
                        "  [FAIL] {} transport ({}ms): {}",
                        provider_id, latency_ms, err
                    );
                    serde_json::json!({
                        "provider_id": provider_id,
                        "url": url,
                        "status": null,
                        "ok": false,
                        "latency_ms": latency_ms,
                        "transport_error": err.to_string(),
                    })
                }
            };
            value
        }));
    }

    let tested = tasks.len();
    eprintln!(
        "discovering {} provider catalogs in parallel (timeout={}s)",
        tested, timeout_secs
    );

    let mut results = skipped;
    for handle in tasks {
        match handle.await {
            Ok(value) => results.push(value),
            Err(err) => results.push(serde_json::json!({
                "discover_error": err.to_string(),
            })),
        }
    }

    serde_json::json!({
        "tested": tested,
        "timeout_secs": timeout_secs,
        "grep": grep,
        "results": results,
    })
}

/// Extract model IDs from a /models response. Supports the standard
/// OpenAI shape `{data: [{id: ...}]}`, plus a few common variants
/// (`models`, top-level array, Anthropic's `data[].display_name`).
fn extract_model_ids(value: &serde_json::Value) -> Vec<String> {
    let mut ids = Vec::new();

    let arrays: [Option<&Vec<serde_json::Value>>; 3] = [
        value.get("data").and_then(|v| v.as_array()),
        value.get("models").and_then(|v| v.as_array()),
        value.as_array(),
    ];
    for array in arrays.into_iter().flatten() {
        for item in array {
            if let Some(s) = item.as_str() {
                ids.push(s.to_string());
                continue;
            }
            for key in ["id", "name", "model", "model_id", "slug"] {
                if let Some(s) = item.get(key).and_then(|v| v.as_str()) {
                    ids.push(s.to_string());
                    break;
                }
            }
        }
        if !ids.is_empty() {
            break;
        }
    }
    ids
}
