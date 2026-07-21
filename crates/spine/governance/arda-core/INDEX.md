# Index: crates/spine/governance/arda-core

The foundational Spine governance crate for Arda. It owns the shared spine
surface: contract versioning, routing, message types, learning primitives,
registry types, and the unified Arda error type.

- `Cargo.toml`
- `src`
  - `lib.rs` — unified exports for the Arda spine
  - `agent.rs`, `config.rs`, `contract.rs`, `daemon.rs`, `error.rs`,
    `governance.rs`, `ledger.rs`, `llm.rs`, `message.rs`, `router.rs`,
    `soterion.rs`, `state.rs`, `task.rs`, `tool.rs` — focused spine modules
  - `service_registry.rs` — folded standalone registry crate

## Purpose (one line)
Shared spine surface for the Arda governance bus.

## Notes
`arda-service-registry` functionality is folded into this crate via the
`service_registry` module. No separate workspace crate is needed.
