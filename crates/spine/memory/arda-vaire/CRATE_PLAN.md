# arda-vaire implementation plan

Source: `crates/spine/memory/arda-vaire`, `CHECKLIST.md`, workspace consumers in `arda-varda`, `arda-governance`, `arda-mandos`, `arda-aule`.

## Canonical public path

- Crate root re-exports the typed HTTP/SSE and IPC forwards plus recall/store APIs.
- Primary integrations: `arda-varda` execution receipts; `arda-governance` evidence scoring; `arda-mandos` evidence/PageIndex inputs.

## Canonical contracts

- `Forwarder` is the transport contract for recall and knowledge-delta promotion.
- Receipt-backed promotion is the governance guarantee for memory promotion.
- Unreachable/default/non-JSON paths must return disclosed partial evidence, never fabricate facts.
- Confidence and trust are explicit in every recall report.

## Implementation work

1. Add integration coverage for unreachable/non-JSON/default forwarder states with disclosed partial results (`src/transport.rs`, test fixtures).
2. Run and verify the knowledge-delta bridge scenario with archetype coverage and recorded evidence.
3. Add benchmark scaffold measuring recall fidelity against public memory systems.
4. Add observability hooks for recall fidelity, queue latency, and consolidation depth.
5. Add financial/receipt-backed promotion regression coverage so governance guarantees survive memory pressure, duplicate tags, and novelty conditions.
6. Document store boundaries and ownership in `src/service.rs` and `src/transport.rs`.

## Open risk items

- Confidence/trust values remain implicit in downstream recall reports.
- No benchmark or live promotion regression evidence recorded since knowledge-delta coverage.
- Governance scoring under overload/duplicate/tag novelty is not yet regression-tested across long-running sessions.

## Status

Public path is intact; implementation coverage for `forwarder.rs`, `transport.rs`, and memory is incomplete.
