# arda-engine status

Crate: `crates/engine`
State: stable first-class foundation
Last verified: 2026-07-28
Branch: `manwe`

## Current contract

- All 8 Rust files are wired: 7 default production modules and 1 integration
  test target.
- The crate has no declared features; no-default and all-feature gates therefore
  exercise the same supported graph.
- The root `arda` package is the only direct Cargo consumer.
- The former no-op `boot()` boundary is removed.
- `arda --once` now loads and resolves `services.toml`, reports required-service
  errors, honors `--no-ui`, and exits before supervision or harness startup.
- Harness Manwe calls use the state-owned client, explicit five-second default
  timeout, and optional bearer forwarding.

## Verification evidence

- `cargo fmt -p arda-engine -p arda -- --check`: passed.
- `cargo check -p arda-engine --no-default-features`: passed.
- `cargo test -p arda-engine --no-default-features -- --test-threads=1`:
  10 unit + 1 integration passed; 0 failed.
- `cargo check -p arda-engine --all-targets --all-features`: passed.
- `cargo test -p arda-engine --all-features -- --test-threads=1`:
  10 unit + 1 integration passed; 0 failed.
- `cargo clippy -p arda-engine --all-targets --all-features -- -D warnings`:
  passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-engine --no-deps --all-features`:
  passed.
- `cargo check -p arda --all-targets --all-features`: passed.
- Missing-registry smoke from `/tmp`: exit 1 with a `services.toml` read error.
- Workspace-root smoke `target/debug/arda --once --no-ui`: exit 0 after
  resolving the canonical Manwe service and before spawning it.

## Remaining posture

No active crate-local implementation plan remains. Future additions must retain
the engine/root ownership boundary and add focused tests plus consumer evidence.
