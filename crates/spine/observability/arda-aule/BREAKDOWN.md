---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-22"
---

# arda-aule
Observability home crate for Arda: host for prometheus, ceo, cli, telemetry, and export surfaces.
Owner: arda | Sigil: 🜃 ANKH | Status: active

## Summary
`arda-aule` is the observability-area home for:
- `prometheus/` — executive orchestrator (`/core` linkage, pipeline, confidence scoring, council gate, orders/escalations, autopilot, CLI integration)
- `ceo/` — CEO orchestration brain (decomposition, delegation, decision engine, learning)
- `cli/` — Arda CLI surface (`arda-cli`) for services such as athena, prometheus, charon, mnemosyne, hades, hermes, plutus, oracle, etc.
- `telemetry/` — telemetry config, tracing, and event wiring
- `export_surface/` — agents/contract surface for interoperability

These surfaces are kept inside `arda-aule` by explicit project decision.

## Where things are today
- Arda workspace member: `crates/spine/observability/arda-aule`
- Primary source surfaces: `src/prometheus/`, `src/ceo/`, `src/cli/`, `src/telemetry/`, `src/export_surface/`
- Cross-crate dependencies are wired through Arda equivalents:
  - `arda-core`, `arda-governance`, `arda-orome`, `arda-vaire`,
    `arda-mandos`, `arda-varda`, `arda-economics`
- `~/Annunimas/crates/` remains read-only reference; imported source has been rewritten toward Arda crate paths.

## Verification status
- `cargo check -p arda-aule`: passing as of 2026-07-22
- `src/lib.rs`: observability-home crate header and documentation surface

## Decisions
- Keep prometheus/ceo/cli/telemetry/export_surface surfaces in `arda-aule`.
- Migrate legacy `annunimas_*` imports to `arda-*` crate paths; defer unresolvable mappings in `DEPENDENCY_AUDIT.md`.
