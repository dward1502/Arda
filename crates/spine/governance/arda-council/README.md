---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  code_point: "U+1F728"
  role: "documentation"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-14"
---

> 🜃 Arda-Council: 📜 blueprint | owner: ARDA | status: active | reviewed: 2026-07-14

# arda-council

Sigil **ANKH** — the canonical blueprint for sovereign Arda agents.

`arda-council` is not a feature crate; it is the *reference contract* that
every new Arda agentic crate is expected to replicate. It defines the
governance and continuity baselines a sovereign agent must satisfy, plus a
small, dependency-free model for multi-agent council deliberation.

## What it provides

- **`contract`** — `ArdaCouncilContract`, the singleton describing required
  governance (triad / bacon-lite / joulework / love-equation / soterion
  trace) and continuity (task-ledger / memory-checkpoint / arda-visibility)
  baselines.
- **`council`** — `CouncilSeat` (7 seats), `QueryMode` (6 modes), and the
  `CouncilQuery` → `CouncilBrief` transform that resolves which seats
  participate and whether licensed-professional escalation is required.
- **`service`** — `status()` readiness probe and `build_brief()` helper.
- **`crate_identity()`** — stable `"arda-council"` identifier.

## Purpose

Multi-agent boardroom deliberation and consensus building — expressed as a
reusable blueprint rather than a running service. New crates copy this
structure so governance stays consistent across the tree.

## Quick example

```rust
use arda_council::contract::contract;
use arda_council::{council::{CouncilQuery, QueryMode}, service};

let c = contract();
assert_eq!(c.realm, "command");
assert!(service::status().governance_ready);

let brief = service::build_brief(&CouncilQuery {
    mode: QueryMode::FullCouncil,
    seats: vec![],
    prompt: "Should we ship this feature?".into(),
});
assert_eq!(brief.participating_seats.len(), 7);
assert!(brief.escalation_required);
```

## Design notes

- Escalation is *structural*: any brief involving `Attorney`, `Cfo`, or
  `TaxStrategist` sets `escalation_required = true`, surfacing the
  "consult a licensed professional" flag before the answer is used.
- The contract is data, not behavior — it serializes to JSON for the
  `core/state/arda-council.json` export and is asserted by
  `tests/contract_smoke.rs`.

## Dependencies

`serde`, `serde_json`, `chrono` — no Arda-internal crates, no async, no I/O.
This is why it lifts cleanly as a Tier-0 leaf in the migration.

## See also

- `src/contract.rs` — baseline definitions
- `src/council.rs` — seat / query model
- `src/service.rs` — status + brief builder
- `tests/contract_smoke.rs` — governance readiness checks
