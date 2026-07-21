# ARDA_VARDA_ATHENA_REINTEGRATION_PLAN

## Goal
Make `arda-varda` the canonical home of ATHENA inside Arda by
bringing in the `annunimas-athena` source state, reorganizing the
on-disk layout, and retiring duplicate ownership from the extracted
executor crates.

## Current divergence
- `arda-varda/src/lib.rs` has a thin `AthenaAgent` runtime shell;
  `annunimas-athena/src/lib.rs` has a thicker runtime with `AthenaStore`
  initialization, typed task handling for `ingest`/`query`/`deep_analyze`,
  and missing `model_used` observability.
- `arda-varda/src/ingest/` and `src/transport/` exist but the main
  `src/ingest.rs` was archived because its tests imported `arda_plutus`.
- `arda-human`, `arda-learning`, `arda-transport`, `arda-service-registry`
  are standalone crates with thin canary tests; their real implementations
  already live in `arda-varda/src/`.

## Proposed reorganization

### 1. Restore `arda-varda` runtime from Annunimas base
Files to restore/overwrite from captured base:
- `src/lib.rs` ← annunimas base with Arda crate renames
- `src/ingest.rs` ← annunimas base, stripped of archived tests only
- `src/human.rs`, `src/learning.rs`, `src/transport/*` ← annunimas base
- `Cargo.toml` ← annunimas base, rewritten for `arda-*` deps only

Rename rules:
- `annunimas_core` → `arda_core`
- `annunimas_governance` → `arda_governance`
- `annunimas_plutus` → remove or map to `arda-economics`/`arda-core`
  concepts
- `ANNUNIMAS_*` env vars → `ARDA_*` env vars

### 2. File layout
Keep the current layout; do not flatten everything into `lib.rs`:
- `src/lib.rs` — agent runtime, lifecycle, store init, task routing
- `src/ingest.rs` — top-level types + orchestrator
- `src/ingest/*.rs` — helper modules
- `src/transport/{mod,ipc,http}.rs` — daemon transport
- `src/human.rs`, `src/learning.rs` — capability modules

Remove:
- `src/test_support.rs` if tests use inline env guards
- Empty `tests/` directories inside executor crates after canary move

### 3. Retire duplicate ownership
- `arda-human`: becomes re-export shim -> `arda-varda::human`
- `arda-learning`: becomes re-export shim -> `arda-varda::learning`
- `arda-transport`: becomes re-export shim for `DaemonConfig`,
  `TransportError`, `expand_home`
- `arda-service-registry`: keep or retire based on whether `arda-core`
  registry is sufficient

### 4. Dependency cleanup
- Remove `arda_plutus` references entirely from archived test code.
- Add missing transport daemon deps to `arda-varda/Cargo.toml`:
  `axum`, `tower`, `tokio-stream`, `tokio`, `reqwest`, `serde`, etc.
- Switch internal deps to `workspace = true`.

### 5. Tests
- Restore IPC/HTTP contract tests from `annunimas-athena/tests/`.
- Keep extracted crate canaries as forward-compat smoke tests.
- Update env var names in tests from `ANNUNIMAS_*` to `ARDA_*`.

## Files likely to change
- `crates/spine/executors/arda-varda/src/*`
- `crates/spine/executors/arda-varda/Cargo.toml`
- `crates/spine/executors/arda-human/src/lib.rs`
- `crates/spine/executors/arda-learning/src/lib.rs`
- `crates/spine/executors/arda-transport/src/lib.rs`
- `crates/spine/executors/arda-service-registry/src/lib.rs`

## Validation
- `cargo check --workspace`
- `cargo test -p arda-varda`
- `cargo test -p arda-human -p arda-learning -p arda-transport canary`

## Open questions
1. Should `arda-service-registry` remain as a public facade or be
   fully absorbed into `arda-core`/`arda-varda`?
2. Should the extracted executor crates be preserved as shims for
   backward compatibility, or removed entirely?
3. Is the `AthenaStore` path schema (`books/`, `digest.jsonl`,
   `deep_queue.jsonl`) still the desired on-disk contract, or should
   it migrate to `arda-vaire`/`arda-mandos` ownership?
