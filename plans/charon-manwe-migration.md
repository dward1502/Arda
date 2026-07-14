# charon → manwe Migration Plan

Status: Phase 0 complete (workspace builds). Phases 1-4 pending.

## Context
`annunimas-charon` was copied (full copy) from `~/Annunimas` to
`crates/old-annunimas/annunimas-charon/`. Goal: merge charon's routing
functionality into `manwe` (the frozen local root gateway at `127.0.0.1:7171`)
without breaking manwe's default contract. Adaptive routing is behind a feature
flag; default 7171 behavior is unchanged.

Decisions (from user):
- Q1: Option 2 (trait shims) for the 4 `annunimas_*` deps that charon needs.
- Q2: routing logic lives in `crates/spine/runtime/manwe`.
- Q3: charon is already present locally; do NOT vendor external annunimas crates.

## Phase 0 — Fix engine dangling reference  [DONE 2026-07-14]
**Root cause (corrected):** `crates/engine/src/lib.rs:10-11` had
`pub use annunimas_charon as charon;` and `pub use annunimas_core as core;`.
The workspace `Cargo.toml` defines `annunimas-charon`/`annunimas-core` under
`[workspace.dependencies]` (a shared *reference catalog*), but `engine/Cargo.toml`
never opted into them, so the re-export could not resolve → `cargo build -p
arda-engine` failed with E0432. They were also **dead**: nothing in the tree
consumes `arda_engine::charon` or `::core`.

**Fix:** removed both lines; kept `arda_onboarding`/`arda_service_registry`.
Verified `cargo build -p arda-engine` and `cargo build --workspace` green
(only pre-existing trivial warnings remain).

The charon re-export will be reintroduced from the new
`crates/spine/runtime/manwe` crate in Phase 1.

## Phase 1 — Stand up routing lib under manwe
- Create `crates/spine/runtime/manwe/` (the `manwe` package / `manwe-routing`
  lib + `manwe` bin).
- Port charon's routing logic from `old-annunimas/annunimas-charon/src/service.rs`.
- The 4 `annunimas_*` deps (core, governance, mnemosyne, plutus) are stubbed/absent
  under `old-annunimas/`. Resolve via Option 2: define local trait shims
  (`CharonCore`, `CharonGovernance`, etc.) that the routing code depends on, with
  in-crate default impls so it compiles without the real annunimas crates.
- `cargo test -p manwe-routing` green.

## Phase 2 — Wire adapter into manwe bin behind feature
- Add `adaptive` feature to the manwe package.
- Default (no feature): manwe serves static catalog at `127.0.0.1:7171` unchanged.
- With `adaptive`: instantiate the routing adapter from Phase 1.

## Phase 3 — Transport layer (feature only)
- Port http/ipc transport behind `adaptive` feature only.

## Phase 4 — Workspace green + docs
- `cargo build --workspace` and `cargo test --workspace` green.
- Fix charon doc path-drift (`old-annunimas/annunimas-charon/{INDEX,README}.md`
  reference `~/Annunimas/...` and `crates/spine/runtime/annunimas-charon`).
- Update `REFACTOR_PLAN.md` if it references the old charon location.
