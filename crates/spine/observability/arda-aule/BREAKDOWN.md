---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-25"
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
- `cargo check -p arda-aule --features full-cli --all-targets`: passing as of 2026-07-25
- `cargo test -p arda-aule`: passing
- `cargo test -p arda-aule --features full-cli --lib --tests`: passing
- `cargo test -p arda-aule --all-features --lib --tests`: passing serially
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- Process-level tests execute `governance-metrics` and `governance-status` and validate JSON contracts.

## Decisions
- Keep observability contracts, governance metrics, telemetry, and the operator binary in `arda-aule`.
- Keep `src/council.rs` as the canonical supported compatibility surface.
- Do not expose command variants whose runtime implementation is absent.
- Keep provider routing in Manwe and task execution in the active core loop/executor; Aule owns
  governance, observability, autopilot coordination, and durable queue/intent production.
