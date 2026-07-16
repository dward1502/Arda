# charon → manwe Migration Phases

Status: Phase 3 complete. Phase 4 pending.

## Phase 1 — Stand up routing lib under manwe
- [x] Split `crates/spine/runtime/manwe` into lib + `manwe` bin.
- [ ] Port routing logic from `crates/old-annunimas/arda-charon`.
- [ ] Add the 4 local trait shims still required by the routing lib.
- [ ] Add tests / build evidence for the routing lib.
- [x] Create migration scratch file: `docs/plans/charon-manwe-migration-phases.md`.

## Phase 2 — Wire adapter into manwe bin behind feature
- [x] Add `adaptive` feature to the `manwe` package.
- [x] Default (no feature): static 7171 gateway unchanged.
- [x] With `adaptive`: compile new adapter code paths; preserve static upstream fallback.

## Phase 3 — Transport layer (feature only)
- [x] Port adaptive HTTP transport shell behind `adaptive` feature.
- [x] Port adaptive IPC transport shell behind `adaptive` feature.
- [x] Keep static upstream fallback by default.

## Phase 4 — Workspace green + docs
- [ ] `cargo build --workspace` and `cargo test --workspace` green.
- [ ] Fix charon doc path-drift.
- [ ] Update `REFACTOR_PLAN.md` if it references old charon location.
