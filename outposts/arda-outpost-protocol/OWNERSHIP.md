# arda-outpost-protocol ownership

## Owned here

- The `arda.outpost.observation.v1` schema identifier.
- Shared observation, feedback, queue, acknowledgement, and error types.
- Canonical JSON enum names and backward-compatible v1 enum decoding.
- Confidence clamping during typed construction.
- Schema checks at feedback-conversion and queue-production boundaries.
- Per-topic queue capacity and failed-ack retry metadata.
- The invariant that no observation authority class grants execution.

## Owned elsewhere

- Observation collection and research: `arda-outpost-scout` and later outposts.
- Durable observation memory and recall: `arda-vaire`, mediated by scout.
- Network/runtime transport: outpost runtime crates.
- Governance, approval, queue mutation outside this process-local observation
  queue, and execution: Arda governance/runtime owners.
- Presentation decisions: consuming UI surfaces.

## Authority boundary

`Advisory` may inform review, `Presentation` may support rendering, and
`ExecutionProhibited` explicitly marks a prohibited execution path. None of
these classes approves, rejects, dispatches, or executes work; all return false
from `AuthorityClass::permits_execution()`.

Adding an execution-capable authority class, changing `SCHEMA_VERSION`, or
removing legacy v1 decoding is a cross-consumer contract change and requires a
new focused plan plus direct-consumer migration evidence.
