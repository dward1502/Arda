# GEN3 landscape (learning/observability/interop)

Status: evidence-backed additive surfaces are live. Further behavior changes
remain gated on concrete consumer needs.

## Considerations
- Learning/memory adaptation: `learning.rs` and `state.rs` already expose shared
  primitives for routing bias, best-agent selection, and memory round trips.
  Any external learning concepts should map onto these contracts rather than
  replace them.
- Observability: append-only auditability in `loop_engine.rs`, `ledger.rs`, and
  `loop_alerts.rs` is the current interface. Any new knobs should preserve
  JSONL append semantics and not bypass governance gates.
- Interoperability views: `Message`, `Decision`, `ServiceRecord`, and
  `SoterionRegistryEntry` are the most likely external-view candidates.
  Review by downstream crates before introducing stable serialization guarantees.

## Evolution rule
GEN2 is closed. Preserve append-only auditability and current dispatch
semantics; merge additional behavior only for a documented consumer scenario.

## Concrete consumer scenarios
- `arda-engine` imports `LoopObservabilityConfig`, `LearningStore`, and
  `build_learning_ledger_receipt(...)` in its compiled `EngineObservabilityStatus` projection.
- `arda-aule` consumes service-registry, loop-alert, state, task, and Soterion contracts in its
  `full-cli` feature graph; it no longer owns the aggregate loop/learning projection.

## Open questions
- Which external tooling, if any, needs a stable serialization commitment
  beyond the current `arda.learning.interop.v1` receipt?

## Resolved
- `arda-engine` now exposes `EngineObservabilityStatus` as the higher-level
  aggregator for loop observability and learning interop state.
