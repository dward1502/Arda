# arda-orome status

Crate: `crates/spine/interface/arda-orome`
Version: `0.1.0`
State: **compiled surface stable; source-tree stabilization required**
Reviewed: 2026-07-26
Required crate-local work: **open; tracked in `PLAN.md`**

## Compiled production surface

- Six public module families: `comm`, `governance`, `grpc`, `message`, `provider`, and `types`.
- Provider dispatch has bounded timeout/retry, expiry rejection, direct/fanout routing,
  fleet-scope policy, metrics, and typed receipts.
- `GovernanceHooks` maps central action policy into typed `arda_core::Ledger` records.
- Two protobuf contracts generate tonic client/server/message surfaces.
- `arda-engine`, Manwe, and `arda-aule` are the three direct workspace consumers.

## Stability findings

1. **P0 source ownership:** 35 Rust files under `src/` are not reachable from `lib.rs` in either
   production or unit-test builds. They include the service, MCP, Discord, context, edge, relay,
   and slash-command trees. They are not covered by the successful Cargo gates.
2. **Resolved module root:** `src/service.rs` is the sole canonical service root. The duplicate
   `src/service/mod.rs` was retired on 2026-07-26; the retained service tree remains unwired pending
   ownership and dependency-closure review.
3. **P1 feature drift:** the default `http` feature enables `axum`, `tower`, and `tokio-stream`, but
   no compiled source is gated by or uses that feature. Default and no-default builds therefore
   expose the same tested Rust behavior.
4. **P1 manifest reconciliation:** several dependencies are referenced only by unwired sources.
   Dependency pruning must follow, not precede, the wire/retire decision.
5. **Intentional test-only boundary:** `intent`, `registry`, `router`, and
   `message_retry_expiry` are unit-test-only modules and unavailable to external consumers.

These findings prevent declaring the entire on-disk source tree stable. They do not invalidate the
compiled public surface or current consumers.

## Verification evidence

Passed from the workspace root on 2026-07-26:

- `cargo check -p arda-orome --all-features`.
- `cargo check -p arda-orome --no-default-features`.
- `cargo test -p arda-orome --all-features`: 21 passed
  (14 unit, 7 integration, 0 doctests).
- `cargo test -p arda-orome --no-default-features`: 21 passed
  (14 unit, 7 integration, 0 doctests).
- `cargo clippy -p arda-orome --all-targets --all-features -- -D warnings`.
- `cargo doc -p arda-orome --no-deps --all-features`.
- `cargo fmt -p arda-orome -p arda-engine -- --check`.
- `cargo test -p arda-engine --test orome_smoke`: 1 passed.
- `cargo check -p manwe --features grpc`.
- `cargo check -p arda-aule --features full-cli`.

The Manwe consumer check emitted seven Manwe-local unused/dead-code warnings. The strict
`arda-orome` Clippy gate was clean. Cargo also emitted the existing workspace informational warning
about the ignored non-root launcher profile.

The first implementation slice was reverified after retiring the two unattached files with all
default/all-feature/no-default producer gates, strict Clippy, rustdoc, the engine smoke test, and
the Aule `full-cli` consumer check. Manwe was not rerun after this source-retirement slice because
another agent is actively modifying Manwe; neither retired file was compiled or imported by Manwe.

## Worktree preservation

`src/registry.rs` already had a user modification before this audit. This documentation pass did
not alter or restore it.

## Stable release criteria

The whole crate tree can be declared stable after the active items in `PLAN.md` resolve the
unreachable source tree, feature contract, and resulting manifest surface,
followed by the same producer and consumer gates above.
