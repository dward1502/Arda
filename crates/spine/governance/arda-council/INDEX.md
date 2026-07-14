---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  code_point: "U+1F728"
  role: "directory_index"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-14"
---

> 🜃 Arda-Council: 📜 directory_index | owner: ARDA | status: active | reviewed: 2026-07-14

# Index: crates/spine/governance/arda-council

Sigil **ANKH** — blueprint crate for sovereign Arda agents.

## Purpose

Canonical reference contract defining the governance and continuity
baselines every Arda agentic crate must replicate, plus a 7-seat council
deliberation model. Lifted from `annunimas-council` during the spine
migration; self-contained (no internal deps).

## Layout

- `Cargo.toml` — package manifest (`arda-council`, deps: serde / serde_json / chrono)
- `README.md` — overview, example, design notes
- `INDEX.md` — this file
- `src/`
  - `lib.rs` — module exports, `crate_identity()`, doc example
  - `contract.rs` — `ArdaCouncilContract`, governance + continuity baselines
  - `council.rs` — `CouncilSeat`, `QueryMode`, `CouncilQuery`, `CouncilBrief`
  - `service.rs` — `status()`, `build_brief()`
  - `README.md`, `INDEX.md` — per-directory docs
- `tests/`
  - `contract_smoke.rs` — governance-readiness + escalation tests
  - `README.md`, `INDEX.md` — per-directory docs

## Surface API

| Symbol | Kind | Summary |
|--------|------|---------|
| `contract()` | fn | Returns the canonical `ArdaCouncilContract` |
| `ArdaCouncilContract` | struct | crate name, realm, productizable, state export path, baselines |
| `GovernanceBaseline` | struct | triad / bacon-lite / joulework / love-equation / soterion |
| `ContinuityBaseline` | struct | task-ledger / memory-checkpoint / arda-visibility |
| `CouncilSeat` | enum | 7 seats (Economist … Operator) |
| `QueryMode` | enum | 6 modes (Single/Dual/Full/DevilsAdvocate/Stress/Review) |
| `CouncilQuery` | struct | mode + seats + prompt |
| `CouncilBrief` | struct | resolved seats + escalation flag + required outputs |
| `status()` | fn | readiness probe (`governance_ready`) |
| `build_brief()` | fn | `CouncilBrief::from_query` wrapper |
| `crate_identity()` | fn | returns `"arda-council"` |

## Notes

- Tier-0 leaf: 0 internal Arda dependencies.
- State export target: `core/state/arda-council.json`.
- Review cadence: quarterly unless owner changes.
