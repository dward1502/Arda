// sigil: REPAIR
//
// Criterion benchmarks for HADES hot paths.
//
// Run:  cargo bench -p ardas-hades
// Report: target/criterion/report/index.html

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::fs;
use std::path::Path;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Create a temp HADES service rooted in a tempdir with no PLUTUS env pollution.
fn fresh_service() -> (tempfile::TempDir, arda_hades::HadesService) {
    let dir = tempfile::tempdir().expect("tempdir");
    // Prevent PLUTOUS background work from panicking in bench context.
    let plutus_home = dir.path().join("plutus");
    std::fs::create_dir_all(&plutus_home).ok();
    std::env::set_var("arda_PLUTUS_HOME", &plutus_home);
    let svc = arda_hades::HadesService::new(dir.path()).expect("hades service");
    (dir, svc)
}

/// Write `n` files under `base` with various sigil headers and junk content.
fn populate_watch_dir(base: &Path, n: usize) {
    let sigils = [
        "---\nsigil: ANKH\n---\n",
        "---\nsigil: EYE\n---\n",
        &format!("{{\"sigil\": \"SCROLL\", \"id\": {}}}\n", 0),
        "---\nsigil: COIN\n---\n",
        "---\nsigil: CONDEMNED\n---\n",
        "# sigil: ANKH\n",
        "// sigil: EYE\n",
        "/* sigil: REPAIR */\n",
        "sigil: OrphanTemp\n",
        "sigil: Quarantine\n",
    ];
    for i in 0..n {
        let dir = base.join(format!("subdir_{}", i % 10));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("file_{:06}.txt", i));
        let header = &sigils[i % sigils.len()];
        let body = format!("{}{}\n", header, "x".repeat(256));
        fs::write(&file, &body).unwrap();
    }
}

/// Write `n` orphan files (no sigil header) under `base`.
fn populate_orphans(base: &Path, n: usize) {
    for i in 0..n {
        let dir = base.join(format!("orphan_subdir_{}", i % 5));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("orphan_{:06}.txt", i));
        fs::write(&file, format!("no sigil here just content {}\n", i)).unwrap();
    }
}

/// Write `n` action queue entries (mixed kinds).
fn populate_queue(dir: &Path, n: usize) {
    use arda_hades::{ActionKind, QuorumProof, TaskItem};
    let kinds = [ActionKind::InvestigateOrphan, ActionKind::Remove];
    let queue_file = dir.join("action_queue.jsonl");
    for i in 0..n {
        let task = TaskItem {
            task_id: format!("hds_bench_{}", i),
            queued_at_utc: chrono::Utc::now().to_rfc3339(),
            action: kinds[i % kinds.len()].clone(),
            file: format!("/tmp/bench_file_{}.txt", i),
            authorized_by: Some("orchestrator".to_string()),
            reason: "benchmark".to_string(),
            execute_after_utc: None,
            quorum_proof: if i % kinds.len() == 1 {
                Some(QuorumProof {
                    approvers: vec!["aurelius".to_string(), "bacon".to_string()],
                    evidence: vec!["bench:1".to_string()],
                    asserted_at_utc: None,
                })
            } else {
                None
            },
        };
        let line = serde_json::to_string(&task).unwrap();
        fs::write(&queue_file, format!("{}\n", line)).unwrap();
    }
    // Actually append properly
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&queue_file)
            .unwrap();
        for i in 0..n {
            let task = TaskItem {
                task_id: format!("hds_bench_{}", i),
                queued_at_utc: chrono::Utc::now().to_rfc3339(),
                action: kinds[i % kinds.len()].clone(),
                file: format!("/tmp/bench_file_{}.txt", i),
                authorized_by: Some("orchestrator".to_string()),
                reason: "benchmark".to_string(),
                execute_after_utc: None,
                quorum_proof: if i % kinds.len() == 1 {
                    Some(QuorumProof {
                        approvers: vec!["aurelius".to_string(), "bacon".to_string()],
                        evidence: vec!["bench:1".to_string()],
                        asserted_at_utc: None,
                    })
                } else {
                    None
                },
            };
            writeln!(f, "{}", serde_json::to_string(&task).unwrap()).unwrap();
        }
    }
}

/// Write a world.json so memory_referenced() has data to scan.
fn write_world_state(hades_dir: &Path, n_references: usize) {
    // HADES looks at core/state/world.json by default, but we override via
    // arda_WORLD_STATE_PATH. Write a file with many entries.
    let world_path = hades_dir.join("world.json");
    let refs: Vec<String> = (0..n_references)
        .map(|i| format!("\"/tmp/ref_file_{}.txt\"", i))
        .collect();
    let content = format!(r#"{{"files": [{}]}}"#, refs.join(","));
    fs::write(&world_path, &content).unwrap();
    std::env::set_var("ARDA_WORLD_STATE_PATH", &world_path);
}

// ── bench: sweep_scaled ──────────────────────────────────────────────────────

fn bench_sweep_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("sweep_scaled");
    group.throughput(Throughput::Elements(100));

    group.bench_function("sweep_100_files", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("watch");
        fs::create_dir_all(&watch).unwrap();
        populate_watch_dir(&watch, 100);

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.finish();
}

fn bench_sweep_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sweep_scaled");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("sweep_1000_files", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("watch_1k");
        fs::create_dir_all(&watch).unwrap();
        populate_watch_dir(&watch, 1000);

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.finish();
}

fn bench_sweep_5000(c: &mut Criterion) {
    let mut group = c.benchmark_group("sweep_scaled");
    group.throughput(Throughput::Elements(5000));

    group.bench_function("sweep_5000_files", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("watch_5k");
        fs::create_dir_all(&watch).unwrap();
        populate_watch_dir(&watch, 5000);

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.finish();
}

// ── bench: orphan_discovery ──────────────────────────────────────────────────

fn bench_orphan_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("orphan_discovery");

    for &n in &[100, 500, 1000] {
        group.throughput(Throughput::Elements(n));
        group.bench_function(format!("orphan_{}_files", n), |b| {
            let (_dir, svc) = fresh_service();
            let watch = _dir.path().join(format!("orphan_watch_{}", n));
            fs::create_dir_all(&watch).unwrap();
            populate_orphans(&watch, n as usize);

            b.iter(|| {
                let _ = svc.sweep(
                    black_box("bench"),
                    black_box(Some(&watch.display().to_string())),
                );
            });
        });
    }

    group.finish();
}

// ── bench: queue_operations ──────────────────────────────────────────────────

fn bench_queue_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_operations");

    for &n in &[100, 1000, 5000] {
        group.throughput(Throughput::Elements(n));
        group.bench_function(format!("queue_read_{}", n), |b| {
            let (_dir, svc) = fresh_service();
            populate_queue(_dir.path(), n as usize);

            b.iter(|| {
                black_box(svc.queue(black_box(10_000)).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_queue_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_operations");

    group.bench_function("queue_append_single", |b| {
        let (_dir, svc) = fresh_service();

        b.iter(|| {
            let _ = svc.queue_remove(black_box("/tmp/bench_nonexistent.txt"), black_box("bench"));
        });
    });

    group.finish();
}

// ── bench: status ────────────────────────────────────────────────────────────

fn bench_status(c: &mut Criterion) {
    let mut group = c.benchmark_group("status");

    for &n_queue in &[0, 100, 1000] {
        group.bench_function(format!("status_queue_{}", n_queue), |b| {
            let (_dir, svc) = fresh_service();
            populate_queue(_dir.path(), n_queue);
            write_world_state(_dir.path(), 100);

            b.iter(|| {
                black_box(svc.status().unwrap());
            });
        });
    }

    group.finish();
}

// ── bench: sigil_parsing ─────────────────────────────────────────────────────

fn bench_sigil_parse_micro(c: &mut Criterion) {
    let mut group = c.benchmark_group("sigil_parse_micro");

    // Benchmark the read_sigil path indirectly: create a file with a known
    // sigil then sweep a single-file directory many times.
    group.bench_function("sigil_json", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("sig_watch");
        fs::create_dir_all(&watch).unwrap();
        fs::write(watch.join("a.json"), r#"{"sigil": "ANKH", "data": "x"}"#).unwrap();

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.bench_function("sigil_frontmatter", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("fm_watch");
        fs::create_dir_all(&watch).unwrap();
        fs::write(
            watch.join("a.md"),
            "---\nsigil: CONDEMNED\n---\ncontent here\n",
        )
        .unwrap();

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.bench_function("sigil_comment_rust", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("cm_watch");
        fs::create_dir_all(&watch).unwrap();
        fs::write(watch.join("a.rs"), "// sigil: REPAIR\nfn main() {}\n").unwrap();

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.bench_function("no_sigil_early_exit", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("no_sig_watch");
        fs::create_dir_all(&watch).unwrap();
        fs::write(
            watch.join("a.txt"),
            "this file has no sigil header at all\n",
        )
        .unwrap();

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.finish();
}

// ── bench: world_state_memory_check ──────────────────────────────────────────

fn bench_memory_referenced(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_check");

    for &n_refs in &[100, 1000, 10_000] {
        group.bench_function(format!("world_state_{}_refs", n_refs), |b| {
            let (_dir, svc) = fresh_service();
            write_world_state(_dir.path(), n_refs as usize);

            b.iter(|| {
                black_box(svc.status().unwrap());
            });
        });
    }

    group.finish();
}

// ── bench: path_filtering ────────────────────────────────────────────────────

fn bench_path_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_filtering");

    // Create many files (mix of skip and non-skip paths) to exercise
    // should_skip_watch_file during sweep.
    group.bench_function("mixed_skip_1000", |b| {
        let (_dir, svc) = fresh_service();
        let watch = _dir.path().join("mixed_watch");
        fs::create_dir_all(&watch).unwrap();

        // Normal files
        populate_watch_dir(&watch, 500);

        // Files in directories that should be skipped
        for subdir in &["target/debug", "core/state", ".git/objects", "tmp/junk"] {
            let p = watch.join(subdir);
            fs::create_dir_all(&p).unwrap();
            for i in 0..125 {
                fs::write(p.join(format!("file_{}.txt", i)), "skip me\n").unwrap();
            }
        }

        b.iter(|| {
            let _ = svc.sweep(
                black_box("bench"),
                black_box(Some(&watch.display().to_string())),
            );
        });
    });

    group.finish();
}

// ── bench: lifecycle_decision ────────────────────────────────────────────────

fn bench_lifecycle_decision(c: &mut Criterion) {
    let mut group = c.benchmark_group("lifecycle_decision");

    for scope_name in &[
        "human_context",
        "boardroom_council",
        "edge_runtime",
        "system_continuity",
    ] {
        group.bench_function(format!("lifecycle_{}", scope_name), |b| {
            let (_dir, svc) = fresh_service();
            let watch = _dir.path().join("lc_watch");
            fs::create_dir_all(&watch).unwrap();

            let subdir = match *scope_name {
                "human_context" => "human",
                "boardroom_council" => "data/hermes/boardroom",
                "edge_runtime" => "edge/runtime",
                _ => "core/docs",
            };
            let target_dir = watch.join(subdir);
            fs::create_dir_all(&target_dir).unwrap();
            populate_watch_dir(&target_dir, 100);

            b.iter(|| {
                let _ = svc.sweep(
                    black_box("bench"),
                    black_box(Some(&watch.display().to_string())),
                );
            });
        });
    }

    group.finish();
}

// ── criterion groups ────────────────────────────────────────────────────────

criterion_group!(
    sweep_benches,
    bench_sweep_100,
    bench_sweep_1000,
    bench_sweep_5000,
);
criterion_group!(orphan_benches, bench_orphan_discovery,);
criterion_group!(queue_benches, bench_queue_read, bench_queue_append,);
criterion_group!(status_benches, bench_status);
criterion_group!(sigil_benches, bench_sigil_parse_micro);
criterion_group!(memory_benches, bench_memory_referenced);
criterion_group!(path_benches, bench_path_filtering);
criterion_group!(lifecycle_benches, bench_lifecycle_decision);

criterion_main!(
    sweep_benches,
    orphan_benches,
    queue_benches,
    status_benches,
    sigil_benches,
    memory_benches,
    path_benches,
    lifecycle_benches,
);
