---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-aule
Observability home crate for Arda: intended host for prometheus/ceo/cli
surfaces; currently contains only a copied council blueprint stub.
Owner: arda | Sigil: 🜃 ANKH | Status: active

## Summary
`arda-aule` is intended to be the observability-area home for:
- `annunimas-prometheus` — executive orchestrator (`/core` linkage,
  pipeline, confidence scoring, council gate, orders/escalations,
  autopilot, cli commands)
- `annunimas-ceo` — CEO orchestration brain (decomposition, delegation,
  decision engine, learning)
- `annunimas-cli` — primary CLI entrypoint for running Annunimas locally
  with commands for athena, prometheus, charon, mnemosyne, hades, hermes,
  apollo, plutus, oracle, chronos, forge-mind, etc.

Currently none of those surfaces live under `arda-aule`. The crate only
contains a near-copy of `arda-council`'s blueprint, and its docs are stale:
`INDEX.md` and `README.md` are missing, and the `.bak` variants still
describe `Arda-Council`.

The actual implementations exist in parallel under
`/var/home/mythos/Annunimas/crates/` as standalone `annunimas-*` crates
that are **not** wired into the Arda workspace. They depend on each other
and on sibling `annunimas-*` crates, not on `arda-*` crates.

## Where things are today
- Arda workspace member: `crates/spine/observability/arda-aule`
- Current source: only `lib.rs`, `contract.rs`, `council.rs`, `service.rs`
  (council blueprint copy)
- Real implementations (outside Arda workspace):
  - `~/Annunimas/crates/annunimas-prometheus/`: ~60 source files, benches,
    tests, binaries
  - `~/Annunimas/crates/annunimas-ceo/`: thin wrapper around prometheus
  - `~/Annunimas/crates/annunimas-cli/`: full CLI with commands/,
    export_surface/, policy_guard, observability entry

## Verification status
- `cargo check -p arda-aule`: OK
- `cargo test -p arda-aule`: 3 integration + 3 doc tests passing (blueprint only)
- `~/Annunimas/crates/annunimas-*` are not buildable from Arda workspace
  because they reference `annunimas-*` deps not present in Arda `Cargo.toml`

## Agentic-OS abstractions that should live here
- **Prometheus pipeline**: receive task, estimate joule cost, score
  confidence, route/delegate, persist lifecycle decisions to ledger
- **CEO orchestration**: objective decomposition, agent delegation,
  council/oracle/warden triangulation, learning feedback loops
- **CLI surface**: operator commands for every service, policy-guard
  gating, export surfaces, observability entry points
- **Transport**: IPC + HTTP/SSE daemon for prometheus/ceo surfaces
- **Council-gate**: confidence adjustment scaffold for complex decisions
- **Core linkage**: bridge to `core/realm/boot.toml`, `core/state/world.json`
- **Orders/escalations**: append-only stores with active/pending counters

## Crate layout desired
| Module | Source | Role |
|--------|--------|------|
| `prometheus/` | `~/Annunimas/crates/annunimas-prometheus/src/*` | Executive orchestrator |
| `ceo/` | `~/Annunimas/crates/annunimas-ceo/src/*` | CEO brain |
| `cli/` | `~/Annunimas/crates/annunimas-cli/src/*` | CLI entrypoint |
| `contract.rs` | existing or new | Observability contract for Arda |
| `council.rs` | existing or shared | Re-export from `arda-council` |
| `service.rs` | new | Readiness probe tying prometheus/ceo/cli status |

## Ideas for improvement
1. **Migrate prometheus/ceo/cli into arda-aule**: copy or symlink the
   `~/Annunimas/crates/annunimas-*` sources under
   `crates/spine/observability/arda-aule/`, then rewrite their deps from
   `annunimas-*` to `arda-*` equivalents
2. **Retire annunimas-* naming in Arda**: replace all `annunimas-*` crate
   references with `arda-*` equivalents to align with active tree
3. **Keep council blueprint minimal**: once the real observability code
   moves in, reduce or remove the council copy; re-export from
   `arda-council` instead
4. **Fix stale docs**: generate real `INDEX.md`/`README.md` for `arda-aule`
   describing its actual contents after migration
5. **Update core/state contracts**: `business_intelligence_suite_contract.json`
   already points to `arda-aule`; ensure it matches the migrated surface
6. **Wire CLI commands into engine/HUD**: make `annunimas-cli` commands
   accessible via engine API or Tauri launcher
7. **Add workspace-level tests**: once migrated, add integration tests
   proving prometheus pipeline runs end-to-end from Arda workspace
