# arda-outpost-scout status

Crate: `outposts/arda-outpost-scout`
Current state: Packet 6 complete locally on 2026-07-29
Branch: `manwe`
Documentation: `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, `OWNERSHIP.md`

Current signature: a standalone library/binary leaf that performs bounded
repository survey and fixed-policy SearXNG research, emits source-bearing
advisory observations, and returns append-only Vairë memory receipt IDs without
queue, approval, promotion, model, or execution authority.

## Live integration

- No direct Cargo consumers.
- Root `arda-engine` harness proxies governed scout requests over HTTP.
- ARDA HUD reads Athena scout ledgers/runtime state as a partial evidence lane.
- Durable Warden→Athena/council projection production remains open plan work.
- Nine production Rust files and six integration-test targets are fully
  classified in `BREAKDOWN.md`.

## Packet 6 contract repairs

- Required the fixed `allowlisted_public_web` policy before network access.
- Required request expiry in the future and within 24 hours.
- Bounded queries to 512 bytes and retained results to 10.
- Rejected result sources without valid HTTP(S) URLs and hosts.
- Added append-only two-receipt proof and explicit no queue/approval artifact
  assertions without changing `src/memory.rs`.
- Preserved `src/memory.rs` and `tests/survey_fixtures.rs` with no task diff.

## Consumer and operational evidence

- Root harness focused proxy test passes and preserves query, limit, policy, and
  expiry fields.
- ARDA HUD focused projection test passes and renders request/finding source
  policy as a partial scout lane without review receipts.
- Live Warden verification: `arda-warden-scout.service` and
  `llama-server.service` are active; scout health on `:8092` reports advisory.
- A live two-result source-bearing research request was receipted and recalled
  with one memory ID, advisory authority, and two source URLs.
- The deployed service predates this local contract update; the local machine has
  no AArch64 Rust target, so current-source Pi redeployment is not claimed.

## Closeout evidence

- No-default and all-feature scout suites: 25 tests passed in each mode.
- All-target checks, rustfmt, strict Clippy, and warning-denied Rustdoc passed.
- Root `arda-engine` harness forwarding test and strict Clippy passed.
- ARDA HUD: 259 tests across 70 files passed and the production build succeeded.
- Live Warden health, one two-source request, and receipt recall succeeded.
- `src/memory.rs` and `tests/survey_fixtures.rs` retained their clean pre-task
  state.

The active Warden service predates this Packet's request-policy/expiry repair.
Current-source AArch64 rebuild/redeploy remains open in the Pi5 integration plan
and is not claimed by this package closeout.