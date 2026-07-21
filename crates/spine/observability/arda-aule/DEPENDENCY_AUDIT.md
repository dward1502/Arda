# arda-aule Dependency Audit — Annunimas Import Surface

> Imported from `~/Annunimas/crates/annunimas-prometheus`, `annunimas-ceo`, `annunimas-cli`.
> Arda workspace is the source of truth. `~/Annunimas` is read-only reference.

---

## 1. Import Fallback Gate

These crate paths are referenced via `use`/`pub use` in the imported sources.
Every entry must be matched to an Arda crate or explicitly deferred before `cargo check` can pass.

| Old Annunimas crate | Arda equivalent or status | Notes |
|---|---|---|
| `annunimas_core` | `arda-core` | Core types, errors, tasks, messages |
| `annunimas_council` | arda-aule | Re-exported types: `CouncilQuery`, `CouncilSeat`, `QueryMode` |
| `annunimas_fleet` | removed | Fleet + edge dispatcher |
| `annunimas_governance` | `arda-governance` | Governance + bacon lite + triad + love dynamics |
| `annunimas_hermes` | `arda-orome` | Hermes MCP/bridge |
| `annunimas_mnemosyne` | `arda-vaire` | Memory service |
| `annunimas_apollo` | `arda-orome` | Transport IPC |
| `annunimas_oracle` | `arda-governance` | Oracle engine |
| `annunimas_plutus` | `arda-economics` | Plutus model |
| `annunimas_charon` | `manwe` | Charon gateway |
| `annunimas_hades` | `arda-mandos` | Hades service |
| `annunimas_athena` | `arda-varda` | Athena ingest |
| `annunimas_chronos` | *removed | Chronos runtime |
| `annunimas_onboarding` | `apps/arda-hud` | Onboarding |
| `annunimas_forge_mind` | removed | Forge render/vision/MCP |
| `annunimas_comm` | arda-orome | A2H comms |
| `annunimas_systemd` | arda-core | systemd client |
| `annunimas-mcp` | arda-core | MCP browser/session |

---

## 2. Crates to resolve before compile

These imports are required by the wired submodules and will block `cargo check -p arda-aule`:

- `prometheus/council.rs`: `annunimas_council::council::...`, `annunimas_council::service::...`
- `prometheus/service/status.rs`: `annunimas_hermes::HermesService`
- `prometheus/pipeline.rs`, `/preflight.rs`, `/local_execution.rs`: `annunimas_core`, `annunimas_governance`, `annunimas_fleet`, `annunimas_mnemosyne`
- `prometheus/transport/ipc.rs`, `/http.rs`: `annunimas_core`
- `prometheus/autopilot/apollo_bridge.rs`: `annunimas_apollo`
- `prometheus/autopilot/a2h.rs`: `annunimas_comm`
- `prometheus/autopilot/outcomes.rs`: `annunimas_core::learning`
- `prometheus/autopilot/oracle_gate.rs`: `annunimas_governance`, `annunimas_oracle`
- `prometheus/autopilot/service_health.rs`: `annunimas_systemd`
- `prometheus/autopilot/runner.rs`: `annunimas_apollo`
- `ceo/pipeline.rs`: `annunimas_core`, `annunimas_fleet`, `annunimas_mnemosyne`

---

## 3. CLI dependencies — probable delay list

`cli/main.rs` and `cli/cli_bootstrap.rs` currently reference almost every service in Annunimas.

A likely simplification for Arda ownership: retain only the Arda-active imports, wrap others behind feature flags or deferred CLI commands.
Replacement candidates under Arda:

|- `annunimas_apollo` → `arda-orome`
|- `annunimas_athena` → `crates/spine/executors/arda-varda`
|- `annunimas_charon` → `crates/spine/runtime/manwe`
|- `annunimas_hades` → `crates/spine/runtime/arda-mandos`
|- `annunimas_hermes` → `arda-orome`
|- `annunimas_mnemosyne` → `crates/spine/memory/arda-vaire`
|- `annunimas_oracle` → `arda-governance`
|- `annunimas_plutus` → `crates/spine/runtime/arda-economics`
|- `annunimas_prometheus` → now `arda-aule::prometheus...`
|- `annunimas_core` → `crates/spine/governance/arda-core`
|- `annunimas_governance` → `crates/spine/governance/arda-governance`
|- `annunimas_forge_mind` → removed
|- `annunimas_onboarding` → `apps/arda-hud`
|- `annunimas_chronos` → removed

Expect many `cli` commands to be gated behind `#[cfg(feature = "full-cli")]` until the above are available in Arda.

---

## 4. String / path deposits still carrying old names

These do not block compilation, but are part of the migration surface.

- `cli/binary.rs`: `"/tmp/annunimas-target"`
- `cli/policy_guard.rs`: `"annunimas_totality"`, `"annunimas-system-control-readonly-*"`, `"annunimas-cli-*"`
- `cli/main.rs`: `"annunimas-root()"`, cargo run `-p annunimas-cli`, temp roots `annunimas-cli-*`, unit `annunimas-*`
- `export_surface.rs`: references `crates/annunimas-...` paths and `annunimas-cli` authorities
- `observability.rs`: `annunimas_prometheus::...`, `annunimas-ceo-autopilot-supervised.service`

---

## 5. Compile blockers observed after wiring modules

- `ceo/mod.rs`: declared `pub mod service;` without source file present.
- `cli/mod.rs`: declared modules `policies`, `telemetry`, `actors`, `narrative`, `tracing` without corresponding source files under `cli/`.
- `prometheus/autopilot/runner.rs`: `serde_json::json!` macro recursion limit exceeded; crate-level attribute `#![recursion_limit = "512"]` needs to move to `lib.rs`.
