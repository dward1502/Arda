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

# Arda Architecture Interaction Audit

Audit type: Architecture / dependency interaction review
Scope: All workspace crates declared in `Cargo.toml` and active Rust source
Evidence: `cargo metadata --format-version=1 --no-deps`, `Cargo.toml` manifests, and direct source inspection

## Executive summary

- `arda-core` is the true foundational library and now hosts the real
  service-registry and dispatcher/reflector substrate.
- `arda-orome` is the broadest composite consumer and the real Hermes/
  boardroom comms runtime.
- `arda-engine` is the daemon-side bridge with real harness/supervisor/registry code.
- `arda-launcher` is manifest-level `arda-core` only; no daemon-spine wiring.
- `arda-mandos` is the Annunimas ORACLE runtime; `arda-economics` owns economics/ledger state.
- `arda-service-registry` is a retired facade; logic moved into `arda-core`.
- `arda-athena` is retired/renamed to `arda-varda`; only git-rename artifacts remain.
- `arda-plutus` is absent from disk; stale test references remain.
- `arda-aule` is a direct copy of `arda-council` source, not observability code.
- `arda-hades`/`arda-prometheus` are retired and absent from active closure.

## Scope

Included crates:
- arda, arda-engine, arda-core, manwe, arda-council, arda-governance
- arda-orome, arda-economics, arda-mandos, arda-vaire, arda-aule
- arda-launcher, arda-varda, arda-service-registry

Excluded/non-active:
- `arda-plutus` — not present on disk; prior metadata references are stale
- Tauri app `arda-hud` — independent workspace

## Methodology

1. Enumerated workspace members from root `Cargo.toml`.
2. Ran `cargo metadata --format-version=1 --no-deps` to capture package graph.
3. Inspected representative implementation files to verify actual crate roles
   and interaction behavior, not just manifest declarations.
4. Cross-checked public surfaces against doc comments and source usage.

## Findings

### CRITICAL

| ID | Finding | Evidence | Severity | Compliance |
|------|------|------|------|------|
| A-01 | `arda-plutus` absent from disk while tests still import `arda_plutus` | `crates/spine/memory/arda-vaire/src/service.rs` tests use `arda_plutus::PlutusService`; `crates/spine/runtime/arda-mandos/src/service.rs` uses `arda_plutus::PlutusService` | Critical | Blocking |
| A-02 | `arda-aule` is a literal copy of `arda-council` with crate names swapped; no observability/Prometheus/CEO implementation present | `crates/spine/observability/arda-aule/src/{lib,contract,council,service}.rs` identical structure to `arda-council` | Critical | Blocking |

### HIGH

| ID | Finding | Evidence | Severity | Compliance |
|------|------|------|------|------|
| A-03 | `arda-launcher` inspected Rust code lacks daemon-spine/IPC integration; no `arda-engine` or harness usage | `apps/arda-launcher/src-tauri/src/{lib,main,onboarding/*}.rs` only Tauri bootstrap and onboarding artifacts | High | Degraded |
| A-04 | `arda-service-registry` standalone crate is a thin facade; real registry lives in `arda-core` | `crates/spine/executors/arda-service-registry/src/lib.rs` re-exports only; `arda-core/src/service_registry/` contains full implementation | High | Duplicate/retired |

### MEDIUM

| ID | Finding | Evidence | Severity | Compliance |
|------|------|------|------|------|
| A-05 | `arda-mandos` and `arda-vaire` tests use Annunimas-era `ARDA_PLUTUS_HOME` env paths | `crates/spine/memory/arda-vaire/src/service.rs:303` sets `ARDA_PLUTUS_HOME`; `crates/spine/runtime/arda-mandos/src/service.rs:217` sets `ARDA_PLUTUS_HOME` | Medium | Stale naming |
| A-06 | `arda-engine` previously audited as placeholder; source now shows real supervisor/harness/registry code | `crates/engine/src/{harness,supervisor,registry}.rs` contain real axum/tokio/service-resolution logic | Medium | Previously incorrect |

## Compliance assessment

- Is the dependency graph accurate? Yes, based on `Cargo.toml` and `cargo metadata`.
- Are crate responsibilities aligned with source? Partially; `arda-aule` and `arda-launcher` are misaligned.
- Is there dead/retired code in active closure? Yes; `arda-service-registry` facade and stale `arda-plutus` references.
- Is there missing observable behavior? Yes; `arda-aule` lacks observability implementation.

## Risk assessment

- Highest risk: broken test/runtime paths from missing `arda-plutus` (A-01)
- High risk: `arda-aule` cannot fulfill observability requirements in current state (A-02)
- Medium risk: launcher/HUD isolation from daemon spine limits runtime control surface (A-03)
- Low risk: stale `ARDA_PLUTUS_HOME` naming (A-05) until orphan removal proceeds

## Remediation roadmap

1. Resolve `arda-plutus` references:
   - Either restore under `arda-economics` merge or remove references from `arda-vaire`/`arda-mandos` tests
2. Replace `arda-aule` source with real observability types or retire the crate
3. Decide `arda-launcher` daemon-spine integration strategy and document intent
4. Retire or repurpose `arda-service-registry` standalone crate
5. Update `arda-mandos`/`arda-vaire` test paths to `arda-economics` service resolution

## Audit trail

- Reviewed crates: 14 workspace crates + orphan assessment
- Source files inspected: 40+ implementation files
- Tooling: cargo metadata, direct file inspection
- Auditor: HADES
- Date: 2026-07-16
