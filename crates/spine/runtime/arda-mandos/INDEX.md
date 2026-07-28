# arda-mandos index

## Canonical documentation

- [`README.md`](README.md) — purpose, public surface, runtime interfaces, and operator commands.
- [`BREAKDOWN.md`](BREAKDOWN.md) — subsystem and source-file map.
- [`STATUS.md`](STATUS.md) — verified package state and exact closeout gates.
- [`OWNERSHIP.md`](OWNERSHIP.md) — authority, persistence, transport, and consumer boundaries.

## Source navigation

- [`src/lib.rs`](src/lib.rs) — canonical public re-exports.
- [`src/reasoning.rs`](src/reasoning.rs) — query contract, policy, gates, outcomes, engine history, and metrics projection.
- [`src/evidence.rs`](src/evidence.rs) — typed evidence provenance and integrity metadata.
- [`src/context.rs`](src/context.rs) — bounded public reasoning graph.
- [`src/pageindex.rs`](src/pageindex.rs) — stable document indexing and search.
- [`src/scoring.rs`](src/scoring.rs) — compatibility scoring APIs.
- [`src/notify.rs`](src/notify.rs) — Unicode-safe typed verdict formatting.
- [`src/service.rs`](src/service.rs) — authoritative persistence, recovery, verification, export, and telemetry delivery.
- [`src/transport/dispatch.rs`](src/transport/dispatch.rs) — shared typed transport dispatcher and structured errors.
- [`src/transport/ipc.rs`](src/transport/ipc.rs) — bounded Unix-socket transport.
- [`src/transport/http.rs`](src/transport/http.rs) — optional bounded HTTP/SSE transport.
- [`src/transport/mod.rs`](src/transport/mod.rs) — listener supervision and shutdown.
- [`tests/target_local.rs`](tests/target_local.rs) — target-local persistence and restart integration proof.
