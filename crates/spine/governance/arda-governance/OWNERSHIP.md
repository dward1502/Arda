# arda-governance ownership

Crate: `crates/spine/governance/arda-governance`
Owner: HADES / governance layer
Status: active
Boundary: deterministic governance primitives, evidence assessment, scoring projections, and advisory environmental signals.
Last reviewed: 2026-07-28

This crate owns:
- Triad/chain evaluation and versioned structured evidence assessment
- philosopher profile parsing, validation, and status projection
- resonance, Love compatibility/Love Dynamics, JouleWork, and alignment scoring
- realm/action policy validation and the sole runtime blocking decision authority
- Bacon-Lite bounded persistent storage and in-process metrics snapshot
- typed audio/vision/solar governance signal evidence and advisory-only environmental coherence

This crate does not own:
- Prometheus exposition server
- autonomous consensus behavior
- policy enforcement outside returned verdicts
- claims of autonomous decision authority
- provider routing, process lifecycle, or metrics HTTP transport

Preferred consumer path:
- `arda-varda`/`arda-mandos` through governance chain / Triad interfaces
- `arda-orome` through governance hooks
- `arda-aule` through exported status/metrics surfaces
- `arda-engine` through aggregate observability and counter projections
- `manwe` through receipted realm governance and runtime blocking decisions

Change authority:
- Public API/wire changes require consumer and compatibility-fixture review.
- Provenance-sensitive algorithm changes require `GOVERNANCE_PROVENANCE.md` review.
- Any expansion from advisory evidence to blocking authority requires explicit readiness,
  rollback, independent-review, and operator-control evidence.
