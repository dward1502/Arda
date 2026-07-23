# arda-economics ownership

Crate: `crates/spine/runtime/arda-economics`
Owner: HADES / runtime layer
Status: active
Boundary: budget/tariff valuation, energy meter accounting, JouleWork provenance, Plutus mutation logging, IPC/HTTP transport.

This crate owns:
- economics engine mutation paths and budget/ROI/tariff valuation
- async energy meter contract with estimator fallback behavior
- explicit transport timeout/IPC/HTTP supervision
- `PlutusService` append-only runtime event writes

This crate does not own:
- Prometheus exposition server
- autonomous finance contingencies beyond conservative caller-defined overrides
- clinical/legal/financial certifications implied by cost projections

Preferred consumer path:
- `arda-varda` through dispatch/budget enforcement interfaces
- `arda-manwe` through adaptive constraint hooks
- `arda-aule` through metrics/status export surfaces
