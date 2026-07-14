# Arda CHANGELOG

Durable record of feature changes and confirmed architecture decisions.
Entries are dated; newest first. Plan-index notes that are still DRAFT are
cross-linked, not authoritative.

## 2026-07-13 — Startup architecture: daemon = lifecycle, launcher = startup brain

Confirmed (verified against the repo, not assumed) the boundary between the
`arda` daemon, the `arda-launcher` app, and the `arda-hud` app.

### Decision
- `arda` daemon (`src/main.rs` + `crates/engine` lib) owns **process lifecycle
  only**: spawn launcher → observe "seeded" marker → HUD renders. It does NOT
  broker data and holds no domain logic.
- `arda-launcher` (separate Tauri app, `apps/arda-launcher/src-tauri/src/lib.rs`)
  owns the **startup brain**: recursive system/inference search, first-run
  config/onboarding, startup-tool suite. This logic lives in the launcher's own
  Rust backend — it is **app-end code, NOT in `src/main.rs`**.
- `arda-hud` (separate Tauri app) is a **dumb consumer**: reads seeded state and
  renders. It must NOT be spawned by the launcher; the daemon supervises it.
- Reusable capabilities the launcher needs are provided as libs it calls
  (e.g. `annunimas-onboarding`, already an `arda-engine` dep). No logic
  duplication between app orchestration and engine capability.
- Seeding handoff is a **shared-store / gateway (manwe, 7171) concern**, not the
  daemon relaying every message.

### Why
Putting launcher logic in `src/main.rs` would undo `arda-engine`'s
unit-testability (`cargo test -p arda-engine`) and break the thin-shell model.
Tauri apps are separate processes spawned via `services.toml`, so the daemon
cannot "load" the launcher as a module — it spawns it.

### Affected
- `src/main.rs` (stays thin shell)
- `crates/engine` (lib; provides onboarding + supervisor/harness)
- `apps/arda-launcher/src-tauri/src/lib.rs` (startup logic owner)
- `apps/arda-hud` (consumer)
- `docs/plans/spine-tooling-00-index.md` (mirrors this under "Confirmed
  architecture")

### Open follow-ups
- Define the exact "seeded/done" marker contract (store key or gateway event).
- Sequence in services.toml: launcher spawned before HUD reads state.
