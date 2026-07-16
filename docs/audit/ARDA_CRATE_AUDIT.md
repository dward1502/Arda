---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: audit
  owner: "HADES"
  status: active
  reviewed: "2026-07-16"
---

# Arda Crate Audit

Evidence basis: workspace `Cargo.toml`, `cargo metadata --format-version=1 --no-deps`,
and direct source inspection of representative implementation files.

## Workspace members

| Crate | Manifest |
|------|---------|
| arda | `Cargo.toml` | `arda` |
| arda-engine | `crates/engine/Cargo.toml` | `arda_engine` |
| arda-core | `crates/spine/governance/arda-core/Cargo.toml` | `arda_core`, `tool_harness_smoke` |
| manwe | `crates/spine/runtime/manwe/Cargo.toml` | `manwe` |
| arda-council | `crates/spine/governance/arda-council/Cargo.toml` | `arda_council`, `contract_smoke` |
| arda-governance | `crates/spine/governance/arda-governance/Cargo.toml` | `arda_governance`, `alignment_stack`, `philosopher_profiles` |
| arda-orome | `crates/spine/interface/arda-orome/Cargo.toml` | `arda_orome` |
| arda-economics | `crates/spine/runtime/arda-economics/Cargo.toml` | `arda_economics` |
| arda-mandos | `crates/spine/runtime/arda-mandos/Cargo.toml` | `arda_mandos`, `target_local` |
| arda-vaire | `crates/spine/memory/arda-vaire/Cargo.toml` | `arda_vaire`, `knowledge_deltas`, `public_flows` |
| arda-aule | `crates/spine/observability/arda-aule/Cargo.toml` | `arda_aule`, `contract_smoke` |
| arda-launcher | `apps/arda-launcher/src-tauri/Cargo.toml` | `arda_launcher_lib`, `arda-launcher` |
| arda-varda | `crates/spine/executors/arda-varda/Cargo.toml` | `arda_varda`, `learning_contract_test`, `local_harness` |
| arda-service-registry | `crates/spine/executors/arda-service-registry/Cargo.toml` | `arda_service_registry`, `contract_smoke` |

## Missing/retired from active closure

- `arda-athena`: retired by rename to `arda-varda`; remaining git-rename artifacts are on-disk only.
- `arda-plutus`: not present on disk; earlier metadata/migration docs still reference it.
- `arda-hades`: not present on disk; retired before this audit.
- `arda-prometheus`: not present on disk; retired/removed from active closure.
- `arda-onboarding`: retired and removed; `arda-launcher` owns the current canonical onboarding flow.

## Dependency graph

- `arda` -> `arda-engine`
- `arda-engine` -> `arda-core`, `manwe`, plus harness/supervisor/registry surfaces
- `arda-launcher` -> `arda-core`
- `arda-council` -> `arda-core`
- `arda-governance` -> `arda-core`
- `arda-economics` -> `arda-core`, `arda-governance`
- `arda-orome` -> `arda-core`, `arda-governance`, `arda-vaire`, `arda-mandos`, `arda-economics`
- `arda-mandos` -> `arda-core`, `arda-governance`, `arda-economics`
- `arda-vaire` -> `arda-core`, `arda-governance`, `arda-economics`
- `arda-varda` -> `arda-core`, `arda-governance`, `arda-vaire`, `arda-economics`
- `arda-service-registry` -> no spine path deps in this tree
- `arda-aule` -> no spine path deps in this tree
- Historical note: `arda-athena` is now retired/renamed to `arda-varda`; surviving source references should follow the new crate name.

## Crate public surface & role

### `arda`
- Single binary in `src/main.rs`.
- Calls `arda_engine::boot()`, loads `services.toml`, resolves services,
  supervises them, and opens harness tap-in on `127.0.0.1:7878` by default.
- No direct spine crate imports besides `arda-engine`.

### `arda-engine`
- Facade crate with real implementations: `boot()`, `harness::serve()`,
  `supervisor::Supervisor`, `registry::Registry`, `manwe` re-export,
  `arda_core::service_registry` re-export.
- `harness.rs` is a real axum surface: `/health`, `/v1/status`, `/v1/models`,
  `/v1/harness`.
- `supervisor.rs` is a real tokio process supervisor with restart backoff,
  shutdown, and PID mirror sync.
- `registry.rs` resolves `services.toml` into runnable `Service` specs.

### `arda-core`
- Root library.
- Exports `Agent`, `Task`, `Message`, `LlmProvider`, `Config`,
  `ToolRegistry`, `Soterion*`, `SystemctlClient`, `Ledger`,
  `spawn_bounded_background`, `try_run_bounded_async`, etc.
- Contains real behavior:
  - `loop_engine.rs`: dispatcher/reflector substrate with intent router,
    joule estimator hook, bid board, triad consultant hook, governance
    gates, goal-level joule budgets, halt-file kill switch.
  - `service_registry/`: in-memory registry, contract/service kinds,
    startup order, state validator.
  - `agent.rs`, `task.rs`, `llm.rs`: core agent/task/LLM abstractions.

### `manwe`
- Local inference gateway; library + binary.
- Library exposes `ProviderCatalog`, `ProviderRecord`, `AdaptiveRoutingAdapter`
  and transport traits/charon shims.
- Binary listens on configurable address; serves `/healthz`, `/v1/models`,
  `/v1/chat/completions` with static catalog fallback.

### `arda-council`
- Blueprint/template crate for sovereign agents.
- Provides `contract`, `council`, and `service` modules.

### `arda-governance`
- Governance logic: triad, resonance, readiness, philosopher profiles,
  vision convergence, audio/bacon-lite/joulework modules.

### `arda-orome`
- Hermes/boardroom comms/runtime bridge; not just A2A/A2H types.
- Real public surface in `comm.rs` and `service.rs`: Hermes service,
  inbound/outbound queues, boardroom posts, council discussions,
  decision prompts/execution, task approvals, interruptions,
  semantic channel / Discord projections, status surfaces,
  MCP server/channel tools.
- Uses `arda_core`, `arda_governance`, `arda_vaire`, `arda_mandos`,
  `arda_economics`.

### `arda-economics`
- Economics layer: cost models, JouleWork tracker, ledger,
  LoveEquation, energy metering, service/transport shells.
- `economics.rs`: `EconomicsEngine`, `LinearCostModel`, `ROIMetrics`.
- `ledger.rs`: `PlutusLedger`.
- `love_equation.rs`: `LoveEquation`.
- `meter.rs`: `MeterRegistry`, `TariffTable`, `EstimatorMeter`.
- `service.rs` / `transport/`: `PlutusService`, `PlutusDaemon`,
  `PlutusDaemonConfig`, IPC/HTTP stubs.

### `arda-mandos`
- Annunimas ORACLE runtime; real implementation visible in source:
  `OracleEngine`, `OracleService`, `OracleQuery`, `Verdict`,
  `TruthScorer`, `PageIndex`.
- `reasoning.rs`: triad/bacon/Sun Tzu gate scoring with verdict history
  and status snapshots.
- `scoring.rs`: `DefaultTruthScorer`, `GateVerdict`.
- `service.rs`: `OracleService` with verdict ledger, runtime status,
  background work and relationship signals.
- Tests still reference `ARDA_PLUTUS_HOME` and `arda_plutus`.
- `lib.rs` re-export additions appear mid-migration: `context`, `notify`, `pageindex`, `transport`. Those modules exist on disk, but represent incomplete surface cleanup.

### `arda-vaire`
- Memory/state service.
- Real implementation visible: episodic/semantic/procedural/archive
  layout, significance scoring, chain hashing, noise ledger,
  Obsidian sync, JSONL append helpers.
- Tests still reference `ARDA_PLUTUS_HOME` and `arda_plutus`.

### `arda-aule`
- Current source is a direct copy of `arda-council` with crate name
  changed to `arda-aule`; same contract/council/service modules and
  smoke tests, no observability code.

### `arda-launcher`
- Tauri app shell; have validated Rust build entry point includes `build.rs`
- Inspected Rust code covers Tauri bootstrap and onboarding
  pipeline: environment profile, device scan, provider checklist,
  readiness projection, private config staging/apply, guided session,
  service plan, receipt/apply flow.
- No `arda-engine` or harness integration in inspected code.

### `arda-varda`
- Athena executor; real ingest surface in source:
  crawl/deep/extraction/github/scholarly pipelines, routing,
  remediation, policy, uncertainty sampling, metrics, interceptor.
- Background work and learning modules exist under `#[cfg(test)]`.

### `arda-service-registry`
- In this tree it is a thin facade shell; the registry implementation
  lives in `arda-core::service_registry`.

## Audit gaps / remediation

- `arda-aule` is a direct copy of `arda-council` source; claim that it is a
  combined Prometheus/CEO/council observability crate is not supported by
  current code evidence.
- `arda-engine` has real wiring now; earlier placeholder claim was wrong.
- `arda-launcher` inspected source lacks daemon-spine/IPC integration; validated build entry point exists (`build.rs`), but no harness path. Still reflects Annunimas-origin shape; needs controller/IPC audit against the Arda runtime.
- `arda-athena` was renamed to `arda-varda`; git-status rename artifacts are visible and should be cleared from discovery results.
- `arda-plutus` is absent from disk; tests in `arda-vaire` and `arda-mandos`
  still reference `arda_plutus`, which will break once the orphan is fully
  removed from active closure.
- `arda-mandos` and `arda-vaire` retain Annunimas-era `ARDA_PLUTUS_HOME`
  test paths; should migrate to `arda-economics` service paths/resolution.
- `arda-mandos/lib.rs` has added re-exports for `context`, `notify`, `pageindex`, `transport`, but those modules are not yet reflected in public docs.
- `cargo check --workspace --all-targets` fails today because of those stale
  plutus imports, which indicates this is not just a doc issue—it’s a live
  compilation blocker.
