use arda_vaire::{InformantEvent, MnemosyneService};
use chrono::Utc;
use std::hint::black_box;
use std::time::Instant;
use tempfile::tempdir;

const FIXTURES: [(&str, &str, &str); 6] = [
    (
        "council evidence receipt",
        "Council evidence receipt verified for governance review",
        "scope:boardroom_council",
    ),
    (
        "human accessibility preference",
        "Human accessibility preference recorded for interaction continuity",
        "scope:human_context",
    ),
    (
        "citadel edge telemetry",
        "Citadel edge telemetry recovered after runtime restart",
        "scope:edge_runtime",
    ),
    (
        "continuity checkpoint",
        "System continuity checkpoint preserved across agent handoff",
        "scope:system_continuity",
    ),
    (
        "memory promotion receipt",
        "Memory promotion receipt links source episodes to semantic pattern",
        "scope:system_continuity",
    ),
    (
        "arden deployment lesson",
        "Arden deployment lesson identifies bounded queue configuration",
        "scope:edge_runtime",
    ),
];

fn main() {
    let dir = tempdir().expect("benchmark tempdir");
    let service = MnemosyneService::new(dir.path()).expect("benchmark service");

    for (index, (_, content, scope)) in FIXTURES.iter().enumerate() {
        service
            .encode(InformantEvent {
                informant_id: "recall_fidelity_bench".to_owned(),
                crate_name: format!("fixture_{index}"),
                event_type: "knowledge_delta".to_owned(),
                ts_utc: Utc::now().to_rfc3339(),
                content: (*content).to_owned(),
                confidence_hint: Some(0.9),
                tags: vec!["benchmark".to_owned(), (*scope).to_owned()],
            })
            .expect("encode fixture")
            .expect("fixture must be durable");
    }

    let iterations = 100usize;
    let started = Instant::now();
    let mut hits = 0usize;
    let mut queries = 0usize;
    for _ in 0..iterations {
        for (index, (query, expected, _)) in FIXTURES.iter().enumerate() {
            let results = service
                .recall_relevant(query, 24, Some(&format!("fixture_{index}")), None, 1)
                .expect("benchmark recall");
            hits += usize::from(
                results
                    .first()
                    .is_some_and(|entry| entry.content == *expected),
            );
            queries += 1;
            black_box(results);
        }
    }

    let elapsed = started.elapsed();
    let fidelity = hits as f64 / queries as f64;
    let micros_per_query = elapsed.as_micros() as f64 / queries as f64;
    let metrics = service.observability_snapshot();
    println!(
        "recall_fidelity: queries={queries} hit_at_1={fidelity:.3} elapsed_ms={} us_per_query={micros_per_query:.2} observed_recall_requests={}",
        elapsed.as_millis(),
        metrics.recall_requests_total
    );
    assert_eq!(fidelity, 1.0, "fixture recall fidelity regressed");
}
