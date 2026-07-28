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

# Index: `crates/engine`

The single dependency surface the `arda` daemon uses to reach system services.
Re-exports the core spine so callers import from `arda_engine` rather than
reaching into vendored crates directly.

- `Cargo.toml` — package metadata and dependency graph.
- `README.md` — public/runtime contract and operator entry point.
- `STATUS.md` — current first-class verification evidence.
- `BREAKDOWN.md` — exhaustive source and dependency classification.
- `OWNERSHIP.md` — engine versus root-daemon authority boundary.
- `INDEX.md` — this direct-child map.
- `src/`
  - `lib.rs` — supported modules and spine re-exports.
  - `harness.rs` — local operator HTTP surface and bounded proxies.
  - `manwe.rs` — supported Manwe re-exports.
  - `observability.rs` — aggregate engine loop/learning projection.
  - `orome.rs` — provider/runtime compatibility surface.
  - `registry.rs` — declarative service loading and resolution.
  - `supervisor.rs` — child lifecycle, restart/backoff, and shutdown.
- `tests/`
  - `orome_smoke.rs` — direct provider dispatch integration smoke.

## Purpose (one line)
Process supervision spine for the Arda daemon — owns child processes (launcher,
HUD, `manwe` gateway) and keeps them alive.

## Silmarillion rename
Stays `arda-engine` (Arda-native, not an annunimas import).

## Current status
- Part of the verified `arda-*` workspace.
- Used by the root `arda` daemon for service resolution, supervision, supported
  provider integration, and harness serving.
