// In-process Prometheus counter/gauge/histogram store for Charon.
//
// Hand-rolled (no `prometheus` crate) to keep the dep tree small and to live
// alongside the existing hand-rendered /metrics output. All ops take a short
// Mutex critical section; emission paths (route picks, mark_provider_result,
// stream chunk errors, proxy completion) are not in the absolute hottest
// loop, so a single Mutex is fine until B1 lock-coalescing lands.
//
// Scope (this is C1 from OPTIMIZATION_PLAN.md):
//   charon_route_decisions_total{provider,model,task_type,lane}
//   charon_provider_failures_total{provider,reason_class}
//   charon_streaming_chunk_errors_total{provider,model}
//   charon_route_score{provider,model}                       (gauge — last score)
//   charon_proxy_latency_seconds{provider,lane}              (histogram)

use std::collections::HashMap;
use std::sync::Mutex;

const LATENCY_BUCKETS_S: [f64; 11] = [
    0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0,
];

#[derive(Default)]
pub struct CharonMetrics {
    inner: Mutex<MetricsInner>,
}

#[derive(Default)]
struct MetricsInner {
    route_decisions: HashMap<(String, String, String, String), u64>,
    provider_failures: HashMap<(String, String), u64>,
    streaming_chunk_errors: HashMap<(String, String), u64>,
    route_scores: HashMap<(String, String), f64>,
    proxy_latency_buckets: HashMap<(String, String), [u64; 12]>,
    proxy_latency_sum: HashMap<(String, String), f64>,
    proxy_latency_count: HashMap<(String, String), u64>,
    /// (provider, outcome) → count. outcome ∈ {"ok","fail"}.
    provider_probes: HashMap<(String, String), u64>,
    /// provider → last probe latency (ms). 0 = no successful probe yet.
    provider_probe_latency_ms: HashMap<String, u64>,
}

impl CharonMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_route_pick(
        &self,
        provider: &str,
        model: &str,
        task_type: &str,
        lane: &str,
        score: f64,
    ) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *g.route_decisions
            .entry((
                provider.to_string(),
                model.to_string(),
                task_type.to_string(),
                lane.to_string(),
            ))
            .or_insert(0) += 1;
        g.route_scores
            .insert((provider.to_string(), model.to_string()), score);
    }

    pub fn observe_provider_failure(&self, provider: &str, reason_class: &str) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *g.provider_failures
            .entry((provider.to_string(), reason_class.to_string()))
            .or_insert(0) += 1;
    }

    pub fn observe_streaming_chunk_error(&self, provider: &str, model: &str) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *g.streaming_chunk_errors
            .entry((provider.to_string(), model.to_string()))
            .or_insert(0) += 1;
    }

    pub fn observe_provider_probe(&self, provider: &str, ok: bool, latency_ms: u64) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let outcome = if ok { "ok" } else { "fail" };
        *g.provider_probes
            .entry((provider.to_string(), outcome.to_string()))
            .or_insert(0) += 1;
        if ok {
            g.provider_probe_latency_ms
                .insert(provider.to_string(), latency_ms);
        }
    }

    pub fn observe_proxy_latency(&self, provider: &str, lane: &str, latency_ms: u64) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let key = (provider.to_string(), lane.to_string());
        let secs = latency_ms as f64 / 1000.0;
        let buckets = g
            .proxy_latency_buckets
            .entry(key.clone())
            .or_insert([0u64; 12]);
        for (idx, bound) in LATENCY_BUCKETS_S.iter().enumerate() {
            if secs <= *bound {
                buckets[idx] = buckets[idx].saturating_add(1);
            }
        }
        buckets[11] = buckets[11].saturating_add(1);
        *g.proxy_latency_sum.entry(key.clone()).or_insert(0.0) += secs;
        *g.proxy_latency_count.entry(key).or_insert(0) += 1;
    }

    pub fn render_prometheus(&self) -> String {
        let g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut buf = String::with_capacity(2048);

        buf.push_str("# HELP charon_route_decisions_total Routing decisions by provider/model/task_type/lane\n");
        buf.push_str("# TYPE charon_route_decisions_total counter\n");
        for ((provider, model, task_type, lane), count) in &g.route_decisions {
            buf.push_str(&format!(
                "charon_route_decisions_total{{provider=\"{}\",model=\"{}\",task_type=\"{}\",lane=\"{}\"}} {}\n",
                escape(provider),
                escape(model),
                escape(task_type),
                escape(lane),
                count
            ));
        }

        buf.push_str("# HELP charon_provider_failures_total Provider failure events by class\n");
        buf.push_str("# TYPE charon_provider_failures_total counter\n");
        for ((provider, reason), count) in &g.provider_failures {
            buf.push_str(&format!(
                "charon_provider_failures_total{{provider=\"{}\",reason_class=\"{}\"}} {}\n",
                escape(provider),
                escape(reason),
                count
            ));
        }

        buf.push_str("# HELP charon_streaming_chunk_errors_total SSE bytes_stream chunk decode errors after a successful HTTP 200\n");
        buf.push_str("# TYPE charon_streaming_chunk_errors_total counter\n");
        for ((provider, model), count) in &g.streaming_chunk_errors {
            buf.push_str(&format!(
                "charon_streaming_chunk_errors_total{{provider=\"{}\",model=\"{}\"}} {}\n",
                escape(provider),
                escape(model),
                count
            ));
        }

        buf.push_str("# HELP charon_route_score Last routing score for a provider/model pick\n");
        buf.push_str("# TYPE charon_route_score gauge\n");
        for ((provider, model), score) in &g.route_scores {
            buf.push_str(&format!(
                "charon_route_score{{provider=\"{}\",model=\"{}\"}} {}\n",
                escape(provider),
                escape(model),
                score
            ));
        }

        buf.push_str(
            "# HELP charon_provider_probes_total Active health-probe attempts by provider/outcome\n",
        );
        buf.push_str("# TYPE charon_provider_probes_total counter\n");
        for ((provider, outcome), count) in &g.provider_probes {
            buf.push_str(&format!(
                "charon_provider_probes_total{{provider=\"{}\",outcome=\"{}\"}} {}\n",
                escape(provider),
                escape(outcome),
                count
            ));
        }
        buf.push_str(
            "# HELP charon_provider_probe_latency_ms Last successful probe latency in ms\n",
        );
        buf.push_str("# TYPE charon_provider_probe_latency_ms gauge\n");
        for (provider, latency) in &g.provider_probe_latency_ms {
            buf.push_str(&format!(
                "charon_provider_probe_latency_ms{{provider=\"{}\"}} {}\n",
                escape(provider),
                latency
            ));
        }

        buf.push_str(
            "# HELP charon_proxy_latency_seconds Upstream proxy latency by provider/lane\n",
        );
        buf.push_str("# TYPE charon_proxy_latency_seconds histogram\n");
        for ((provider, lane), buckets) in &g.proxy_latency_buckets {
            for (idx, bound) in LATENCY_BUCKETS_S.iter().enumerate() {
                buf.push_str(&format!(
                    "charon_proxy_latency_seconds_bucket{{provider=\"{}\",lane=\"{}\",le=\"{}\"}} {}\n",
                    escape(provider),
                    escape(lane),
                    bound,
                    buckets[idx]
                ));
            }
            buf.push_str(&format!(
                "charon_proxy_latency_seconds_bucket{{provider=\"{}\",lane=\"{}\",le=\"+Inf\"}} {}\n",
                escape(provider),
                escape(lane),
                buckets[11]
            ));
            let sum = g
                .proxy_latency_sum
                .get(&(provider.clone(), lane.clone()))
                .copied()
                .unwrap_or(0.0);
            let count = g
                .proxy_latency_count
                .get(&(provider.clone(), lane.clone()))
                .copied()
                .unwrap_or(0);
            buf.push_str(&format!(
                "charon_proxy_latency_seconds_sum{{provider=\"{}\",lane=\"{}\"}} {}\n",
                escape(provider),
                escape(lane),
                sum
            ));
            buf.push_str(&format!(
                "charon_proxy_latency_seconds_count{{provider=\"{}\",lane=\"{}\"}} {}\n",
                escape(provider),
                escape(lane),
                count
            ));
        }

        buf
    }
}

/// Escape Prometheus label values per text-format spec: `\\`, `\"`, `\n`.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

/// Coarse classifier from a free-form provider error string. Bins the long
/// tail of provider error strings into a fixed set so the metric cardinality
/// stays bounded.
pub fn classify_failure_reason(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("streaming chunk decode failed") {
        "streaming_chunk_decode"
    } else if lower.contains("http 429") || lower.contains("rate") {
        "rate_limited"
    } else if lower.contains("http 5") {
        "upstream_5xx"
    } else if lower.contains("http 4") {
        "upstream_4xx"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout"
    } else if lower.contains("connect") {
        "connect_failed"
    } else if lower.contains("preflight") {
        "preflight_blocked"
    } else if lower.contains("hermes_agent_cli") {
        "hermes_cli_exit"
    } else {
        "other"
    }
}