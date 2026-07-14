// D1: in-process active health probes.
//
// Without this, providers that haven't been routed to recently go cold —
// their TLS connections close, capacity probes go stale, and any breakage
// (auth expiry, endpoint deprecation, transient region outage) is only
// discovered when a real user request arrives. The external systemd timer
// (`annunimas-charon-inference-probe.timer`) runs every 10 minutes and is
// good for cron-style sweeps but too coarse to keep the connection pool
// warm or to give Grafana a useful liveness signal.
//
// What this loop does:
// - Every PROBE_INTERVAL, snapshot the providers list under a brief read.
// - For each enabled, HTTP-driver provider with a base_url, GET
//   {base_url}/models with a short timeout. This is the cheapest endpoint
//   that every OpenAI-compatible upstream supports (including llama.cpp).
// - Record `charon_provider_probes_total{provider,outcome}` and the last
//   successful probe latency.
// - Probe-side connection reuse comes for free via the B4 client cache.
//
// What this loop deliberately does NOT do:
// - Mutate provider state (consecutive_failures, in_cooldown, etc.). The
//   live failure-feedback path already handles real user-traffic failures;
//   poisoning provider state from probe blips would create a feedback loop
//   where a flaky external probe path takes down good providers.
// - Replace the existing `annunimas-charon-inference-probe` timer (that
//   one runs heavier checks like real chat completions to validate
//   end-to-end reachability; this loop is connection-warmer + liveness).

use super::CharonService;
use crate::types::ProviderState;
use std::time::{Duration, Instant};

const PROBE_INTERVAL: Duration = Duration::from_secs(60);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub fn spawn(service: CharonService) {
    tokio::spawn(async move {
        // Stagger the first sweep so we don't hammer every upstream the
        // moment Charon starts (other startup tasks are also racing for
        // CPU/network).
        tokio::time::sleep(Duration::from_secs(15)).await;
        loop {
            probe_all(&service).await;
            tokio::time::sleep(PROBE_INTERVAL).await;
        }
    });
}

async fn probe_all(service: &CharonService) {
    let snapshot: Vec<ProviderState> = {
        let providers = service.providers_read().await;
        providers
            .iter()
            .filter(|p| should_probe(p))
            .cloned()
            .collect()
    };
    if snapshot.is_empty() {
        return;
    }
    // Spawn each probe so a slow provider can't block the sweep. The probe
    // function itself uses a 5s reqwest timeout, so the worst-case wall
    // time per cycle is bounded.
    let mut handles = Vec::with_capacity(snapshot.len());
    for provider in snapshot {
        let service = service.clone();
        handles.push(tokio::spawn(async move {
            probe_one(&service, &provider).await;
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

fn should_probe(p: &ProviderState) -> bool {
    if !p.enabled {
        return false;
    }
    if p.driver == "hermes_agent_cli" {
        return false;
    }
    p.base_url.is_some()
}

async fn probe_one(service: &CharonService, provider: &ProviderState) {
    let Some(base_url) = provider.base_url.as_deref() else {
        return;
    };
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    // Build a lightweight client per probe call — this is rare enough (1×
    // per provider per minute) that bypassing the per-(lane,mode) cache is
    // fine, and keeps probe failures from leaving stale entries in the
    // hot-path client cache. The connection pool is still shared with the
    // real proxy clients via reqwest's internal global pool only when the
    // hostname matches; cold probes mostly serve as a TLS keepalive nudge.
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(PROBE_TIMEOUT)
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!(error = %err, provider = %provider.id, "charon probe client build failed");
            service
                .metrics()
                .observe_provider_probe(&provider.id, false, 0);
            return;
        }
    };
    let mut request = client.get(&url);
    if let Some(env_key) = provider.api_key_env.as_deref() {
        if let Ok(key) = std::env::var(env_key) {
            if !key.trim().is_empty() {
                request = request.bearer_auth(key);
            }
        }
    }
    let started = Instant::now();
    match request.send().await {
        Ok(response) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let ok = response.status().is_success() || response.status().as_u16() == 401;
            // 401 = endpoint reachable but auth missing/wrong; that's a config
            // issue, not a liveness one — count as "reachable" so the gauge
            // doesn't oscillate while operators rotate keys.
            service
                .metrics()
                .observe_provider_probe(&provider.id, ok, latency_ms);
            if !ok {
                tracing::debug!(
                    provider = %provider.id,
                    status = response.status().as_u16(),
                    "charon probe non-2xx"
                );
            }
        }
        Err(err) => {
            service
                .metrics()
                .observe_provider_probe(&provider.id, false, 0);
            tracing::debug!(provider = %provider.id, error = %err, "charon probe failed");
        }
    }
}
