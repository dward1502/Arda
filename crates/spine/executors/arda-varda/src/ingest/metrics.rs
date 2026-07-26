// sigil: REPAIR
//
// In-process Prometheus counter/gauge/histogram store for ATHENA.
// Mirrors `crates/annunimas-charon/src/service/metrics.rs` — hand-rolled,
// no extra dep, single Mutex; emission paths are not in the hottest loop.
//
// Phase 4 scope (subset of OPTIMIZATION_PLAN.md C1):
//   athena_ingest_documents_total{source_kind,outcome}
//   athena_deep_runs_total{outcome}                          (outcome ∈ extraction_status)
//   athena_deep_queue_depth                                  (gauge — pending_deep events)
//   athena_query_total
//   athena_query_latency_seconds                             (histogram)
//   athena_index_rebuilds_total
//   athena_index_entries                                     (gauge — last index size)
//   athena_policy_readiness_malformed_records                (gauge — JSONL parse failures)
//   athena_source_age_seconds{source_id}                     (gauge — full-refresh age)

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

const QUERY_LATENCY_BUCKETS_S: [f64; 10] =
    [0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0];

#[derive(Default)]
pub struct AthenaMetrics {
    inner: Mutex<MetricsInner>,
}

#[derive(Default)]
struct MetricsInner {
    ingest_documents: HashMap<(String, String), u64>,
    deep_runs: HashMap<String, u64>,
    deep_queue_depth: u64,
    query_total: u64,
    query_latency_buckets: [u64; 11],
    query_latency_sum: f64,
    query_latency_count: u64,
    index_rebuilds: u64,
    index_entries: u64,
    policy_readiness_malformed_records: u64,
    source_age_seconds: BTreeMap<String, u64>,
}

impl AthenaMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_ingest(&self, source_kind: &str, outcome: &str) {
        let mut g = self.lock();
        *g.ingest_documents
            .entry((source_kind.to_string(), outcome.to_string()))
            .or_insert(0) += 1;
    }

    pub fn observe_deep_run(&self, outcome: &str) {
        let mut g = self.lock();
        *g.deep_runs.entry(outcome.to_string()).or_insert(0) += 1;
    }

    pub fn set_deep_queue_depth(&self, depth: u64) {
        let mut g = self.lock();
        g.deep_queue_depth = depth;
    }

    pub fn observe_query(&self, latency_secs: f64) {
        let mut g = self.lock();
        g.query_total += 1;
        for (idx, bound) in QUERY_LATENCY_BUCKETS_S.iter().enumerate() {
            if latency_secs <= *bound {
                g.query_latency_buckets[idx] = g.query_latency_buckets[idx].saturating_add(1);
            }
        }
        g.query_latency_buckets[10] = g.query_latency_buckets[10].saturating_add(1);
        g.query_latency_sum += latency_secs;
        g.query_latency_count += 1;
    }

    pub fn observe_index_rebuild(&self, entry_count: u64) {
        let mut g = self.lock();
        g.index_rebuilds += 1;
        g.index_entries = entry_count;
    }

    pub fn set_policy_readiness_malformed_records(&self, count: u64) {
        let mut g = self.lock();
        g.policy_readiness_malformed_records = count;
    }

    pub fn set_source_age_seconds<I>(&self, ages: I)
    where
        I: IntoIterator<Item = (String, u64)>,
    {
        let mut g = self.lock();
        g.source_age_seconds = ages.into_iter().collect();
    }

    pub fn snapshot(&self) -> AthenaMetricsSnapshot {
        let g = self.lock();
        AthenaMetricsSnapshot {
            ingest_documents_total: g.ingest_documents.values().sum(),
            deep_runs_total: g.deep_runs.values().sum(),
            deep_queue_depth: g.deep_queue_depth,
            query_total: g.query_total,
            query_latency_sum: g.query_latency_sum,
            query_latency_count: g.query_latency_count,
            index_rebuilds: g.index_rebuilds,
            index_entries: g.index_entries,
            policy_readiness_malformed_records: g.policy_readiness_malformed_records,
            source_age_series: g.source_age_seconds.len(),
        }
    }

    pub fn render_prometheus(&self) -> String {
        let g = self.lock();
        let mut buf = String::with_capacity(2048);

        buf.push_str(
            "# HELP athena_ingest_documents_total Documents ingested by source kind and outcome\n",
        );
        buf.push_str("# TYPE athena_ingest_documents_total counter\n");
        for ((kind, outcome), count) in &g.ingest_documents {
            buf.push_str(&format!(
                "athena_ingest_documents_total{{source_kind=\"{}\",outcome=\"{}\"}} {}\n",
                escape(kind),
                escape(outcome),
                count
            ));
        }

        buf.push_str(
            "# HELP athena_deep_runs_total Deep-analysis (digestion) runs by extraction outcome\n",
        );
        buf.push_str("# TYPE athena_deep_runs_total counter\n");
        for (outcome, count) in &g.deep_runs {
            buf.push_str(&format!(
                "athena_deep_runs_total{{outcome=\"{}\"}} {}\n",
                escape(outcome),
                count
            ));
        }

        buf.push_str(
            "# HELP athena_deep_queue_depth Pending deep-analysis tasks awaiting processing\n",
        );
        buf.push_str("# TYPE athena_deep_queue_depth gauge\n");
        buf.push_str(&format!("athena_deep_queue_depth {}\n", g.deep_queue_depth));

        buf.push_str("# HELP athena_query_total Total query() calls served\n");
        buf.push_str("# TYPE athena_query_total counter\n");
        buf.push_str(&format!("athena_query_total {}\n", g.query_total));

        buf.push_str("# HELP athena_query_latency_seconds Query end-to-end latency\n");
        buf.push_str("# TYPE athena_query_latency_seconds histogram\n");
        for (idx, bound) in QUERY_LATENCY_BUCKETS_S.iter().enumerate() {
            buf.push_str(&format!(
                "athena_query_latency_seconds_bucket{{le=\"{}\"}} {}\n",
                bound, g.query_latency_buckets[idx]
            ));
        }
        buf.push_str(&format!(
            "athena_query_latency_seconds_bucket{{le=\"+Inf\"}} {}\n",
            g.query_latency_buckets[10]
        ));
        buf.push_str(&format!(
            "athena_query_latency_seconds_sum {}\n",
            format_metric_float(g.query_latency_sum)
        ));
        buf.push_str(&format!(
            "athena_query_latency_seconds_count {}\n",
            g.query_latency_count
        ));

        buf.push_str("# HELP athena_index_rebuilds_total In-memory digest index rebuilds\n");
        buf.push_str("# TYPE athena_index_rebuilds_total counter\n");
        buf.push_str(&format!(
            "athena_index_rebuilds_total {}\n",
            g.index_rebuilds
        ));

        buf.push_str(
            "# HELP athena_index_entries Entries in the last in-memory digest index build\n",
        );
        buf.push_str("# TYPE athena_index_entries gauge\n");
        buf.push_str(&format!("athena_index_entries {}\n", g.index_entries));

        buf.push_str("# HELP athena_policy_readiness_malformed_records Malformed policy-readiness JSONL records observed during status aggregation\n");
        buf.push_str("# TYPE athena_policy_readiness_malformed_records gauge\n");
        buf.push_str(&format!(
            "athena_policy_readiness_malformed_records {}\n",
            g.policy_readiness_malformed_records
        ));

        buf.push_str(
            "# HELP athena_source_age_seconds Seconds since the source was last fully refreshed\n",
        );
        buf.push_str("# TYPE athena_source_age_seconds gauge\n");
        for (source_id, age_seconds) in &g.source_age_seconds {
            buf.push_str(&format!(
                "athena_source_age_seconds{{source_id=\"{}\"}} {}\n",
                escape(source_id),
                age_seconds
            ));
        }

        buf
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MetricsInner> {
        match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AthenaMetricsSnapshot {
    pub ingest_documents_total: u64,
    pub deep_runs_total: u64,
    pub deep_queue_depth: u64,
    pub query_total: u64,
    pub query_latency_sum: f64,
    pub query_latency_count: u64,
    pub index_rebuilds: u64,
    pub index_entries: u64,
    pub policy_readiness_malformed_records: u64,
    pub source_age_series: usize,
}

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

fn format_metric_float(value: f64) -> String {
    let mut formatted = format!("{value:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

/// Classify an ingest outcome from an IngestRecord without exposing the
/// internal types to callers. Kept narrow on purpose to bound label cardinality.
pub fn classify_ingest_outcome(deduplicated: bool, error: Option<&str>) -> &'static str {
    if error.is_some() {
        "fail"
    } else if deduplicated {
        "dedup"
    } else {
        "ok"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_ingest_by_kind_and_outcome() {
        let m = AthenaMetrics::new();
        m.observe_ingest("github_repo", "ok");
        m.observe_ingest("github_repo", "ok");
        m.observe_ingest("github_repo", "dedup");
        m.observe_ingest("scholarly_link", "ok");
        let out = m.render_prometheus();
        assert!(out.contains(
            "athena_ingest_documents_total{source_kind=\"github_repo\",outcome=\"ok\"} 2"
        ));
        assert!(out.contains(
            "athena_ingest_documents_total{source_kind=\"github_repo\",outcome=\"dedup\"} 1"
        ));
        assert!(out.contains(
            "athena_ingest_documents_total{source_kind=\"scholarly_link\",outcome=\"ok\"} 1"
        ));
    }

    #[test]
    fn deep_runs_count_per_outcome() {
        let m = AthenaMetrics::new();
        m.observe_deep_run("llm_extraction_complete");
        m.observe_deep_run("llm_extraction_complete");
        m.observe_deep_run("llm_extraction_failed");
        let out = m.render_prometheus();
        assert!(out.contains("athena_deep_runs_total{outcome=\"llm_extraction_complete\"} 2"));
        assert!(out.contains("athena_deep_runs_total{outcome=\"llm_extraction_failed\"} 1"));
    }

    #[test]
    fn query_latency_histogram_buckets() {
        let m = AthenaMetrics::new();
        m.observe_query(0.0008);
        m.observe_query(0.012);
        m.observe_query(0.300);
        let out = m.render_prometheus();
        // 3 total observations
        assert!(out.contains("athena_query_total 3"));
        assert!(out.contains("athena_query_latency_seconds_count 3"));
        // 0.0008 + 0.012 + 0.300 = 0.3128
        assert!(out.contains("athena_query_latency_seconds_sum 0.3128"));
        // +Inf bucket should have all 3
        assert!(out.contains("athena_query_latency_seconds_bucket{le=\"+Inf\"} 3"));
    }

    #[test]
    fn classify_outcome_distinguishes_dedup_ok_fail() {
        assert_eq!(classify_ingest_outcome(false, None), "ok");
        assert_eq!(classify_ingest_outcome(true, None), "dedup");
        assert_eq!(classify_ingest_outcome(false, Some("boom")), "fail");
    }

    #[test]
    fn index_rebuild_updates_gauge_and_counter() {
        let m = AthenaMetrics::new();
        m.observe_index_rebuild(42);
        m.observe_index_rebuild(43);
        let out = m.render_prometheus();
        assert!(out.contains("athena_index_rebuilds_total 2"));
        assert!(out.contains("athena_index_entries 43"));
    }

    #[test]
    fn malformed_policy_readiness_gauge_renders_and_snapshots() {
        let m = AthenaMetrics::new();
        m.set_policy_readiness_malformed_records(2);
        let out = m.render_prometheus();
        assert!(out.contains("athena_policy_readiness_malformed_records 2"));
        assert_eq!(m.snapshot().policy_readiness_malformed_records, 2);
    }

    #[test]
    fn source_age_gauges_replace_and_render_per_source() {
        let m = AthenaMetrics::new();
        m.set_source_age_seconds([("src_beta".to_string(), 12), ("src_alpha".to_string(), 7)]);
        let out = m.render_prometheus();
        assert!(out.contains("athena_source_age_seconds{source_id=\"src_alpha\"} 7"));
        assert!(out.contains("athena_source_age_seconds{source_id=\"src_beta\"} 12"));
        assert_eq!(m.snapshot().source_age_series, 2);

        m.set_source_age_seconds([("src_alpha".to_string(), 8)]);
        let refreshed = m.render_prometheus();
        assert!(refreshed.contains("athena_source_age_seconds{source_id=\"src_alpha\"} 8"));
        assert!(!refreshed.contains("src_beta"));
    }

    #[test]
    fn snapshot_aggregates_counters() {
        let m = AthenaMetrics::new();
        m.observe_ingest("github_repo", "ok");
        m.observe_ingest("github_repo", "ok");
        m.observe_deep_run("llm_extraction_complete");
        m.set_deep_queue_depth(7);
        let s = m.snapshot();
        assert_eq!(s.ingest_documents_total, 2);
        assert_eq!(s.deep_runs_total, 1);
        assert_eq!(s.deep_queue_depth, 7);
    }
}
