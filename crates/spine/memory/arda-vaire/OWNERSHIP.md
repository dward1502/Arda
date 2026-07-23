# arda-vaire ownership

Crate: `crates/spine/memory/arda-vaire`
Owner: HADES / memory layer
Status: active
Boundary: knowledge-delta transport, recall/store mechanics, and governance-backed memory promotion.

This crate owns:
- recall fidelity, knowledge-delta promotion, and memory store behavior
- forwarder contract and disclosed partial evidence on degraded/default/non-JSON receipt
- receipt-backed promotion regression guarantees

This crate does not own:
- global governance scoring policy
- external persistence transport mechanics for softer telemetry
- unbacked authority over provenance truth

Preferred consumer path:
- `arda-varda` through memory/forwarder interfaces
- `arda-governance` through evidence input contracts
