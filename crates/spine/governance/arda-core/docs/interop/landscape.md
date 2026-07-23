# GEN3 landscape (learning/observability/interop)

Status: planning/landscape only. No behavior changes in `arda-core` until a consumer crate needs them.

## Current intercept surface
- Additive observability types were introduced in `src/loop_observability.rs`.
- Consumers may import them through `arda_engine::loop_observability`.
- These types do not change dispatch semantics or append-only ledger output.

## Constraints
- Interoperability depends on external tooling integration points.
- Merging behavior changes requires a concrete consumer usage scenario.

## Deferral rule
Do not merge behavior changes until GEN2 remains fully closed and a consumer
crate introduces a concrete interop scenario.
