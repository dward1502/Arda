//! Per-execution inference capability. Payload gets only a connected stdin;
//! neither a socket pathname nor a caller-selected HTTP destination is exposed.
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    io::Read, os::fd::AsRawFd, os::unix::net::UnixStream, thread::JoinHandle, time::Instant,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const REQUEST_LIMIT: usize = 2_097_152;
const RESPONSE_LIMIT: usize = 8_388_608;

pub struct Broker {
    routes: attestation::RouteLog,
    failure: std::sync::Arc<std::sync::Mutex<Option<&'static str>>>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}
impl Broker {
    pub fn start(
        runtime: &crate::snapshot_runtime::CapturedRuntime,
        deadline: Instant,
    ) -> Result<(Self, UnixStream)> {
        let home = runtime
            .policy
            .policy()
            .fixed_environment
            .get("HERMES_HOME")
            .context("missing profile")?;
        let destination = std::path::Path::new(home).join("config.yaml");
        let grant = runtime
            .grants
            .iter()
            .find(|g| g.identity.destination == destination)
            .context("missing captured profile")?;
        let mut raw = Vec::new();
        std::fs::File::open(format!("/proc/self/fd/{}", grant.descriptor.as_raw_fd()))?
            .take(65_537)
            .read_to_end(&mut raw)?;
        if raw.len() > 65_536 {
            bail!("profile exceeds broker bound");
        }
        let profile: Value = serde_json::from_slice(&raw)?;
        let endpoint = format!("{}/v1/chat/completions", runtime.endpoint()?);
        let policy = Policy::from_selected_profile(&profile, runtime.endpoint()?)?;
        use sha2::{Digest, Sha256};
        let routes =
            attestation::RouteLog::new(format!("sha256:{:x}", Sha256::digest(&raw)), &policy);
        let captured_routes = routes.clone();
        let (host, payload) = UnixStream::pair()?;
        host.set_nonblocking(true)?;
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let reported_failure = failure.clone();
        let thread = std::thread::Builder::new()
            .name("inference-broker".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    *reported_failure.lock().unwrap() = Some("runtime_init");
                    return;
                };
                rt.block_on(async move {
                    tokio::select! {
                        _ = stopped => {},
                        _ = tokio::time::sleep_until(deadline.into()) => {},
                        result = serve(host, policy, captured_routes, &endpoint) => {
                            if let Err(error) = result {
                                let phase = if let Some(error) = error.downcast_ref::<reqwest::Error>() {
                                    if error.is_builder() { "http_client_init" } else { "http_transport" }
                                } else if error.downcast_ref::<std::io::Error>().is_some() { "channel_io" }
                                else { match error.to_string().as_str() {
                                    "inference upstream rejected request" => "upstream_rejected",
                                    "broker_body_policy" => match error.root_cause().to_string().as_str() {
                                        "unapproved request fields" => "body_fields",
                                        "unapproved request model" => "body_model",
                                        "unapproved request routing" => "body_routing",
                                        _ => "body_policy",
                                    },
                                    "missing route evidence" => "missing_route_evidence",
                                    "unapproved inference route" => "route_mismatch",
                                    _ => "request_or_route_policy",
                                }};
                                eprintln!("inference broker stopped: phase={phase}");
                                *reported_failure.lock().unwrap() = Some(phase);
                            }
                        },
                    }
                });
            })?;
        Ok((
            Self {
                routes,
                failure,
                stop: Some(stop),
                thread: Some(thread),
            },
            payload,
        ))
    }
    pub fn evidence(&self) -> arda_engine::adapters::InferenceRouteEvidence {
        self.routes.snapshot()
    }
    pub fn failure(&self) -> Option<&'static str> {
        *self.failure.lock().unwrap()
    }
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Broker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[path = "inference_policy.rs"]
mod policy;
use policy::Policy;
#[path = "inference_attestation.rs"]
mod attestation;

async fn serve(
    host: UnixStream,
    policy: Policy,
    routes: attestation::RouteLog,
    endpoint: &str,
) -> Result<()> {
    serve_at_with_routes(host, policy, endpoint, routes).await
}

#[cfg(test)]
async fn serve_at(host: UnixStream, policy: Policy, endpoint: &str) -> Result<()> {
    let routes = attestation::RouteLog::new(format!("sha256:{}", "0".repeat(64)), &policy);
    serve_at_with_routes(host, policy, endpoint, routes).await
}

async fn serve_at_with_routes(
    host: UnixStream,
    policy: Policy,
    endpoint: &str,
    routes: attestation::RouteLog,
) -> Result<()> {
    let mut channel = tokio::net::UnixStream::from_std(host)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    loop {
        let size = channel.read_u32().await? as usize;
        if !(1..=REQUEST_LIMIT).contains(&size) {
            bail!("broker request size rejected");
        }
        let mut raw = vec![0; size];
        channel.read_exact(&mut raw).await?;
        let body = policy.request(&raw).context("broker_body_policy")?;
        routes.ensure_capacity()?;
        let mut response = client.post(endpoint).json(&body).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            // Preserve status for SDK retry/error handling; never forward an
            // untrusted provider error body or follow its redirect.
            let out = serde_json::to_vec(&json!({"status":status,
                "headers":{"content-type":"application/json"},
                "body":json!({"error":{"message":format!("Local Manwe rejected inference (HTTP {status})"),
                    "type":"upstream_error","code":status}}).to_string()}))?;
            channel.write_u32(out.len() as u32).await?;
            channel.write_all(&out).await?;
            continue;
        }
        let mut headers = attestation::verified_headers(&response, &policy)?;
        if let Some(kind) = response.headers().get("content-type") {
            headers.insert("content-type".into(), json!(kind.to_str()?));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > RESPONSE_LIMIT {
                bail!("broker response size rejected");
            }
            bytes.extend_from_slice(&chunk);
        }
        let frame = serde_json::to_vec(
            &json!({"status":200, "headers":headers, "body":String::from_utf8(bytes)?}),
        )?;
        if frame.len() > 16_777_216 {
            bail!("broker serialized response exceeds bound");
        }
        routes.record(&headers)?;
        channel.write_u32(frame.len() as u32).await?;
        channel.write_all(&frame).await?;
    }
}

#[cfg(test)]
#[path = "inference_broker_retry_tests.rs"]
mod retry_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn real_http_forwarding_and_local_cancellation() {
        // Synthetic local HTTP peer; never contact the live inference service.
        for (cancel, status) in [(false, 200), (true, 200), (false, 429)] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!(
                "http://{}/v1/chat/completions",
                listener.local_addr().unwrap()
            );
            let policy = Policy {
                model: "fixture-model".into(),
                provider: "edge_fixture".into(),
                routing: json!({"local_only":true}),
            };
            let (host, payload) = UnixStream::pair().unwrap();
            host.set_nonblocking(true).unwrap();
            payload.set_nonblocking(true).unwrap();
            let mut payload = tokio::net::UnixStream::from_std(payload).unwrap();
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let routes = attestation::RouteLog::new(format!("sha256:{}", "1".repeat(64)), &policy);
            let observed = routes.clone();
            let task = tokio::spawn(async move {
                tokio::select! { _ = stopped => {}, result = serve_at_with_routes(host, policy, &endpoint, routes) => { result.unwrap(); } }
            });
            let request =
                serde_json::to_vec(&json!({"model":"fixture-model","messages":[]})).unwrap();
            payload.write_u32(request.len() as u32).await.unwrap();
            payload.write_all(&request).await.unwrap();
            let (mut upstream, _) =
                tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            let mut received = Vec::new();
            loop {
                received.push(upstream.read_u8().await.unwrap());
                if received.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let headers = String::from_utf8(received).unwrap();
            assert!(headers.starts_with("POST /v1/chat/completions HTTP/1.1"));
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(|s| s.parse().unwrap())
                })
                .unwrap();
            let mut body = vec![0; length];
            upstream.read_exact(&mut body).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&body).unwrap()["routing"],
                json!({"local_only":true})
            );
            if !cancel {
                let body = "data: synthetic fixture\n\ndata: [DONE]\n\n";
                let response = format!("HTTP/1.1 {status} Fixture\r\ncontent-length: {}\r\ncontent-type: text/event-stream\r\nx-manwe-route-id: fixture\r\nx-manwe-provider-id: edge_fixture\r\nx-manwe-model-id: fixture-model\r\nx-manwe-route-class: tool_oriented\r\n\r\n{body}", body.len());
                upstream.write_all(response.as_bytes()).await.unwrap();
                let length = payload.read_u32().await.unwrap();
                let mut result = vec![0; length as usize];
                payload.read_exact(&mut result).await.unwrap();
                let result: Value = serde_json::from_slice(&result).unwrap();
                assert_eq!(result["status"], status);
                if status == 200 {
                    assert_eq!(result["body"], body);
                } else {
                    assert!(!result["body"]
                        .as_str()
                        .unwrap()
                        .contains("synthetic fixture"));
                }
            }
            stop.send(()).unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(2), task)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    upstream.read(&mut [0; 1])
                )
                .await
                .unwrap()
                .unwrap(),
                0
            );
            assert_eq!(payload.read(&mut [0; 1]).await.unwrap(), 0);
            let evidence = observed.snapshot();
            assert_eq!(evidence.routes.len(), usize::from(!cancel && status == 200));
            if !evidence.routes.is_empty() {
                assert!(evidence.validate());
                assert_eq!(evidence.routes[0].route_id, "fixture");
            }
        }
    }
    #[test]
    fn destination_and_route_are_supervisor_owned() {
        let routing = json!({"local_only":true,"inference_origin":"local","origin_preference":"local","force_provider_id":"edge_test","force_model_id":"test","allow_forced_provider_fallback":false,"tool_use_required":true});
        let profile = json!({"model":{"default":"test"},"custom_providers":[{"base_url":"http://127.0.0.1:7171/v1","extra_body":{"routing":routing}}]});
        let policy = Policy::from_profile(&profile).unwrap();
        let valid = json!({"model":"test","routing":routing,"messages":[]});
        assert!(policy.request(&serde_json::to_vec(&valid).unwrap()).is_ok());
        for field in ["url", "method", "headers", "provider", "base_url"] {
            let mut bad = valid.clone();
            bad[field] = json!("override");
            assert!(policy.request(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = valid;
        bad["routing"]["allow_forced_provider_fallback"] = json!(true);
        assert!(policy.request(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}
