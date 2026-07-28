# arda-economics ownership

Crate: `crates/spine/runtime/arda-economics`
Owner: HADES / runtime layer
Status: stable
Boundary: budget/tariff valuation, energy meter accounting, JouleWork provenance, Plutus mutation logging, IPC/HTTP transport.

This crate owns:
- economics engine mutation paths and budget/ROI/tariff valuation
- async energy meter contract with estimator fallback behavior
- typed meter fallback, tariff freshness, and IPC/HTTP request-latency accounting
- `PlutusService` append-only runtime event writes

This crate does not own:
- Prometheus exposition server
- autonomous finance contingencies beyond conservative caller-defined overrides
- clinical/legal/financial certifications implied by cost projections

Direct Cargo consumers:
- `arda-mandos`
- `arda-vaire`
- `arda-varda`

`arda-aule`, `arda-governance`, Manwe, and launcher may consume projected state
indirectly but do not currently depend on this crate.
