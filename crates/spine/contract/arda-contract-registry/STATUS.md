# arda-contract-registry status

Crate: `crates/spine/contract/arda-contract-registry`
State: stable first-class read-only schema/loader
Last verified: 2026-07-28
Direct consumer: `arda-launcher`

## Current contract

- All 3 Rust files are wired: 2 default production modules and 1 integration
  test target.
- Parser/error tests use explicit temporary fixtures and do not depend on live
  workspace state.
- The integration smoke remains intentionally live and read-only: it verifies
  canonical schema, paths, and schema identifiers against the workspace.
- Launcher duplicate file reading/JSON parsing was replaced by the crate loader.
- Unused `glob-match` was removed.

## Verification evidence

- `cargo fmt -p arda-contract-registry -p arda-launcher -- --check`: passed.
- `cargo check -p arda-contract-registry --no-default-features`: passed.
- `cargo test -p arda-contract-registry --no-default-features -- --test-threads=1`:
  3 unit + 3 integration passed; 0 failed.
- `cargo check -p arda-contract-registry --all-targets --all-features`: passed.
- `cargo test -p arda-contract-registry --all-features -- --test-threads=1`:
  3 unit + 3 integration passed; 0 failed.
- `cargo clippy -p arda-contract-registry --all-targets --all-features -- -D warnings`:
  passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-contract-registry --no-deps --all-features`:
  passed.
- `cargo check -p arda-launcher --all-targets --all-features`: passed.
- `cargo test -p arda-launcher --lib -- --test-threads=1`: 8 passed; 0 failed.
- Canonical artifact SHA-256 before and after strict registry gates:
  `5ff66fddba546f9c0144839929bc2167beef61f73961c37474ca73a67b866d71`.

No active crate-local plan remains.

