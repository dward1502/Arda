# arda-outpost-protocol breakdown

## Scope

`arda-outpost-protocol` owns the versioned Rust and JSON contracts shared by
Arda outposts. It defines observation provenance, confidence, freshness,
classification, non-execution authority markers, scout-feedback conversion,
and a bounded in-memory topic queue.

It does not collect observations, persist memory, perform network transport,
approve work, or execute tasks. Those responsibilities belong to consumers.

## Supported source graph

All five Rust files under `src/` are compiled unconditionally; the crate has no
Cargo features, binaries, build script, generated includes, or unwired source.

| Path | Classification | Role |
| --- | --- | --- |
| `src/lib.rs` | production/default | Module declarations, schema constant, and public re-exports |
| `src/authority.rs` | production/default | Closed non-execution authority classes and display/wire behavior |
| `src/observation.rs` | production/default | Feedback, observation, scope, classification, builders, and conversion |
| `src/queue.rs` | production/default | Bounded topic queues, queue items, acknowledgements, retry metadata, generate/consume helpers |
| `src/error.rs` | production/default | Typed schema, identifier, payload, queue, and serialization errors |

All three Rust files under `tests/` are integration-test targets:

| Path | Classification | Proof |
| --- | --- | --- |
| `tests/observation_authority.rs` | test-only | JSON round trip, canonical and legacy wire values, malformed values, authority boundary |
| `tests/observation_feedback.rs` | test-only | Feedback conversion and schema mismatch rejection |
| `tests/queue_generate_consume.rs` | test-only | Producer/consumer flow, independent topic capacity, failed-ack retry |

## Contract inventory

- `SCHEMA_VERSION`: `arda.outpost.observation.v1`.
- `AuthorityClass`: `Advisory`, `Presentation`, `ExecutionProhibited`. Every
  class reports `permits_execution() == false`.
- `ObservationScope`: built-in crate, app, memory, health, environmental, and
  runtime-telemetry scopes plus an explicit custom scope.
- `ObservationClassification`: raw measurement, derived estimate, self report,
  default, unavailable, and experimental-derived states.
- `AgentFeedback`: source-bearing producer payload with schema, confidence,
  classification, authority, and JSON payload.
- `OutpostObservation`: identified and timestamped governed observation with
  freshness, confidence, provenance, and local-only metadata.
- `QueueTopic`: named routing key.
- `QueueItem`: queued observation with creation time and retry count.
- `QueueAck`: consumed item identifier and success/failure result.
- `OutpostQueue`: bounded per-topic queued/in-flight state.
- `generate_queue` / `consume_queue`: batch producer and consumer helpers.
- `OutpostProtocolError` / `OutpostQueueError`: typed boundary failures.

The live contract does not define manifests, chat, health-report, finding,
evidence, or dispatch envelope types. Packet 5 does not fabricate absent wire
surfaces; health is currently only an observation scope.

## Wire compatibility

Canonical enum output is snake_case and matches each type's display contract.
Legacy v1 PascalCase enum input remains accepted for stored observations.
Unknown scope, classification, and authority values are rejected. Schema
mismatches are rejected at feedback conversion and queue production boundaries.

## Dependencies and consumers

Normal dependencies are `chrono`, `parking_lot`, `serde`, `serde_json`,
`thiserror`, and `uuid`. There are no optional dependencies or feature modes.

`cargo tree -i arda-outpost-protocol --edges normal` identifies one direct Cargo
consumer: `arda-outpost-scout`. Scout converts survey feedback, creates research
and survey observations, queues protocol observations, and stores/reloads the
complete observation through `arda-vaire`.

## Runtime boundaries

The crate is library-only and performs no filesystem or network I/O. Queue state
is process-local, mutex-protected, and capacity-bounded independently per topic.
Failed acknowledgements requeue an item with incremented retry metadata.
