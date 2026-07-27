# arda-orome stabilization plan

Crate: `crates/spine/interface/arda-orome`
State: active stabilization decisions
Reviewed: 2026-07-26

## Purpose

Use this file for unresolved stabilization work and future proposals. `STATUS.md` reports current
truth; this file defines the decisions and acceptance criteria needed to change that status.

The previous implementation checklist and crate plan were complete trackers whose durable provider
and governance contracts are now in `README.md` and `BREAKDOWN.md`. They were removed from the
maintained set.

## Current commitments

### P0 — classify the 35 unwired Rust files

For every group in `BREAKDOWN.md`, choose one outcome:

1. **Wire:** make it reachable from one unambiguous module root and test its public contract.
2. **Migrate:** move behavior to its actual owning crate and update consumers.
3. **Retire:** prove it has no live consumer or include path, then delete it and stale references.

Required evidence:

- repo-wide symbol and path search;
- history review for intended ownership;
- explicit compiled successor for migrations/retirements;
- no broad `pub mod` exposure merely to make files compile;
- producer and affected-consumer tests after each bounded batch.

### P1 — reconcile the `http` feature

Choose one reviewed contract:

- wire a bounded HTTP/SSE surface and prove default/no-default behavior differs as documented; or
- remove the feature and its feature-only dependencies if HTTP is not owned here.

Acceptance requires feature-specific tests and README/Cargo agreement.

### P1 — reconcile the manifest

After source classification, remove dependencies used only by retired files and retain dependencies
required by wired/migrated contracts. Validate default, no-default, all-target, rustdoc, and direct
consumer builds.

### P1 — decide the test-only API boundary

Confirm whether `intent`, `registry`, `router`, and retry/expiry behavior are fixtures for internal
testing or intended production contracts. If production, expose them deliberately with integration
tests; otherwise keep them test-only and avoid advertising them as public capabilities.

## Stable-tree acceptance gates

- Every Rust source file is production-compiled, explicitly test-only, generated, or absent.
- Exactly one service module root exists if service behavior remains in this crate.
- Cargo features correspond to compiled behavior.
- Manifest dependencies correspond to the retained compiled/test/build surfaces.
- `README.md`, `BREAKDOWN.md`, `STATUS.md`, and all indexes match the live tree.
- Formatting, all-feature/no-default checks and tests, strict Clippy, rustdoc, engine smoke, Manwe
  gRPC, and Aule full-CLI consumer checks pass.
- Pre-existing user work is preserved.

## Completed implementation slices

### 2026-07-26 — duplicate source retirement and service-root decision

- Retired `src/router_retry_expiry.rs`; it was an unattached duplicate of retry/expiry tests that
  are compiled from `src/message_retry_expiry.rs`.
- Verified the retained retry and expiry tests individually before retirement.
- Retired `src/service/mod.rs`; `src/service.rs` already defines `HermesService`, declares every
  retained service child module, and is now the sole canonical service root.
- Reduced the source inventory from 56 to 54 files and the unwired inventory from 37 to 35.
- Repassed formatting, all-feature/no-default checks and tests, strict Clippy, rustdoc, the engine
  smoke test, and the Aule `full-cli` consumer check.
- Did not rerun Manwe after the deletion because another agent is actively modifying it; neither
  retired file was in Manwe's compiled dependency graph.
- Did not expose the service tree: ownership and dependency-closure classification remains P0.

## Future proposal template

1. Problem and live evidence.
2. Proposed ownership and public contract.
3. Non-goals and compatibility impact.
4. Safety, governance, observability, and rollback impact.
5. Exact acceptance commands and affected consumers.
6. Owner and decision: proposed, accepted, deferred, or rejected.
