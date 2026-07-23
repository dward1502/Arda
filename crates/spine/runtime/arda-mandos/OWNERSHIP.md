# arda-mandos ownership

Crate: `crates/spine/runtime/arda-mandos`
Owner: HADES / Oracle runtime
Status: active
Boundary: explainable decision-support oracle with auditable verdicts, typed evidence, reasoning context, and bounded persistence.

This crate owns:
- typed query validation and policy-gated outcome decisions
- versioned policy thresholds, veto rules, conditional bands, and evidence caps
- evidence provenance (`EvidenceRef`), reasoning context, and stable `pageindex://` references
- auditable persistence of verdicts before exposure

This crate does not own:
- autonomous consensus authority
- chain-of-thought LLM model traces stored as hidden reasoning
- external telemetry owner; side effects are bounded best-effort only

Preferred consumer path:
- `arda-aule` through operator/CLI/HUD interfaces
- `arda-mandos` root re-exports as canonical public path
