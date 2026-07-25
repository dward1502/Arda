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
- `arda-aule` loop observability consumer:
  - `LoopCommands::Observability` reads `arda-core::loop_observability::LoopObservabilityConfig::from_env()`
  - Reports whether economy snapshots / latency probes are enabled
  - Does not change dispatch semantics
- `arda-aule` learning receipt consumer:
  - `LearningCommands::Ledger` builds `arda-core::learning_adapter::LearningLedgerReceipt`
  - Emits JSON under contract `arda.learning.interop.v1`
  - Consumer can pipe this into external tooling without depending on `arda-core` internals

## Open questions
- Should `arda-aule` consume live `LearningStore` data instead of default state?
- Which external tooling, if any, needs a stable serialization commitment
  beyond the current `arda.learning.interop.v1` receipt?

## Resolved
- `arda-engine` now exposes `EngineObservabilityStatus` as the higher-level
  aggregator for loop observability and learning interop state.
