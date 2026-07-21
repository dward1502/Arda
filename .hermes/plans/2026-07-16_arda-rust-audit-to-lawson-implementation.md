# Arda Audit Recovery + Lawson Roadmap
*Saved for implementation planning; no code edits yet.*

## Goal
Convert the verified 2026-07-16 audit state into an executable batch plan:
1. Fix stale INDEX/docs mismatches.
2. Normalize dependency declarations to `workspace = true`.
3. Close the `arda-varda` mixed-concern split by extracting `human`, `learning`, `transport`, and `service_registry` into real crates.
4. Migrate test layout to reduce executor-local context.
5. Preserve a verified no-cycle layering contract.

## Assumptions / Constraints
- `arda-core` stays the dependency root with zero workspace-internal edges.
- Extracted executor crates are phased in, never removed without migrating their consumer imports first.
- Tests stay inside individual crate `tests/` directories only if the owning crate owns them; otherwise archive or delete.
- No new docs are created; existing README/INDEX blocks are repaired to match current tree.

## Current verified state
- 17 workspace members confirmed by `cargo metadata --format-version 1`.
- `arda-mandos` uses 4-level deep relative `arda-economics` path.
- `arda-varda` still defines `pub mod human;`, `learning;`, `transport;`, `service_registry;`.
- `runtime/INDEX.md` still lists a non-existent `arda-systemd`.
- `executors/INDEX.md` omits `arda-human`, `arda-learning`, `arda-transport`, `arda-service-registry.
- Resolved graph currently has no cycles; concern is future maintenance risk.

## Batch plan

### Batch 0 — INDEX/Readme repair (immediate hygiene)
1. Update `crates/spine/runtime/INDEX.md` to replace `arda-systemd` with `manwe`.
2. Update `crates/spine/executors/INDEX.md` to include `arda-human`, `arda-learning`, `arda-transport`, `arda-service-registry`.
3. Audit `crates/spine/governance/arda-core/tests/`, `arda-governance/tests/`, `arda-council/tests/` for outdated README/INDEX mentions; patch in place.
4. Verify no INDEX block maps observability to a stale crate name.

Validation: `rg arda-systemd -S -g '!target'`.

### Batch 1 — Dependency style normalization
Scope: only workspace-internal edges that already exist. Do not add or remove edges.
1. `arda-varda/Cargo.toml`: change workspace deps to `workspace = true`; keep path deps to extracted crates only until Batch 3 completes.
2. `arda-human/Cargo.toml`, `arda-learning/Cargo.toml`, `arda-service-registry/Cargo.toml`: unify to `workspace = true`.
3. `arda-orome/Cargo.toml`, `arda-vaire/Cargo.toml`, `arda-economics/Cargo.toml`: convert internal deps to `workspace = true`.
4. `arda-mandos/Cargo.toml`: convert `arda-economics` dep from `../../../../crates/...` to `workspace = true`.
5. `arda-core/Cargo.toml`, `arda-governance/Cargo.toml`: no workspace edges exist; keep as-is.

Validation: `cargo metadata --format-version 1` then full graph checksum script.

### Batch 2 — `arda-varda` concern extraction
1. For `arda-human`, `arda-learning`, `arda-transport`, `arda-service-registry`:
   - Copy `human.rs`, `learning.rs`, `transport/*`, `service_registry.rs` into respective crate `src/`.
   - Add module declarations to each crate `lib.rs`.
   - Port deps needed by the copied code; keep the surface identical to current public API where callers already import those items directly.
2. In `arda-varda/src/lib.rs`:
   - Replace `pub mod human;`/`pub mod learning;`/`pub mod transport;`/`pub mod service_registry;` with `pub use arda_human::...;` re-exports.
   - Keep runtime feature flags aligned with existing feature slice behavior.
3. `arda-varda/Cargo.toml`:
   - Remove now-unused direct path deps for extracted crates; switch to `workspace = true`.

Validation: `cargo check -p arda-varda -p arda-human -p arda-learning -p arda-transport -p arda-service-registry`.

### Batch 3 — Test layout migration
1. Move `crates/spine/executors/arda-varda/tests/*` into a single `tests/varda_legacy.rs` directory only if `arda-varda` still requires them.
2. Delete duplicate/shadowed test files if the executor no longer owns them; validate with `cargo test -p <crate>`.

Validation: each affected crate runs its test suite without regressions.

## Validation harness
- `cargo metadata --format-version 1 > /tmp/arda-metadata.json && python script to verify:`:
  - workspace member count = 17
  - no cycles among workspace crates
  - all internal deps are `workspace = true`

## Files likely to change
- INDEX/README markdown in each crate/ bucket root (`crates/spine/.../INDEX.md`).
- `crates/spine/executors/arda-varda/Cargo.toml`
- `crates/spine/executors/arda-*/Cargo.toml`
- `crates/spine/interface/arda-orome/Cargo.toml`
- `crates/spine/memory/arda-vaire/Cargo.toml`
- `crates/spine/runtime/arda-economics/Cargo.toml`
- `crates/spine/runtime/arda-mandos/Cargo.toml`
- `crates/spine/executors/arda-varda/src/lib.rs`
- Source/test files moved between executor crates.

## Risks / open questions
- Deep copies during Batch 2 may temporarily expand compile surface; keep PRs small.
- If `arda-council` becomes the canonical executor owner later, extractions must target `arda-council` instead of dedicated executor crates.
- `arda-aule` docs/usage path may still call into other observability code; that contract remains unverified until dependency graph includes `arda-aule` consumers.
