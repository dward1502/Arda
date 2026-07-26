# arda-vaire — implementation checklist

This checklist tracks concrete work surfaced from crate review, testing, and the BREAKDOWN action items.

- [x] Verified crate metadata, public surface, and data contracts
- [x] Update README with current runtime/check/test evidence (2026-07-25: default and no-default test suites plus benchmark)
- [x] Confirm HTTP/SSE transport tests pass on the default `http` feature
- [x] Confirm IPC transport round-trip tests pass with default features off HTTP
- [x] Run the knowledge-delta bridge scenario and confirm boardroom, human-context, edge-runtime, and continuity archetype coverage
- [x] Add benchmark scaffold for recall fidelity vs. public memory systems (`benches/recall_fidelity.rs`; controlled local baseline ready for equivalent external adapters)
- [x] Add integration coverage for non-JSON, unreachable, and default IPC forwarder paths; missing-provider is not applicable because this provider-free transport has no provider selection
- [x] Enforce receipt-backed promotion regression coverage for governance guarantees
- [x] Validate governance scoring under overload/duplicate/tag novelty conditions
- [x] Add observability hooks for recall fidelity, IPC queue latency, and consolidation depth
- [x] Document store boundaries and ownership in service.rs and transport/mod.rs
- [x] Make confidence/trust explicit in downstream recall reports

Verification evidence: `cargo test -p arda-vaire` (29 unit + 5 integration), `cargo test -p arda-vaire --no-default-features` (27 unit + 5 integration), and `cargo bench -p arda-vaire --bench recall_fidelity` (600/600 Hit@1 fixtures) passed on 2026-07-25.
