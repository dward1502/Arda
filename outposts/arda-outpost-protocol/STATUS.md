# arda-outpost-protocol status

Crate: `outposts/arda-outpost-protocol`
Current state: first-class active; Packet 5 closed
Branch: `manwe`
Documentation: `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, `OWNERSHIP.md`

Current signature: a library-only, versioned observation contract with stable
snake_case JSON output, backward-compatible v1 input, explicit non-execution
authority, typed feedback conversion, and bounded process-local topic queues.

## Live integration

- Direct Cargo consumer: `arda-outpost-scout`.
- No optional Cargo features, binary target, build script, generated source, or
  runtime configuration.
- Five production Rust modules and three integration-test targets are fully
  classified in `BREAKDOWN.md`.

## Packet 5 contract repairs

- Canonicalized scope, classification, and authority JSON output to snake_case.
- Preserved decoding of legacy v1 PascalCase enum values.
- Pinned rejection of unknown authority/classification/scope values.
- Added an explicit invariant that no current authority class permits execution.

## Closeout evidence

- `cargo fmt -p arda-outpost-protocol -- --check`: passed.
- No-default and all-feature checks passed; the crate has no feature
  declarations, so both modes intentionally compile the same contract.
- No-default and all-feature tests each passed 11 integration tests: 6
  authority/wire, 2 feedback conversion, and 3 queue tests.
- Strict all-target/all-feature Clippy passed with warnings denied.
- All-feature Rustdoc passed with warnings denied and no dependencies.
- `cargo check -p arda-outpost-scout --all-targets --all-features`: passed.
- `cargo test -p arda-outpost-scout --all-features`: 20 tests passed after its
  runtime API fixture was migrated to the canonical lowercase `custom` scope
  key.
- The only command warning was the pre-existing ignored non-root profile in
  `apps/arda-launcher/src-tauri/Cargo.toml`.
