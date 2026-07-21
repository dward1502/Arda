# Arda

> The slimmed-down continuation of Annunimas: a local-first, auditable agent control plane.

Arda is the operator-facing shell that evolved from Annunimas. It keeps the same control-plane purpose — routing model calls, coordinating agents, recording decisions, gating autonomy, and visualizing operational state — but refactored into a smaller, more composable `arda-*` crate layout. The canonical Rust workspace lives in this repo; `~/Annunimas` remains the live reference architecture and should not be modified unless explicitly requested.

## Current workspace status

- Verified with `cargo check --workspace` on the current `arda-*` crate set.
- `arda` daemon entrypoint: `src/main.rs`
- Service supervision spine: `crates/engine` (`arda-engine`)
- Observability/CLI surface: `crates/spine/observability/arda-aule`
- Reference/legacy sources remain in `~/Annunimas`; migrated sources in this repo are source-of-truth.

## Repository Map

| Path | Kind | Purpose |
|---|---|---|
| `src/main.rs` | Rust binary | `arda` daemon entrypoint; boots `arda-engine`, supervises services from `services.toml` |
| `crates/engine` | Rust library | Service supervision spine: registry, supervisor, harness, manwe bridge |
| `crates/spine/governance/arda-core` | Rust library | Core types, config, ledger, tasks, LLM provider/routing, systemd client |
| `crates/spine/governance/arda-governance` | Rust library | Governance, triad validation, resonance, philosopher profiles |
| `crates/spine/runtime/arda-economics` | Rust library | Plutus model: economics, joule work, cost/ledger/service transport |
| `crates/spine/runtime/arda-mandos` | Rust library | Oracle/runtime: reasoning, verdicts, scoring, transport |
| `crates/spine/memory/arda-vaire` | Rust library | Memory service: informant events, transport |
| `crates/spine/runtime/manwe` | Rust library | Charon/manwe gateway types and routing adapter |
| `crates/spine/interface/arda-orome` | Rust library | Comms/bridge: A2H/A2A message types |
| `crates/spine/executors/arda-varda` | Rust library | Athena agent + ingest/query/deep-analysis + HTTP transport |
| `crates/spine/observability/arda-aule` | Rust library + CLI | Prometheus/CEO autopilot, CLI, observability surfaces |
| `apps/arda-launcher` | Tauri app | Operator desktop launcher |
| `config/` | Config | Operator-managed config and generated runtime env files |
| `docs/` | Docs | Architecture, operations, plans, identity docs |

## Recommended Reading Order

1. This file (`README.md`).
2. `AGENTS.md` — working rules and canonical source layout.
3. `docs/identity/ARDA_IDENTITY.md` — Annunimas-to-Arda identity transfer and operating assumptions.
4. `crates/engine/README.md` / `BREAKDOWN.md` — how the daemon bootstraps and supervises services.
5. `apps/arda-launcher/README.md` — what the launcher is and how to run it.
6. `apps/arda-launcher/src-tauri/tauri.conf.json` — desktop packaging/config.

## Verification

- Workspace check: `cargo check --workspace`
- Daemon boot smoke path: `cargo run -p arda -- --once`
