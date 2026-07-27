---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-27"
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

## Verification status
- `cargo check -p arda-aule --all-targets --all-features`: passing as of 2026-07-27
- `cargo test -p arda-aule --all-features --lib --tests`: 185 tests passed
- `cargo test -p arda-aule --all-features --doc`: 2 doctests passed
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- `cargo fmt -p arda-aule -- --check`: passing
- Process-level tests execute governance and metrics-exporter commands and validate JSON and
  Prometheus exposition contracts.

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
