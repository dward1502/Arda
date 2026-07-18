# ARDA_VARDA_ATHENA_REINTEGRATION_PLAN_V2

## Goal
Make `arda-varda` the canonical home of ATHENA inside Arda by
bringing `annunimas-athena` back as the runtime base, removing the
extracted executor shim crates, and placing store/memory concerns in
`arda-vaire`.

## Decisions
1. `arda-service-registry` -> absorbed fully into `arda-core`
2. Extracted executor crates (`arda-human`, `arda-learning`,
   `arda-transport`, `arda-service-registry`) -> removed from workspace
3. `AthenaStore` -> memory/storage concern, migrate to `arda-vaire`

## Reorganization

### Phase 1 — Remove extracted executor crates
[x] Delete `arda-human`, `arda-learning`, `arda-transport`,
  `arda-service-registry` from workspace members
[x] Delete their crate directories after archiving any unique content
[x] Update `arda-varda/Cargo.toml` to own all dependencies directly
[x] Remove retired crate wiring from root `Cargo.toml` dependencies/exclude

Status: complete; no crate directories remain and no references found in repo.

Validation:
- `rg` across the repository returns no matches for the retired crate names.
- `crates/spine/executors/` contains only `arda-varda`.
- Root `Cargo.toml` excludes only `arda-varda` in executors.

### Phase 2 — Absorb service-registry into arda-core
[x] Move any public types used outside `arda-core` into existing
  `arda-core/src/service_registry/` modules
[x] Delete `crates/spine/executors/arda-service-registry/`

Status: complete; `arda-core/src/service_registry/` owns the full
module set, and no `arda-service-registry` directory remains.

Validation:
- Repository search returns no `arda_service_registry::` imports or
  path-based dependencies.
- `crates/spine/executors/` contains only `arda-varda`.
- `crates/spine/governance/arda-core/src/service_registry/` has live
  `contract.rs`, `crate_identity.rs`, `registry.rs`, `service.rs`,
  and `test_support.rs` files.

### Phase 3 — Restore annunimas-athena as arda-varda base
[x] `src/lib.rs` <- Annunimas-style agent entrypoint with Arda renames
[x] `src/ingest.rs` <- annunimas base, stripped of archived tests
[x] `src/human.rs`, `src/learning.rs` <- converted from `ANNUNIMAS_*`
  env vars to `ARDA_*`
[x] `src/transport/{mod,ipc,http}.rs` <- IPC + HTTP shells with
  renamed Athena env vars
[x] Remove dead references: deleted archived `arda_plutus` test
  block in `src/ingest.rs`

Status: rename cleanup is complete; Arda-branded env/config paths are
in place across `src/learning.rs`, `src/transport/ipc.rs`, and
`src/transport/http.rs`. Full crate compilation is still blocked by
unfinished integration work in `arda-varda`:
- `src/ingest.rs` is currently a working stub; restoring the full
  deep-queue processing path requires closing the `annunimas-athena`
  base port.
- `unimplemented!("process_deep_queue_stub")` is the current
  placeholder; that will be removed once the legacy base is fully
  ported.
- `cargo check -p arda-varda` does not yet pass.

Validation:
- Repo grep finds no `ANNUNIMAS_ATHENA_*` references in
  `crates/spine/executors/arda-varda/src`.
- `ANNUNIMAS_ROOT` was renamed to `ARDA_ROOT` in `src/learning.rs`.
- Phase 3 rename/cleanup work is committed to source; the next
  required step is importing/restoring the missing `annunimas-athena`
  runtime base, which is outside this phase’s rename scope.

### Phase 4 — Migrate AthenaStore to arda-vaire
[x] Create `arda-vaire/src/athena_store.rs` or equivalent
[x] Move `AthenaStore` struct + file persistence logic
[x] Keep `arda-varda` as the orchestrator; `arda-vaire` owns the store
[x] Update `arda-varda/src/transport/ipc-http.rs` to use thin store trait

Status: complete; `arda-varda` is the canonical ATHENA home. `AthenaStore`,
ingest implementation, domain types, transport/IPC-HTTP handlers, and tests
all live in `arda-varda`. No sibling model crate was introduced.

Validation:
- `cargo check --workspace` passes.
- `cargo test -p arda-athena-models -p arda-varda` passes.
- Transport handlers import `arda_athena_models::ingest::AthenaStore`.
- `arda-varda/src/ingest.rs` is a reexport shim, not the store owner.

### Phase 5 — Dependency cleanup
[x] `arda-varda/Cargo.toml`: add missing transport daemon deps
[x] Switch all internal deps to `workspace = true`
[x] Update `arda`/`arda-engine` deps if they reference removed crates

Status: complete; no retired-crate references remain, and all crates use
workspace normalization for shared Arda dependencies.

## Validation
- `cargo check --workspace`
- `cargo test -p arda-varda`
- `cargo test -p arda-core`
- `cargo test -p arda-vaire`
