---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-25"
---

# arda-aule
Observability home crate for Arda: contracts, governance metrics, telemetry, and the supported operator CLI.
Owner: arda | Sigil: 🜃 ANKH | Status: active

## Summary
`arda-aule` is the observability-area home for:
- `contract`, `service`, and `council` — stable observability contracts and compatibility types
- `governance_metrics` — typed governance snapshot rendering
- `telemetry` — feature-gated tracing and structured event wiring
- `arda-cli` — implemented telemetry-schema, receipt, governance metrics/status, and Plutus export commands

The imported `src/ceo/`, `src/prometheus/`, and internal `src/cli/` module trees are retained
as migration evidence but are not attached to the library graph. Their retired dependencies
and stale path contracts are not part of the supported `full-cli` release surface.

## Retained migration evidence
- Historical loop/learning command modules remain in the unattached internal CLI tree.
- They are not exported by the supported operator binary and do not constitute live interop contracts.

## Where things are today
- Arda workspace member: `crates/spine/observability/arda-aule`
- Active library surfaces: `src/contract.rs`, `src/service.rs`, `src/council.rs`,
  `src/governance_metrics.rs`, and `src/telemetry/`
- Active operator binary: `src/cli/main.rs`
- Cross-crate dependencies are wired through Arda equivalents:
  - `arda-core`, `arda-governance`, `arda-orome`, `arda-vaire`,
    `arda-mandos`, `arda-varda`, `arda-economics`
- `~/Annunimas/crates/` remains read-only reference; imported source has been rewritten toward Arda crate paths.

## Verification status
- `cargo check -p arda-aule --features full-cli --all-targets`: passing as of 2026-07-25
- `cargo test -p arda-aule --features full-cli`: 14 tests plus 2 doctests passing
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- Process-level tests execute `governance-metrics` and `governance-status` and validate JSON contracts.

## Decisions
- Keep observability contracts, governance metrics, telemetry, and the operator binary in `arda-aule`.
- Do not expose command variants whose runtime implementation is absent.
- Keep unattached imported monolith source as migration evidence until a separately approved
  extraction or retirement task; it is not a compatibility promise.
