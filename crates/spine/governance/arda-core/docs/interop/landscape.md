# GEN3 landscape (learning/observability/interop)

Status: planning/landscape only. No behavior changes in `arda-core` until a consumer crate needs them.

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

## Deferral rule
Do not merge behavior changes until GEN2 remains fully closed and a consumer
crate introduces a concrete interop scenario.
