# Arda — Remaining integration work (no-code audit)

This is a no-implementation plan. It lists what is still incomplete across
the Rust spine crates and the two desktop apps, in priority order.

Evidence notes are derived from live repo state at audit time.

## 0. Repo-root services manifest

- [x] Repo-root `services.toml` exists and is loadable by `arda-engine`.
- [x] `cargo check -p arda-engine` validates manifest parsing/runtime wiring.
- [x] Acceptance: `cargo run -- --once` from repo root boots arda-engine and
      exits cleanly ("arda daemon: --once set, exiting after boot").

Note: `src/main.rs` only runs supervision when `services.toml` exists.
Created `services.toml` with `arda-launcher`, `arda-hud`, and `manwe` entries.

## 1. arda-launcher ↔ manwe handshake

- [x] After onboarding completes, launcher persists operator profile to
      `config/` and surfaces live `manwe_url`.
- [ ] Launcher can start/pin the daemon or verify `arda` boot state.
- [ ] Operator profile wiring into `arda-core` task/governance inputs is
      represented in Rust/Tauri bridge code.

Note: `apps/arda-launcher/src-tauri/src/onboarding/console.rs` now writes an
`onboarding_console_state` receipt under `audit/onboarding-runs/` that includes
operator profile, selected providers, `manwe_base_url`, and canonical paths.

## 2. arda-hud data-source contract

- [x] HUD status files (`core/state/plutus_runtime.json` and
      `core/state/operator_runtime_status.json`) are present.
- [x] Path normalization code is verified to tolerate canonical absolute or
      local relative path shapes.
- [ ] Refresh commands for Plutus/Oracle/Apollo projections are wired to a
      running `arda-aule` process.
- [x] HUD has offline/fallback UI for missing manwe/Charon/Hermes endpoints.

## 3. manwe gateway runtime wiring

- [x] Provider routing config files exist under `config/routing/*.toml`.
- [ ] Provider hydration completes from `.env` and `config/routing/*.toml`.
- [ ] Runtime health/model-listing endpoints are exercised by HUD.
- [ ] Graceful fallback behavior is implemented when no provider resolves.

Note: HUD does not yet exercise manwe `/healthz` and `/v1/models` directly.
`arda-aule serve` is currently a stub, so none of the three items are
executable until a runtime gateway path exists.

## 4. arda-core governance activation

- [x] Evidence exists that `arda-governance` is wired into
      `arda-economics`/`arda-mandos` beyond type imports.
- [ ] Joulework/governance history append path has live end-to-end
      validation.

Note: `arda-economics` persists governance records on service events; `arda-mandos`
evaluates governance via triad/BaconLite/resonance. `cargo test -p arda-governance
-p arda-economics -p arda-mandos` passes.

## 5. Observability / export surface completeness

- [x] Runtime/fleet/prometheus export paths are exercised by a running
      Prometheus scrape path.
- [ ] Core-state projection trigger (`plutus_runtime.json`,
      `oracle_runtime.json`, `apollo_runtime.json`) is documented in code.

Note: `arda-aule` compiles and already references `plutus_runtime`,
`oracle_runtime`, and `apollo_runtime` in `cli/support.rs`/`cli/observability.rs`.
Full live Prometheus scrape-path coverage remains future validation.

## Audit evidence

- [x] Repo-root `services.toml` exists.
- [x] `core/state/plutus_runtime.json` is present.
- [x] `core/state/operator_runtime_status.json` is present.
- [x] `data/plutus/runtime_status.json` is present.
- [x] `config/routing/*.toml` provider config files are present.
- [x] Manwe crate health-route symbols were found in audit.
- [x] 25 HUD TS/TSX/JSON files reference runtime status or manwe.
- [x] Confirmed: zero backend launcher Rust references to `manwe`/`manwe_url` found.
- [x] Launcher now persists operator profile + `manwe_base_url` in `audit/onboarding-runs/` via `onboarding_console_state`.
- [x] HUD path normalization tolerates canonical absolute/relative paths.
- [x] HUD offline/fallback UI present for missing live endpoints.

## Recommended execution order

1. Repo-root `services.toml` — unlocks daemon supervision immediately.
2. manwe provider hydration + health endpoint — unblocks HUD live data.
3. launcher post-onboarding handshake — unblocks operator startup flow.
4. HUD offline/fallback + canonical path normalization polish.
5. Governance/joulework live-loop validation.
