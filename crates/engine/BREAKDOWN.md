---
soterion:
  sigil: "SPINE"
  glyph: "🧬"
  role: "supervision_spine"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-17"
---

> 🧬 arda-engine: 🧬 supervision_spine | owner: arda | status: active | reviewed: 2026-07-17

# Breakdown: `crates/engine`

## Purpose (one sentence)

`arda-engine` is the single library boundary through which the root `arda`
daemon resolves and supervises services, exposes its local harness, and reaches
the supported provider/governance spine.

## Why it exists

The daemon previously had service spawning and discovery hardcoded in `main.rs`. `engine` separates that into a proper module with declarative registry, supervised restart/backoff, a local status port, and a stable abstraction layer over `manwe` and `arda-core`.

## What it does

| capability | owner module | notes |
|---|---|---|
| Process supervision with restart + backoff | `supervisor.rs` | Tokio-based `Supervisor`, `Service`, `Shutdown`; exponential backoff capped at 10s |
| Declarative service discovery | `registry.rs` | Loads `services.toml`, resolves executables relative to repo root, supports required/optional and `--no-ui` filtering |
| Harness HTTP surface | `harness.rs` | Axum app on `127.0.0.1:7878`; routes `/health`, `/v1/status`, `/v1/models`, `/v1/harness` |
| `/v1/models` proxy to manwe | `harness.rs` | Proxies to `manwe` gateway on `:7171` so callers use one tap-in port |
| Spine re-exports | `lib.rs`, `manwe.rs` | Re-exports `manwe`, `arda-core::service_registry`, `arda-core::loop_observability`, and `observability` |
| Daemon startup integration | root `src/main.rs` | Loads/resolves the real registry before `--once`; engine exposes no no-op boot hook |

## Crate layout

```
crates/engine
├── Cargo.toml
├── BREAKDOWN.md
├── INDEX.md
├── OWNERSHIP.md
├── README.md
├── STATUS.md
└── src
    ├── lib.rs
    ├── manwe.rs
    ├── harness.rs
    ├── observability.rs
    ├── orome.rs
    ├── supervisor.rs
    └── registry.rs
└── tests
    └── orome_smoke.rs
```

## Crate dependencies

```
arda-engine
├── arda-core       workspace  // service registry and loop observability
├── arda-governance workspace  // aggregate governance status
├── arda-orome      workspace  // provider runtime and dispatch smoke
├── manwe           workspace  // inference gateway transport + re-exports
├── tokio         workspace  // async runtime + process + task
├── tracing       workspace  // logging
├── anyhow        workspace  // error handling
├── serde         workspace  // services.toml deserialization
├── toml          workspace  // services.toml parsing
├── axum          workspace  // harness HTTP surface
├── serde_json    workspace  // JSON responses + proxy parsing
└── reqwest       workspace  // outbound proxy to manwe /v1/models
```

## Supported source classification

| Classification | Count | Paths |
|---|---:|---|
| Production/default | 7 | `src/lib.rs`, `harness.rs`, `manwe.rs`, `observability.rs`, `orome.rs`, `registry.rs`, `supervisor.rs` |
| Production/feature-gated | 0 | No features are declared |
| Generated include | 0 | None |
| Standalone test-only source | 0 | Unit tests are inline |
| Integration test | 1 | `tests/orome_smoke.rs` |
| Build script | 0 | None |
| Unwired | 0 | None |

Every source file is reached through the default library graph or Cargo's
integration-test discovery. No module-root collisions exist.

## Connection to Manwe

### Compile-time

- `arda-engine/Cargo.toml` depends on `manwe = { workspace = true }`
- `src/manwe.rs` re-exports the full `manwe` crate publicly so callers import
  from `arda_engine::manwe`.

### Runtime

- `src/harness.rs::models()` proxies `/v1/models` to `manwe` using its base URL
- `HarnessState` stores `manwe_url: String`
- Default listen address `127.0.0.1:7878` is distinct from manwe's `7171`

## Usage contract

A daemon/tool using `arda_engine` can:

1. Load services via `Registry::load(path)` and `Registry::resolve(root, no_ui)`.
2. Reject required-service resolution errors.
3. For smoke mode, exit only after that validation.
4. Otherwise supervise through `Supervisor::new(services, shutdown).run()`.
5. Start the tap-in port through `harness::serve(addr, state, shutdown)`.
6. Use the supported re-exports without adding parallel direct dependencies.

## Verification status

- `cargo check -p arda-engine --all-targets --all-features`: passed
- `cargo test -p arda-engine --all-features`: 10 unit + 1 integration passed
- strict all-target Clippy and strict rustdoc: passed
- root `arda` all-target/all-feature consumer check: passed
- Static links to `manwe` confirmed in source:
  - `crates/engine/src/manwe.rs`
  - `crates/engine/src/harness.rs`
- GEN3 interop surface verified:
  - `arda-core::loop_observability` re-exported
  - `arda-core::learning_adapter` consumed through `arda_engine::observability::EngineObservabilityStatus`
