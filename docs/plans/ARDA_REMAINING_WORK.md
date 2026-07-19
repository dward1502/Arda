# Arda — Remaining integration work (no-code audit)

This is a no-implementation plan. It lists what is still incomplete across
the Rust spine crates and the two desktop apps, in priority order.

## 0. Repo-root services manifest

`src/main.rs` only runs service supervision when a `services.toml` exists in
the repo root. That file is currently absent, which means daemon-mode
supervision is dead even though the Rust binaries compile.

Needed: create repo-root `services.toml` with entries for:

- `arda-launcher` — `apps/arda-launcher/src-tauri/target/debug/arda-launcher`
- `arda-hud` — optional UI service; same search pattern as above
- `manwe` gateway — either via `arda-aule` CLI or a manwe binary once it has
  a stable build target

Acceptance: `cargo run -- --once` from repo root skips no required service.

## 1. arda-launcher ↔ manwe handshake

Current state:
- `apps/arda-launcher/src-tauri/` is an onboarding shell; it does not yet
  talk to manwe/Arda core after first-run.

Missing pieces:
- After onboarding completes, launcher should persist operator profile to
  `config/` and surface the live `manwe_url`.
- Launcher currently cannot start/pin the daemon or verify `arda` boot
  state; either Tauri sidecar spawning or a socket/health probe is required.
- Operator profile wiring into `arda-core` task/governance inputs is not yet
  represented in Rust/Tauri bridge code.

## 2. arda-hud data-source contract

Current state:
- HUD consumes `core/state/operator_runtime_status.json` and
  `data/plutus/runtime_status.json` via TS adapters/projections.
- After the `spine/data` → `data/` move, absolute path metadata inside
  `data/plutus/runtime_status.json` was updated; HUD readers should be re-
  verified to normalize paths and tolerate either canonical absolute path or
  the relative local path shape.

Missing pieces:
- Surface refresh commands for Plutus/Oracle/Apollo projections are present
  but not wired to a running `arda`/`arda-aule` process.
- Offline/fallback UI for missing manwe/Charon/Hermes endpoints is still
  TODO per `arda-hud/BREAKDOWN.md`.

## 3. manwe gateway runtime wiring

Current state:
- `crates/spine/runtime/manwe/` builds; provider catalog exists with a
  functional `default_bootstrap()`.
- Routing docs/config expected at `config/routing/*.toml`; current files are
  present but wiring into manwe runtime is not verified end-to-end.

Missing pieces:
- Actual OpenAI-compatible/Anthropic provider hydration from `.env` and
  `config/routing/*.toml`.
- Runtime health/model listing endpoints exercised by HUD.
- Graceful fallback when no provider resolves.

## 4. arda-core governance activation

Current state:
- Triad/BaconLite/resonance modules compile and expose full APIs.
- `arda-core/src/loop_engine.rs` has scoring/skipping semantics but is not
  shown to be exercised by a live runtime loop.

Missing pieces:
- Evidence that `arda-governance` is wired into `arda-economics`/`arda-mandos`
  beyond type imports.
- Joulework/governance history append path needs live end-to-end validation.

## 5. Observability / export surface completeness

Current state:
- `arda-aule` CLI is feature-gated, not stubbed.
- Runtime exports, fleet exports, and prometheus core_link projections are
  present.

Open questions / gaps:
- Whether `prometheus-core_link/arda.rs` is actually invoked from a running
  Prometheus scrape path or only generated on demand.
- `plutus_runtime.json`, `oracle_runtime.json`, `apollo_runtime.json`
  core-state projection freshness depends on a trigger; that trigger is not
  documented in code.

## Recommended execution order

1. Repo-root `services.toml` — unlocks daemon supervision immediately.
2. manwe provider hydration + health endpoint — unblocks HUD live data.
3. launcher post-onboarding handshake — unblocks operator startup flow.
4. HUD offline/fallback + canonical path normalization polish.
5. Governance/joulework live-loop validation.
