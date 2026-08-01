---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-28"
---

# arda-aule
Consolidated observability and operator-control crate for Arda.
Owner: arda | Sigil: 🜃 ANKH | Status: active

## Summary
`arda-aule` is the observability-area home for:
- `contract`, `service`, and `council` — stable observability contracts and compatibility types
- `governance_metrics` — typed governance snapshot rendering
- `telemetry` — feature-gated tracing and structured event wiring
- `ceo` — core autonomy profile, pipeline, and router primitives under `full-cli`
- `prometheus` — projections, council gate, heartbeat, orders, planning, registry, service,
  thought ledger, autopilot, execution intents, IPC, and optional HTTP transport under `full-cli`
- `arda-cli` — one supported binary containing governance, Plutus, Prometheus, and CEO-autopilot
  commands
- `arda-cli metrics` — the `http`-gated projection exporter and one-shot snapshot surface for
  queue, autonomy, pressure, audit-health, provider-budget, and optional node metrics

Provider and fleet routing are owned by Manwe. Aule records durable routing intents and does not
duplicate Manwe's provider-selection implementation.

## Where things are today
- Arda workspace member: `crates/spine/observability/arda-aule`
- Active library surfaces: the default modules plus `src/ceo/` and every module declared by
  `src/prometheus/mod.rs` under `full-cli`
- Active operator binary: `src/cli/main.rs`
- `full-cli` enables the consolidated CEO/Prometheus graph and `arda-cli`
- `http` adds the optional Prometheus HTTP transport; IPC is available with `full-cli`

## Exact Rust source classification

Inventory date: 2026-07-28. Total: 85 Rust files.

### Production/default (5)

- `src/lib.rs`
- `src/contract.rs`
- `src/council.rs`
- `src/governance_metrics.rs`
- `src/service.rs`

### Production/feature-gated (72)

`telemetry` (4):

- `src/telemetry/mod.rs`
- `src/telemetry/config.rs`
- `src/telemetry/events.rs`
- `src/telemetry/tracer.rs`

`full-cli` (66):

- `src/ceo/mod.rs`, `src/ceo/core_link.rs`, `src/ceo/pipeline.rs`, `src/ceo/router.rs`
- `src/cli/main.rs`
- `src/prometheus/mod.rs`
- `src/prometheus/autopilot/{mod,a2h,bootstrap,core_executor_bridge,dashboard,decomposer,delegation,evidence_registry,governance_policy,knowledge_triage,learning,oracle_gate,outcomes,pipeline_bridge,planner,queue_operation,queue_writer,reporting,runner,service_health,source_registry,task_queue,taxonomy,validator}.rs`
- `src/prometheus/core_link.rs`
- `src/prometheus/core_link/{arda,chronos,fleet,governance_runtime,hermes_command,human_context,io,memory,operations_flow,operator_actions,package_enablement,paperclip,snapshot,soterion,storage_pressure,support,topology,warden}.rs`
- `src/prometheus/{council,error,heartbeat,orders,planner,queue_authority,registry,router,service,thought}.rs`
- `src/prometheus/service/{drift,execution_intents,runtime,status,support}.rs`
- `src/prometheus/transport/{mod,ipc}.rs`

`http` (2; `http` implies `full-cli`):

- `src/cli/metrics_exporter.rs`
- `src/prometheus/transport/http.rs`

### Generated include (0)

No `include!` or generated Rust source is present.

### Test-only (0)

No standalone file under `src/` is reachable only through `cfg(test)`. Inline unit-test modules remain classified with their production host files.

### Integration test/build script (8)

- `tests/autopilot_surface.rs`
- `tests/ceo_surface.rs`
- `tests/contract_smoke.rs`
- `tests/council_surface.rs`
- `tests/council_surface_full.rs`
- `tests/governance_cli.rs`
- `tests/metrics_exporter_cli.rs`
- `tests/telemetry_surface.rs`

There is no `build.rs`.

### Unwired (0)

Every Rust source is reachable from `src/lib.rs`, the `arda-cli` binary root, or a declared integration-test target. There are no simultaneous `foo.rs` and `foo/mod.rs` roots.

## Feature and dependency graph

- Default features are empty and compile only the five default production files.
- `telemetry` activates the OpenTelemetry/OTLP/tracing dependency closure and four telemetry files.
- `full-cli` activates `arda-core`, `arda-vaire`, `arda-economics`, `arda-orome`, `arda-mandos`, `arda-varda`, and `telemetry`; it compiles CEO, Prometheus, and `arda-cli`.
- `http` activates `full-cli`, Axum, Tower, Tokio Stream, Sysinfo, HTTP transport, node metrics, and `metrics_exporter.rs`.
- `metrics_exporter_cli` is explicitly declared with `required-features = ["http"]`.

The only direct Cargo consumer is Manwe: its `telemetry` feature enables `arda-aule/telemetry`, and its startup/service-event paths call that public module.

## Verification status

- `cargo fmt -p arda-aule -- --check`: passing as of 2026-07-28.
- `cargo check -p arda-aule --no-default-features`: passing.
- `cargo test -p arda-aule --no-default-features -- --test-threads=1`: 5 unit/integration tests plus 2 doctests passed.
- `cargo check -p arda-aule --all-targets --all-features`: passing.
- `cargo test -p arda-aule --all-features -- --test-threads=1`: 187 unit/integration tests plus 2 doctests passed.
- Process-level governance and metrics-exporter tests validate JSON and Prometheus contracts.
- Strict Clippy, Rustdoc, and Manwe telemetry consumer gates are maintained in `STATUS.md`.

## Decisions
- Keep observability contracts, governance metrics, telemetry, and the operator binary in `arda-aule`.
- Keep `src/council.rs` as the canonical supported compatibility surface.
- Do not expose command variants whose runtime implementation is absent.
- Keep provider routing in Manwe and task execution in the active core loop/executor; Aule owns
  governance, observability, autopilot coordination, and durable queue/intent production.
- Preserve the currently consumed `annunimas_*` metric names as an external monitoring ABI until
  dashboards, alert rules, runtime consumers, and scrape jobs migrate together; do not add new
  legacy-named series.

## Archived foundation records

- [`PROMETHEUS.md`](../../../../docs/archive/PROMETHEUS.md)
- [`BASELINE.md`](../../../../docs/archive/arda-aule/BASELINE.md)
- [`IMPROVEMENT_PLAN.md`](../../../../docs/archive/arda-aule/IMPROVEMENT_PLAN.md)
- [`DEPENDENCY_AUDIT.md`](../../../../docs/archive/arda-aule/DEPENDENCY_AUDIT.md)
- [`STEP6_HANDOFF.md`](../../../../docs/archive/arda-aule/STEP6_HANDOFF.md)
