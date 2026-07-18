---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "governance_blueprint"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-council
Blueprint crate for sovereign Arda agents: governance baseline, continuity
contract, and 7-seat council deliberation model.
Owner: arda | Sigil: 🜃 ANKH | Status: active

## Summary
`arda-council` is the canonical reference contract for new Arda agents. It
defines what governance and continuity commitments every sovereign agent
must make, plus a small dependency-free deliberation surface with 7 seats
and 6 query modes. It is a Tier-0 leaf: 0 internal Arda dependencies,
so it lifts cleanly anywhere.

Currently no consumer in `arda-engine` or `apps` imports this crate; its
value is normative rather than runtime-integrated in the main tree.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/governance/arda-council`
- Key files: `src/lib.rs`, `src/contract.rs`, `src/council.rs`, `src/service.rs`
- State export target: `core/state/arda-council.json`

## Agentic-OS relevant abstractions
- `ArdaCouncilContract` — required baseline contract:
  - governance: triad, bacon-lite, joulework, love-equation, soterion trace
  - continuity: task-ledger linkage, memory checkpoint expectation,
    arda visibility definition
- `CouncilSeat` — 7 distinct roles:
  - Economist, Attorney, Cfo, TaxStrategist, ContractSpecialist, Strategist,
    Operator
- `QueryMode` — 6 deliberation modes:
  - SingleSeat, DualSeat, FullCouncil, DevilsAdvocate, ScenarioStressTest,
    DocumentReview
- `CouncilBrief` — resolved seat set + structured outputs:
  - `seat_opinions`
  - `points_of_agreement`
  - `points_of_tension`
  - `synthesis_recommendation`
  - `licensed_professional_escalation_flag`
- Structural escalation gate: Attorney/Cfo/TaxStrategist automatically set
  `escalation_required = true`

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Module exports + `crate_identity()` + doc example |
| `contract.rs` | Canonical `ArdaCouncilContract`, validation `contract()` fn |
| `council.rs` | Seat/mode enums + query-to-brief transformer |
| `service.rs` | `status()` readiness probe, `build_brief()` helper |

## Verification
- `cargo check -p arda-council`: OK, 0 warnings
- `cargo test -p arda-council`: 3 integration + 3 doc tests passing
- No consumers detected in `crates/engine` or `apps`
- Contract serializes to JSON path `core/state/arda-council.json`

## Key source links
- `crates/spine/governance/arda-council/src/contract.rs`
- `crates/spine/governance/arda-council/src/council.rs`
- `crates/spine/governance/arda-council/src/service.rs`

## Ideas for improvement (agentic-OS angle)
- **Policy mode on `CouncilBrief`**: add a `PolicyMode` field or derive it
  from `arda-core`’s `GovernancePolicyMode` so the council respects the
  active policy rather than being hardwired.
- **Escalation policy enum**: replace boolean `escalation_required` with
  enum (`None`, `LicensedProfessional`, `ArdaCoreGovernance`, `Halted`) to
  differentiate reasons and integrate with `arda-core` governance gates.
- **Baseline versioning**: add a `contract_version` to `ArdaCouncilContract`
  so crates can declare which baseline they satisfy without parsing full
  contract JSON.
- **Seat output schema**: make `required_outputs` stored in config or a
  dedicated table so new output schemas don’t require code changes.
- **Persistence hooks**: currently only status/brief builder; add trait
  `CouncilPersistence` with ledger/memory writes so deliberation results
  actually persist.
- **Simulation score for `DevilsAdvocate`/`ScenarioStressTest` modes**:
  return a numeric stress score alongside opinions so HUDs can visualize
  confidence under contrarian or stressed assumptions.
- **Workspace wiring**: no current consumer; add `engine` or `cli` smoke
  integration proving council briefs flow through manwe/HADES paths and
  appear in telemetry.
- **Philosophical triad variances**: extend `QueryMode` with explicit
  philosopher-specialist seats tied to `arda-core`’s `PhilosopherVerdict`
  (`aurelius`, `bacon`, `sun_tzu`) instead of generic `Strategist`.
- **Continuity baselines as tests**: add regression tests for each
  baseline field so any drift requires a contract update.
- **`required_outputs` lifetimes**: `&'static str` limits extensibility;
  replace with `Cow<'static, str>` or store a registry for dynamic output
  types.
