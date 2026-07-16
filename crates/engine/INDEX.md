---
soterion:
  sigil: "SPINE"
  glyph: "🧬"
  role: "supervision_spine"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-13"
---

> 🧬 arda-engine: 🧬 supervision_spine | owner: arda | status: active | reviewed: 2026-07-13

# Index: crates/arda-engine

The single dependency surface the `arda` daemon uses to reach system services.
Re-exports the core spine so callers import from `arda_engine` rather than
reaching into vendored crates directly.

- `Cargo.toml`
- `src`
  - `lib.rs` — `boot()` placeholder + aliased re-exports (`core`, `onboarding`, `service_registry`)
  - `supervisor.rs` — `Supervisor` / `Service` / `Shutdown`: spawn, watch, restart
    with exponential backoff; clean shutdown via a shared `Notify`.

## Purpose (one line)
Process supervision spine for the Arda daemon — owns child processes (launcher,
HUD, `manwe` gateway) and keeps them alive.

## Silmarillion rename
Stays `arda-engine` (Arda-native, not an annunimas import).

## TODO
- Rename aliased crates (`annunimas_*` -> Arda-native names) once the
  targeted crate scope is decided; keep `arda_engine` stable until then.
