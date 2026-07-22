# arda-orome HERMES Execution Checklist

Source: combined from `crates/spine/interface/arda-orome/BREAKDOWN.md` and `docs/plans/HERMES.md`

Items with [ ] pending; [~] in progress; [x] done.

## Baseline (verified)
- [x] Read live source/crate state
- [x] `cargo check -p arda-orome` passes
- [x] `cargo test -p arda-orome` passes (14 tests, 0 failures)
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
- [x] add unit/integration coverage: router retry/expiry, intent classification, MCP governance
- [x] unify duplicate message abstractions/canonical types across crate
- [x] replace `'static str` labels with typed enums
- [x] make registry/router state sharable via core runtime trait
- [x] persist `MessageQueue`/agent registry state
- [x] remove broken `arda_plutus` test import / fix async-fn test compile path
- [x] cleanup unused-import warnings across `arda-orome` and `arda-core`
- [ ] normalize governance hooks centrally
- [ ] typed approval/interruption envelopes backed by ledger writes
- [x] replace static Lazy context cache with bounded async cache
- [ ] wire one interface package into engine/CLI as live smoke path

## Immediate first-step
- [x] capture this checklist
- [x] inspect `src/lib.rs` module registration vs surfaced items
- [x] add first unit tests for router retry/expiry
- [x] `cargo test -p arda-orome` green with 14 tests
- [x] add missing `service/mod.rs` re-exports for `ProviderRuntime`, `ProviderConfig`, `ProviderType`, `DispatchReceipt`
- [x] add missing provider `runtime.rs` types with `ProviderType` references + `Default`
- [x] patch `service/runtime.rs` to remove `StdMutex` dependency

## Repairs
- [x] register missing `provider::runtime` module from `provider/mod.rs`
- [x] implement missing `ProviderRuntime`, `ProviderConfig`, `ProviderType`, `DispatchReceipt`
- [x] replace `StdMutex` in `service/runtime.rs` with `Mutex`
- [x] cleanup stray `mcp.rs` attempts to keep module graph coherent

## Live evidence
- `src/provider/runtime.rs` — canonical `ProviderRuntime`, `ProviderConfig`, `ProviderType`, `DispatchReceipt`
- `src/service/mod.rs` — public shortcut re-exports from `provider::runtime`
- `src/service/runtime.rs` — switched `StdMutex` -> `Mutex`
- `src/provider/tests.rs` — expanded coverage: runtime defaults, kind mapping, dispatch receipt safety, adapter round trip
- `src/context_cache.rs` — added `AsyncContextCache<K,V>` with bounded async get/put/metrics
- `src/mnemosyne_integration.rs` — replaced `Lazy<Arc<Mutex<...>>>` with `Lazy<AsyncContextCache<...>>`
