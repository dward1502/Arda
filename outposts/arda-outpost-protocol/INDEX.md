# arda-outpost-protocol index

## Canonical documents

- [`README.md`](README.md) — mission, public API, wire/runtime contract, and verification commands.
- [`STATUS.md`](STATUS.md) — current first-class status and latest gate evidence.
- [`BREAKDOWN.md`](BREAKDOWN.md) — exhaustive source graph, type inventory, dependencies, and consumer wiring.
- [`OWNERSHIP.md`](OWNERSHIP.md) — schema, runtime, and authority ownership boundaries.

## Source entry points

- [`Cargo.toml`](Cargo.toml) — package metadata and unconditional dependencies.
- [`Cargo.lock`](Cargo.lock) — standalone outpost build lockfile.
- [`src/lib.rs`](src/lib.rs) — crate root and public exports.
- [`src/authority.rs`](src/authority.rs) — non-execution authority markers.
- [`src/observation.rs`](src/observation.rs) — feedback and observation contracts.
- [`src/queue.rs`](src/queue.rs) — bounded topic queue contract.
- [`src/error.rs`](src/error.rs) — typed protocol errors.

## Test entry points

- [`tests/observation_authority.rs`](tests/observation_authority.rs)
- [`tests/observation_feedback.rs`](tests/observation_feedback.rs)
- [`tests/queue_generate_consume.rs`](tests/queue_generate_consume.rs)
