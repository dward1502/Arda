use arda_prometheus::autopilot::{
    AgentCapabilities, AgentRegistry, LearningState, OutcomeObserver, TaskQueueAnalyzer,
};
use arda_prometheus::core_link::CoreAutonomyProfile;
use arda_prometheus::heartbeat::select_heartbeat_mode;
use arda_prometheus::orders::{OrderStatus, OrderStore};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

fn make_profile(world_status: &str, resonance: f64) -> CoreAutonomyProfile {
    CoreAutonomyProfile {
        heartbeat_ms: 500,
        triad_bypass: false,
        base_costs: HashMap::new(),
        world_status: Some(world_status.to_string()),
        world_resonance: Some(resonance),
        source_root: PathBuf::from("core"),
    }
}

fn bench_heartbeat_mode_selection(c: &mut Criterion) {
    let online = make_profile("ONLINE", 75.0);
    let degraded = make_profile("DEGRADED", 30.0);

    c.bench_function("heartbeat_mode_online", |b| {
        b.iter(|| select_heartbeat_mode(Some(&online)))
    });
    c.bench_function("heartbeat_mode_degraded", |b| {
        b.iter(|| select_heartbeat_mode(Some(&degraded)))
    });
    c.bench_function("heartbeat_mode_no_profile", |b| {
        b.iter(|| select_heartbeat_mode(None))
    });
}

fn bench_order_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_store");

    for n in [10u32, 100, 500] {
        group.bench_with_input(BenchmarkId::new("active_orders_count", n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let dir = tempdir().expect("tempdir");
                    let store = OrderStore::new(dir.path()).expect("store");
                    for _ in 0..n {
                        let id = Uuid::new_v4();
                        store
                            .append_order(id, "query", OrderStatus::Open, None, "bench")
                            .expect("append");
                    }
                    (dir, store)
                },
                |(_dir, store)| store.active_orders_count().expect("count"),
                BatchSize::SmallInput,
            )
        });

        group.bench_with_input(
            BenchmarkId::new("pending_escalations_count", n),
            &n,
            |b, &n| {
                b.iter_batched(
                    || {
                        let dir = tempdir().expect("tempdir");
                        let store = OrderStore::new(dir.path()).expect("store");
                        for _ in 0..n {
                            let id = Uuid::new_v4();
                            store
                                .append_escalation(id, "confidence low", 0.4)
                                .expect("escalate");
                        }
                        (dir, store)
                    },
                    |(_dir, store)| store.pending_escalations_count().expect("count"),
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn bench_escalation_resolve(c: &mut Criterion) {
    c.bench_function("escalation_resolve_cycle", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().expect("tempdir");
                let store = OrderStore::new(dir.path()).expect("store");
                let id = Uuid::new_v4();
                let esc_id = store
                    .append_escalation(id, "confidence low", 0.4)
                    .expect("escalate");
                (dir, store, esc_id)
            },
            |(_dir, store, esc_id)| {
                store
                    .resolve_escalation(&esc_id, "approved")
                    .expect("resolve")
            },
            BatchSize::SmallInput,
        )
    });
}

fn write_queue(path: &std::path::Path, rows: usize, terminal_every: usize) {
    let mut file = std::fs::File::create(path).expect("queue");
    for i in 0..rows {
        let status = if terminal_every > 0 && i % terminal_every == 0 {
            "completed"
        } else {
            "pending"
        };
        writeln!(
            file,
            r#"{{"id":"bench_{i}","status":"{status}","owner":"warden","task_type":"monitor","queued_at_utc":"2026-01-01T00:00:00Z","completed_at_utc":"2026-01-01T00:00:01Z"}}"#
        )
        .expect("write row");
    }
}

fn bench_autopilot_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("autopilot_queue");

    for rows in [10_000usize, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("analyze_jsonl", rows),
            &rows,
            |b, &rows| {
                b.iter_batched(
                    || {
                        let dir = tempdir().expect("tempdir");
                        let queue = dir.path().join("queue.jsonl");
                        write_queue(&queue, rows, 10);
                        (dir, queue)
                    },
                    |(_dir, queue)| TaskQueueAnalyzer::new(queue).analyze(),
                    BatchSize::LargeInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("outcome_tail_first_pass", rows),
            &rows,
            |b, &rows| {
                b.iter_batched(
                    || {
                        let dir = tempdir().expect("tempdir");
                        let queue = dir.path().join("queue.jsonl");
                        write_queue(&queue, rows, 10);
                        let cursor = dir.path().join("cursor.json");
                        let mut registry = AgentRegistry::new();
                        registry.register(AgentCapabilities {
                            agent_id: "warden".into(),
                            task_types: vec!["monitor".into()],
                            max_concurrent: rows,
                            current_load: 0,
                            success_rate: 1.0,
                        });
                        (dir, queue, cursor, registry, LearningState::default())
                    },
                    |(_dir, queue, cursor, mut registry, mut learning)| {
                        OutcomeObserver::new(cursor).ingest(&queue, &mut registry, &mut learning)
                    },
                    BatchSize::LargeInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("outcome_tail_incremental", rows),
            &rows,
            |b, &rows| {
                b.iter_batched(
                    || {
                        let dir = tempdir().expect("tempdir");
                        let queue = dir.path().join("queue.jsonl");
                        write_queue(&queue, rows, 10);
                        let cursor = dir.path().join("cursor.json");
                        let mut registry = AgentRegistry::new();
                        registry.register(AgentCapabilities {
                            agent_id: "warden".into(),
                            task_types: vec!["monitor".into()],
                            max_concurrent: rows,
                            current_load: 0,
                            success_rate: 1.0,
                        });
                        let mut learning = LearningState::default();
                        let observer = OutcomeObserver::new(&cursor);
                        observer.ingest(&queue, &mut registry, &mut learning);
                        let mut file = std::fs::OpenOptions::new()
                            .append(true)
                            .open(&queue)
                            .expect("append");
                        writeln!(
                            file,
                            r#"{{"id":"bench_appended","status":"completed","owner":"warden","task_type":"monitor"}}"#
                        )
                        .expect("append row");
                        (dir, queue, cursor, registry, learning)
                    },
                    |(_dir, queue, cursor, mut registry, mut learning)| {
                        OutcomeObserver::new(cursor).ingest(&queue, &mut registry, &mut learning)
                    },
                    BatchSize::LargeInput,
                )
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_heartbeat_mode_selection,
    bench_order_store,
    bench_escalation_resolve,
    bench_autopilot_queue,
);
criterion_main!(benches);
