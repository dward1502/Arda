# arda-orome HERMES Execution Checklist

Source: combined from `crates/spine/interface/arda-orome/BREAKDOWN.md` and `docs/plans/HERMES.md`

Items with [ ] pending; [~] in progress; [x] done.

## Baseline (verified)
- [x] Read live source/crate state
- [x] `cargo check -p arda-orome` passes
- [x] `cargo test -p arda-orome` passes (7 tests, 0 failures)
- [x] grep actual `arda-orome` import consumers across workspace (6 crates/consumers)
- [x] Document cross-references resolved (`HERMES.md`, `CHARON.md`, `INDEX.md`)
- [x] Crate manifest repaired: added `serenity`, `async-trait`
- [x] Module registration fixed (`service.rs` duplicate removed, `service::` pkg retained)

## Hermes open tasks from docs/plans/HERMES.md
- [ ] richer provider adapters and live streaming surfaces
- [ ] strengthen fanout and routing orchestration
- [ ] expand edge-worker and fleet communication policy
- [ ] broaden ARDA HUD consumption of core and human plan surfaces

## Breakdown priorities
- [ ] add unit/integration coverage: router retry/expiry, intent classification, MCP governance
- [ ] unify duplicate message abstractions/canonical types
- [ ] replace `'static str` labels with typed enums
- [ ] make registry/router state sharable via core runtime trait
- [ ] persist `MessageQueue`/agent registry state
- [ ] normalize governance hooks centrally
- [ ] typed approval/interruption envelopes backed by ledger writes
- [ ] replace static Lazy context cache with bounded async cache
- [ ] wire one interface package into engine/CLI as live smoke path

## Immediate first-step
- [x] capture this checklist
- [x] inspect `src/lib.rs` module registration vs surfaced items
- [x] add first unit tests for router retry/expiry
- [x] `cargo test -p arda-orome` green with 7 tests

## Live evidence
- `crates/spine/interface/arda-orome/src/intent.rs:162` — 2 intent tests
- `crates/spine/interface/arda-orome/src/message_retry_expiry.rs` — 5 new tests
- `src/lib.rs` — `#[cfg(test)]` module entries for test-only coverage
