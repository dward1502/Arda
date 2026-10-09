// sigil: REPAIR
use super::*;
use arda_governance::{
    GovernanceScoreFuture, GovernanceScoreRequest, GovernanceScorer, GovernanceScorerError,
    GovernanceScorerErrorKind, GovernanceScorerState,
};
use std::sync::Arc;

const PROVIDER_ENV_KEYS: [&str; 4] = [
    "ARDA_PROVIDER_INTELLIGENCE_PATH",
    "ARDA_MANWE_PROVIDER_CONFIG",
    "ANNUNIMAS_CHARON_PROVIDER_CONFIG",
    "ARDA_FLEET_BOOTSTRAP_STATE",
];
struct ProviderEnvironment(Vec<(&'static str, Option<std::ffi::OsString>)>);
impl ProviderEnvironment {
    fn isolate(dir: &std::path::Path) -> Self {
        let guard = Self(
            PROVIDER_ENV_KEYS
                .iter()
                .map(|&key| (key, std::env::var_os(key)))
                .collect(),
        );
        isolate_provider_sources(dir);
        guard
    }
}
impl Drop for ProviderEnvironment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn bounded<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(std::time::Duration::from_secs(15), future)
        .await
        .expect("fixture operation deadline exceeded")
}

#[test]
fn provider_environment_restores_after_panic() {
    let _lock = isolate_env_lock(&ENV_LOCK);
    let before: Vec<_> = PROVIDER_ENV_KEYS.iter().map(std::env::var_os).collect();
    let dir = tempdir().unwrap();
    let result = std::panic::catch_unwind(|| {
        let _environment = ProviderEnvironment::isolate(dir.path());
        panic!("intentional fixture failure");
    });
    assert!(result.is_err());
    let after: Vec<_> = PROVIDER_ENV_KEYS.iter().map(std::env::var_os).collect();
    assert!(before == after, "provider environment was not restored");
}

// A real loopback provider observes dispatch; no external inference is used.
#[tokio::test]
async fn proxy_scorer_outages_preserve_advisory_results_and_provider_exclusion() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let _guard = isolate_env_lock(&ENV_LOCK);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let observed = hits.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        tokio::pin!(stopped);
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                connection = listener.accept() => {
                    let (mut stream, _) = connection.unwrap();
                    observed.fetch_add(1, Ordering::SeqCst);
                    let mut bytes = Vec::new();
                    let request = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                        loop {
                            let mut chunk = [0; 4096];
                            let count = stream.read(&mut chunk).await.unwrap();
                            assert!(count > 0, "request ended early");
                            bytes.extend_from_slice(&chunk[..count]);
                            assert!(bytes.len() < 65536, "oversized fixture request");
                            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                                let header = String::from_utf8_lossy(&bytes[..end]);
                                let length: usize = header.lines().find_map(|line| {
                                    let (key, value) = line.split_once(':')?;
                                    key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
                                }).expect("content length");
                                if bytes.len() >= end + 4 + length {
                                    break serde_json::from_slice::<serde_json::Value>(&bytes[end+4..end+4+length]).unwrap();
                                }
                            }
                        }
                    }).await.unwrap();
                    let streaming = request["stream"] == true;
                    let (kind, body) = if streaming {
                        ("text/event-stream", "data: {\"choices\":[{\"delta\":{\"content\":\"Local fixture response.\"}}]}\n\ndata: [DONE]\n\n")
                    } else {
                        ("application/json", r#"{"choices":[{"message":{"role":"assistant","content":"Local fixture response."},"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":4,"total_tokens":8}}"#)
                    };
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                    bounded(stream.write_all(response.as_bytes())).await.unwrap();
                }
            }
        }
    });
    let _server_cleanup = AbortOnDrop(server.abort_handle());
    for state in [
        GovernanceScorerState::Unavailable,
        GovernanceScorerState::Timeout,
        GovernanceScorerState::Error,
    ] {
        let dir = tempdir().unwrap();
        let _environment = ProviderEnvironment::isolate(dir.path());
        let mut svc = CharonService::new(dir.path()).unwrap();
        svc.realm_scorer = Arc::new(FaultScorer(state));
        {
            let mut providers = svc.providers.write().await;
            providers.retain(|p| p.id == "local_fallback");
            assert_eq!(providers.len(), 1);
            let p = &mut providers[0];
            p.base_url = Some(format!("http://{address}/v1"));
            p.api_key_env = None;
            p.enabled = true;
            p.healthy = true;
            p.has_api_key = true;
            p.probe_profile = None;
            p.in_cooldown = false;
            p.models[0].context_window = 65536;
        }
        let mut req = ManweRequestEnvelope {
            agent_id: "r08-proxy-fixture".into(),
            task_type: "chat".into(),
            priority: "normal".into(),
            messages: vec![
                serde_json::json!({"role":"user","content":"Return a bounded local note."}),
            ],
            options: serde_json::json!({"force_provider_id":"local_fallback", "tool_use_required":false,"dry_run":false}),
        };
        let before = hits.load(Ordering::SeqCst);
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            svc.proxy_openai(req.clone()),
        )
        .await
        .unwrap()
        .expect("advisory scorer failure is not a global veto");
        assert_eq!(hits.load(Ordering::SeqCst), before + 1);
        assert_eq!(out["status"], 200);
        assert_eq!(out["route"]["governance"]["realm_passed"], false);
        assert_eq!(
            out["route"]["governance"]["runtime_blocking"]["blocking_enabled"],
            false
        );
        let receipts = out["route"]["governance"]["scorer_receipts"]
            .as_array()
            .unwrap();
        assert_eq!(receipts.len(), 3);
        for receipt in receipts {
            assert_eq!(receipt["state"], serde_json::to_value(state).unwrap());
        }
        req.agent_id = "r08-stream-fixture".into();
        let stream_body = serde_json::json!({"messages":req.messages,"stream":true});
        let streamed = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            svc.proxy_openai_streaming(req.clone(), stream_body.clone()),
        )
        .await
        .unwrap()
        .expect("stream admission");
        assert_eq!(streamed.response.status().as_u16(), 200);
        assert!(bounded(streamed.response.text())
            .await
            .unwrap()
            .contains("data: [DONE]"));
        assert_eq!(hits.load(Ordering::SeqCst), before + 2);
        let events = svc.recent_governance_events(16);
        let stream_event = events
            .iter()
            .find(|event| {
                event["event"] == "route_selected" && event["payload"]["agent_id"] == req.agent_id
            })
            .unwrap();
        assert_eq!(stream_event["payload"]["agent_id"], req.agent_id);
        assert_eq!(stream_event["payload"]["governance"]["realm_passed"], false);
        assert_eq!(
            events
                .iter()
                .filter(|event| event["event"] == "route_selected")
                .count(),
            2
        );
        assert_eq!(
            stream_event["payload"]["governance"]["runtime_blocking"]["blocking_enabled"],
            false
        );
        assert_eq!(
            stream_event["payload"]["governance"]["scorer_receipts"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        for receipt in stream_event["payload"]["governance"]["scorer_receipts"]
            .as_array()
            .unwrap()
        {
            assert_eq!(receipt["state"], serde_json::to_value(state).unwrap());
        }
        req.options["exclude_provider_ids"] = serde_json::json!(["local_fallback"]);
        let refused = bounded(svc.proxy_openai(req.clone())).await;
        assert!(
            refused.is_err(),
            "scorer outage must not bypass provider exclusion"
        );
        assert!(bounded(svc.proxy_openai_streaming(req, stream_body))
            .await
            .is_err());
        assert_eq!(
            hits.load(Ordering::SeqCst),
            before + 2,
            "refusal sent a provider request"
        );
    }
    stop.send(()).unwrap();
    bounded(server).await.unwrap();
}

struct FaultScorer(GovernanceScorerState);
impl GovernanceScorer for FaultScorer {
    fn scorer_id(&self) -> &str {
        "route-fault-fixture"
    }
    fn score<'a>(&'a self, _: GovernanceScoreRequest) -> GovernanceScoreFuture<'a> {
        Box::pin(async move {
            match self.0 {
                GovernanceScorerState::Timeout => std::future::pending().await,
                GovernanceScorerState::Unavailable => Err(GovernanceScorerError::new(
                    GovernanceScorerErrorKind::Unavailable,
                    "fixture scorer unavailable",
                )),
                _ => Err(GovernanceScorerError::new(
                    GovernanceScorerErrorKind::Backend,
                    "fixture scorer failed",
                )),
            }
        })
    }
}

#[tokio::test]
async fn route_preserves_scorer_outage_without_minting_blocking_authority() {
    let _guard = isolate_env_lock(&ENV_LOCK);
    for state in [
        GovernanceScorerState::Unavailable,
        GovernanceScorerState::Timeout,
        GovernanceScorerState::Error,
    ] {
        let dir = tempdir().expect("tempdir");
        let _environment = ProviderEnvironment::isolate(dir.path());
        let mut svc = CharonService::new(dir.path()).expect("service");
        svc.realm_scorer = Arc::new(FaultScorer(state));
        let out = svc
            .route(ManweRequestEnvelope {
                agent_id: "athena".into(),
                task_type: "chat".into(),
                priority: "normal".into(),
                messages: vec![serde_json::json!({"role":"user","content":"test"})],
                options: serde_json::json!({}),
            })
            .await
            .expect("advisory outage does not invent a global veto");
        assert!(!out.provider_id.is_empty());
        assert!(!out.governance.realm_passed);
        assert!(!out.governance.runtime_blocking.blocking_enabled);
        assert_eq!(out.governance.scorer_receipts.len(), 3);
        for receipt in &out.governance.scorer_receipts {
            assert_eq!(receipt.scorer_id, "route-fault-fixture");
            assert_eq!(receipt.state, state);
            assert_eq!(receipt.score, 0.0);
            assert!(receipt.diagnostic.is_some());
        }
        let events = svc.recent_governance_events(8);
        let recorded = events
            .iter()
            .find(|e| e["event"] == "route_selected")
            .expect("retained route decision");
        assert_eq!(recorded["payload"]["governance"]["realm_passed"], false);
        assert_eq!(
            recorded["payload"]["governance"]["scorer_receipts"],
            serde_json::to_value(&out.governance.scorer_receipts).unwrap()
        );
    }
}
