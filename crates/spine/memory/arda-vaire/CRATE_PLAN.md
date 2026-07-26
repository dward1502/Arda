# arda-vaire implementation plan

Source: `crates/spine/memory/arda-vaire`, `CHECKLIST.md`, workspace consumers in `arda-varda`, `arda-governance`, `arda-mandos`, `arda-aule`.

## Canonical public path

- Crate root re-exports the typed HTTP/SSE and IPC forwards plus recall/store APIs.
- Primary integrations: `arda-varda` execution receipts; `arda-governance` evidence scoring; `arda-mandos` evidence/PageIndex inputs.

## Canonical contracts

- IPC `send_command` and daemon handlers are the provider-free transport contract for recall and encode forwarding; there is no separate `Forwarder` type in the live crate.
- Receipt-backed promotion is the governance guarantee for memory promotion.
- Unreachable/default/non-JSON paths must return disclosed partial evidence, never fabricate facts.
- Confidence and trust are explicit in every recall report.

## Implementation work

1. [done] Cover unreachable/non-JSON/default IPC forwarding states with disclosed errors; missing-provider is inapplicable to the provider-free transport.
2. [done] Verify knowledge-delta bridge behavior across all memory-scope archetypes.
3. [done] Add a controlled Hit@1/latency benchmark scaffold, ready for equivalent public-system adapters.
4. [done] Add process-local observability for recall fidelity/latency, IPC queue latency, consolidation depth, and receipt totals.
5. [done] Add receipt-backed promotion and adaptive overload/duplicate/novelty regressions.
6. [done] Document service/store/promotion/retrieval and transport ownership boundaries.

## Open risk items

- The controlled local benchmark does not yet provide an equivalent-dataset comparison with public memory systems.
- Observability is in-process rather than exported to a durable metrics backend.
- Long-duration soak coverage remains future work; current overload coverage is deterministic and bounded.

## Status

Checklist implementation is complete with default/no-default Cargo test evidence and a compiled, executed benchmark. Cross-system benchmark adapters and durable metrics export remain future iterations, not missing crate contracts.
