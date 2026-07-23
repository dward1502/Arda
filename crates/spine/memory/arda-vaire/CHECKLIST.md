# arda-vaire — implementation checklist

This checklist tracks concrete work surfaced from crate review, testing, and the BREAKDOWN action items.

- [x] Verified crate metadata, public surface, and data contracts
- [ ] Update README with current runtime/check/test evidence
- [x] Confirm HTTP/SSE transport tests pass on the default `http` feature
- [x] Confirm IPC transport round-trip tests pass with default features off HTTP
- [ ] Run the knowledge-delta bridge scenario and confirm archetype coverage
- [ ] Add benchmark scaffold for recall fidelity vs. public memory systems
- [ ] Add integration coverage for missing-provider/non-JSON/unreachable/default Forwarder paths
- [ ] Enforce receipt-backed promotion regression coverage for governance guarantees
- [ ] Validate governance scoring under overload/duplicate/tag novelty conditions
- [ ] Add observability hooks for recall fidelity, queue latency, and consolidation depth
- [ ] Document store boundaries and ownership in service.rs and transport.rs
- [ ] Make confidence/trust explicit in downstream recall reports
