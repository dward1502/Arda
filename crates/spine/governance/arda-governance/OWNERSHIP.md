# arda-governance ownership

Crate: `crates/spine/governance/arda-governance`
Owner: HADES / governance layer
Status: active
Boundary: deterministic governance primitives, evidence assessment, scoring projections, and advisory environmental signals.

This crate owns:
- Triad/chain evaluation and versioned structured evidence assessment
- philosopher profile parsing, validation, and status projection
- resonance, Love Equation/Love Dynamics, JouleWork, and alignment scoring
- Bacon-Lite bounded persistent storage and in-process metrics snapshot
- typed audio/vision/solar governance signal evidence and advisory-only environmental coherence

This crate does not own:
- Prometheus exposition server
- autonomous consensus behavior
- policy enforcement outside returned verdicts
- claims of autonomous decision authority

Preferred consumer path:
- `arda-varda`/`arda-mandos` through governance chain / Triad interfaces
- `arda-orome` through governance hooks
- `arda-aule` through exported status/metrics surfaces
