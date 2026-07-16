---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  code_point: "U+1F50D"
  role: index
  owner: "HADES"
  status: active
  reviewed: "2026-07-16"
---

# Arda Repo Index

Evidence basis: `src/main.rs`, `cargo metadata --format-version=1 --no-deps`,
`Cargo.toml` manifests, and representative source inspection.

Surface-by-surface inventory:

- `arda` shell -> `arda-engine` facade surface:
  - Design contract: keep the daemon binary and top-level `arda_engine` imports
    independent from any single app shell.
  - Surface areas: CLI flags, `boot()`, `Registry::load("services.toml")`, HW/SW readiness
    surfaces, shutdown/supervision behavior.
  - `arda-engine` remains the single facade for the daemon; `arda-launcher` app logic
    should not reach into engine internals without explicit controller wiring.
- `arda-launcher` -> `arda-core` app path:
  - Design contract: Tauri app shell consumes `arda-core` types/config only.
  - Surface areas: onboarding pipeline, environment profile, device scan, provider
    checklist, readiness projection, private config staging/apply, guided session,
    service plan, receipt/apply flow, build script (`build.rs`).
  - No `arda-engine`/harness dependency present in inspected code.
- `arda-varda` ingestion surface:
  - Design contract: executor crate owns crawl/deep/extraction/github/scholarly
    pipeline surfaces in `src/ingest/*.rs` and `src/*.rs`.
  - Surface areas: ingest/remediation/policy/uncertainty-sampling routers, metrics,
    interceptor hooks, test/background learning modules.
- `arda-orome` comms surface:
  - Design contract: bridge crate for Hermes/boardroom comms/mcp surfaces.
  - Surface areas: inbound/outbound queues, boardroom posts, council discussions,
    decision prompts/execution, task approvals, interruptions, semantic channel,
    Discord projections, status surfaces, MCP server/channel tools,
    `PlutusService` usage via `arda-economics`.
