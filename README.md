# Arda

> A local-first, phone-accessible, governed personal agent ecosystem.

Arda helps one sovereign operator organize personal life and projects, preserve context, coordinate hosted and local workers, integrate external systems, and act proactively without taking unearned authority. Workbench, Personal Operations, research, council, economic tools, and device/outpost integrations are composable capabilities over one task, memory, governance, communications, and receipt model—not separate product identities. The canonical Rust workspace lives in this repository; `~/Annunimas` is reference architecture and should not be modified unless explicitly requested.

The current product authority is the [Arda 0.9 Baseline](docs/releases/0.9/BASELINE.md). The broader [Arda 1.0 Personal Agent Ecosystem](docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md) remains future doctrine rather than active 0.9 execution authority.

## Current workspace status

- `cargo metadata --no-deps --format-version 1` resolves 18 current workspace
  packages (16 default members) as of 2026-08-08.
- The root binary is `arda`; the workspace has no `arda-council` package.
- Installed services and listeners are runtime evidence, not Cargo topology or
  release support. Use `ARDA_SYSTEM_STATUS_REPORT.md` for the latest bounded
  snapshot and re-probe before operational decisions.
- `arda` composition-root entrypoint: `src/main.rs`
- Service supervision spine: `crates/engine` (`arda-engine`)
- Observability/CLI surface: `crates/spine/observability/arda-aule`
- Reference/legacy sources remain in `~/Annunimas`; migrated sources in this repo are source-of-truth.

## Repository Map

| Path | Kind | Purpose |
|---|---|---|
| `src/main.rs` | Rust binary | Composition root: discovers the repo, resolves `services.toml`, projects Warden/fleet state, starts the harness and supervisor, and coordinates shutdown |
| `crates/engine` | Rust library | Service supervision spine: registry, supervisor, harness, manwe bridge |
| `crates/spine/governance/arda-core` | Rust library | Core types, config, ledger, tasks, LLM provider/routing, systemd client |
| `crates/spine/governance/arda-governance` | Rust library | Governance, triad validation, resonance, philosopher profiles |
| `crates/spine/runtime/arda-economics` | Rust library | JouleWork and resource accounting: measured/default costs, confidence, status, ledger/service transport |
| `crates/spine/runtime/arda-mandos` | Rust library | Oracle/runtime: reasoning, verdicts, scoring, transport |
| `crates/spine/memory/arda-vaire` | Rust library | Memory service: informant events, transport |
| `crates/spine/runtime/manwe` | Rust library | Single governed model/provider runtime, route policy, capability state, and adaptive routing |
| `crates/spine/interface/arda-orome` | Rust library | Comms/bridge: A2H/A2A message types |
| `crates/spine/executors/arda-varda` | Rust library | Athena agent + ingest/query/deep-analysis + HTTP transport |
| `crates/spine/observability/arda-aule` | Rust library + CLI | Prometheus/CEO autopilot, CLI, observability surfaces |
| `apps/arda-launcher` | Tauri app | Operator desktop launcher |
| `config/` | Config | Operator-managed config and generated runtime env files |
| `docs/` | Docs | Architecture, operations, plans, identity docs |

## Recommended Reading Order

1. This file (`README.md`).
2. `AGENTS.md` — working rules and canonical source layout.
3. `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` — product doctrine and capability-composition model.
4. `docs/plans/2026-08-12-arda-0.9-baseline-and-improvement-plan.md` — finite active improvement and evidence frontier.
5. `docs/root-daemon.md` — root package status, composition boundary, ownership, and verification.
6. `crates/engine/README.md` and `crates/engine/BREAKDOWN.md` — registry,
   harness, and supervisor implementation.
7. `apps/arda-launcher/README.md` — what the launcher is and how to run it.

## Verification

- Package gates: `cargo test -p arda --all-features -- --test-threads=1`
- Maintained daemon smoke: `cargo build -p arda && ./target/debug/arda --once --no-ui`
- Workspace check: `cargo check --workspace --all-targets --all-features`
- Workspace Clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
