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

# Breakdown: crates/engine

## Purpose (one sentence)

`arda-engine` is the single dependency surface the `arda` daemon uses to reach system services. It owns process supervision, declares services from data (`services.toml`), exposes a harness HTTP control surface, and re-exports spine types so callers import from `arda_engine` instead of reaching into vendored crates directly.

## Why it exists

The daemon previously had service spawning and discovery hardcoded in `main.rs`. `engine` separates that into a proper module with declarative registry, supervised restart/backoff, a local status port, and a stable abstraction layer over `manwe` and `arda-core`.

## What it does

| capability | owner module | notes |
|---|---|---|
| Process supervision with restart + backoff | `supervisor.rs` | Tokio-based `Supervisor`, `Service`, `Shutdown`; exponential backoff capped at 10s |
| Declarative service discovery | `registry.rs` | Loads `services.toml`, resolves executables relative to repo root, supports required/optional and `--no-ui` filtering |
| Harness HTTP surface | `harness.rs` | Axum app on `127.0.0.1:7878`; routes `/health`, `/v1/status`, `/v1/models`, `/v1/harness` |
| `/v1/models` proxy to manwe | `harness.rs` | Proxies to `manwe` gateway on `:7171` so callers use one tap-in port |
| Spine re-exports | `lib.rs`, `manwe.rs` | Re-exports `manwe` and `arda-core::service_registry` |
| Daemon boot entrypoint | `lib.rs::boot()` | Placeholder today; real wiring lands here |

## Crate layout

```
crates/engine
├── Cargo.toml
├── INDEX.md
└── src
    ├── lib.rs
    ├── manwe.rs
    ├── harness.rs
    ├── supervisor.rs
    └── registry.rs
```

## Crate dependencies

```
arda-engine
├── arda-core     workspace  // service_registry types/constructs
├── manwe         workspace  // inference gateway transport + re-exports
├── tokio         workspace  // async runtime + process + task
├── tracing       workspace  // logging
├── anyhow        workspace  // error handling
├── serde         workspace  // services.toml deserialization
├── toml          workspace  // services.toml parsing
├── axum          workspace  // harness HTTP surface
├── serde_json    workspace  // JSON responses + proxy parsing
└── reqwest       workspace  // outbound proxy to manwe /v1/models
```

## Connection to manwe (`crates/spine/runtime/manwe`)

### Compile-time

- `arda-engine/Cargo.toml` depends on `manwe = { workspace = true }`
- `src/manwe.rs` re-exports the full `manwe` crate publicly:
  - `SpannedManweGateway`, `ProviderRecord`, `ProviderCatalog`
  - `Transport`, `ApiTransport`, `CharonTransport`
  - `CharonCore`, `CharonGovernance`, `CharonMnemosyne`, `CharonPlutus`
  - optional `CharonService` when `manwe` is built with `adaptive` feature

### Runtime

- `src/harness.rs::models()` proxies `/v1/models` to `manwe` using its base URL
- `HarnessState` stores `manwe_url: String`
- Default listen address `127.0.0.1:7878` is distinct from manwe's `7171`

## Usage contract

A daemon/tool using `arda_engine` can:

1. Call `arda_engine::boot()`
2. Load services via `Registry::load(path)` and `registry::resolve(root, no_ui)`
3. Supervise them via `Supervisor::new(services, shutdown)` + `.run()`
4. Start the tap-in port via `harness::serve(addr, state, shutdown)`
5. Use `arda_engine::manwe::{...}` and `arda_engine::service_registry` without depending on both crates individually

## Verification status

- `cargo check -p arda-engine`: successful
- `cargo test -p arda-engine`: 1 passed (`supervisor::tests::supervises_and_reaps_child_on_shutdown`)
- Static links to `manwe` confirmed in source:
  - `crates/engine/src/manwe.rs`
  - `crates/engine/src/harness.rs`
