# arda-mandos ownership

Crate: `crates/spine/runtime/arda-mandos`
Owner: HADES / Oracle runtime
Status: active
Boundary: explainable advisory decision-support oracle with auditable verdicts, typed evidence, bounded public reasoning context, and integrity-verifiable persistence.

This crate owns:
- typed query validation and policy-gated outcome decisions
- versioned policy thresholds, veto rules, conditional bands, and evidence caps
- evidence provenance (`EvidenceRef`), reasoning context, and stable `pageindex://` references
- versioned, ordered, digest-linked persistence of verdicts before exposure
- restart hydration, degraded-prefix recovery, ledger verification, and verified atomic export
- the shared direct/IPC/HTTP dispatch contract, transport limits, listener supervision, and structured Mandos errors
- bounded low-cardinality gate and best-effort telemetry delivery counters

This crate does not own:
- autonomous consensus authority
- chain-of-thought LLM model traces stored as hidden reasoning
- external telemetry owner; side effects are bounded best-effort only
- consumer-side presentation, Prometheus exposition, execution, approval, or ledger repair

Consumer boundaries:
- `arda-aule` consumes Mandos through operator/CLI/HUD orchestration surfaces
- `arda-orome` consumes Mandos under its `service-runtime` feature
- `arda-governance` is an upstream dependency used by Mandos, not a Mandos consumer
- `arda-mandos` crate-root re-exports are the canonical Rust API path
