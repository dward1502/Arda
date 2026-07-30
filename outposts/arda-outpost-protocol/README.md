# arda-outpost-protocol

`arda-outpost-protocol` is Arda's shared, versioned observation wire contract.
It provides source-bearing observations, explicit confidence/freshness and
provenance, non-execution authority markers, scout-feedback conversion, and a
bounded process-local topic queue.

The crate is a library contract. It does not collect data, persist memory,
perform network transport, approve work, or execute tasks.

## Public API

- `SCHEMA_VERSION`: current schema identifier, `arda.outpost.observation.v1`.
- `OutpostObservation`: identified, timestamped observation with source, scope,
  classification, authority, confidence, freshness, payload, provenance, and
  local-only metadata.
- `AgentFeedback`: producer-facing payload that converts into a governed
  observation after schema validation.
- `ObservationScope` and `ObservationClassification`: closed built-in contract
  values plus `ObservationScope::Custom` for named extension scopes.
- `AuthorityClass`: `Advisory`, `Presentation`, and `ExecutionProhibited`; no
  current class permits execution.
- `OutpostQueue`, `QueueTopic`, `QueueItem`, and `QueueAck`: bounded per-topic
  in-memory generation, consumption, acknowledgement, and failed-item retry.
- `generate_queue` and `consume_queue`: batch helpers over `OutpostQueue`.
- `OutpostProtocolError` and `OutpostQueueError`: typed schema, payload, queue,
  identifier, and serialization failures.

## Wire contract

New JSON uses snake_case enum values such as `runtime_telemetry`,
`experimental_derived`, and `execution_prohibited`. Existing v1 observations
with PascalCase enum values remain readable. Unknown enum values are rejected,
and schema mismatches are rejected at feedback-conversion and queue-production
boundaries.

Changing `SCHEMA_VERSION`, authority semantics, or compatibility decoding
requires coordinated migration of direct consumers.

## Queue behavior

Queue capacity is enforced independently per topic across queued and in-flight
items. Successful acknowledgement removes in-flight work. Failed
acknowledgement requeues the observation and increments retry metadata. The
queue is process-local and is not Arda's governance or execution queue.

## Consumer and runtime

`arda-outpost-scout` is the sole direct Cargo consumer. It creates governed
survey/research observations and persists complete observation records through
its `arda-vaire` bridge.

The protocol crate has no Cargo features, binaries, build script, filesystem
state, environment variables, sockets, or network endpoints.

## Verification

From the workspace root:

```bash
cargo fmt -p arda-outpost-protocol -- --check
cargo check -p arda-outpost-protocol --no-default-features
cargo test -p arda-outpost-protocol --no-default-features
cargo check -p arda-outpost-protocol --all-targets --all-features
cargo test -p arda-outpost-protocol --all-features
cargo clippy -p arda-outpost-protocol --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p arda-outpost-protocol --no-deps --all-features
cargo check -p arda-outpost-scout --all-targets --all-features
```

See `BREAKDOWN.md` for the complete source/type map, `STATUS.md` for current
gate evidence, `OWNERSHIP.md` for authority boundaries, and `INDEX.md` for
navigation.

Packet 5 closeout passed 11 protocol integration tests in both Cargo feature
modes, strict Clippy and Rustdoc, and the direct consumer's all-target check and
20-test all-feature suite.
