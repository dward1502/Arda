# arda-orome status

Crate: `crates/spine/interface/arda-orome`
Version: `0.1.0`
State: **transport-complete, source-classified, and verified**
Reviewed: 2026-07-27
Required crate-local stabilization work: **complete**

## Current state

- The default six-family interface surface remains stable.
- The former 35-file unwired inventory is resolved: 29 source files were retained behind `service-runtime`, one compatibility module was added, and six unsupported files were retired.
- No Rust file is unwired: the all-feature tree has 47 production-compiled files and three unit-test-only files.
- The no-op default `http` feature and its unused optional dependencies were removed.
- Service/MCP/context/Discord-contract dependencies are optional and activated only by `service-runtime`.
- `intent` and `registry` support both unit tests and `service-runtime`; `message_retry_expiry`, `router`, and provider tests remain test-only.
- `HttpJsonTransport` provides opt-in live HTTP dispatch with bounded responses and provider-message receipt proof.
- Fleet policy defaults to local-only; trusted-fleet targets require an explicit provider allowlist and external scope requires approval.
- `HermesService` scopes Bacon-Lite, Mnemosyne, and Plutus evidence to its configured project root while retaining explicit home overrides.

## Runtime boundary

`service-runtime` compiles and tests the preserved resident-service closure and the live HTTP JSON transport. The resident-service compatibility adapter remains deterministic/no-network and does not report configured providers online without health evidence. Slack, Email, Matrix, and Discord clients remain outside this crate. Manwe retains provider/model routing authority.

## Verification evidence

Passed from the workspace root on 2026-07-27:

- `cargo fmt -p arda-orome -- --check`.
- `cargo check -p arda-orome --no-default-features`.
- `cargo test -p arda-orome --no-default-features`: 21 passed (14 unit, 7 integration).
- `cargo test -p arda-orome --all-features -- --test-threads=1`: 96 passed (86 unit, 10 integration).
- `cargo clippy -p arda-orome --all-targets --all-features --quiet -- -D warnings`.
- `cargo doc -p arda-orome --no-deps --all-features`.
- `cargo test -p arda-engine --test orome_smoke`: 1 passed.
- `cargo check -p manwe --features grpc`.
- `cargo check -p arda-aule --features full-cli`.

Cargo emitted only the existing workspace warning about the ignored non-root launcher profile.

## HERMES disposition

All crate-owned HERMES tasks are complete:

1. concrete transport and receipt projection: implemented by `HttpJsonTransport`, `TransportOutcome`, and `DispatchReceipt::delivery_proven()`;
2. fleet policy and edge evidence: documented as fail-closed and verified by three live-socket/denied-before-network integration tests;
3. bounded fanout and routing: retained and verified;
4. HUD expansion: remains correctly outside this crate under `apps/arda-hud` ownership.

The completed transient plan was removed from `docs/plans/` in accordance with
active-plan-only policy. The completed crate-local stabilization plan was also
retired after its durable decisions were absorbed into `README.md`,
`BREAKDOWN.md`, `STATUS.md`, and `OWNERSHIP.md`.
