---
soterion:
  sigil: "FORGE"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 5 — Fleet + Backbone Topology Workstation

> 🜏 Soterion: Fleet now owns physical/runtime node topology, backbone status, installed-model and hardware evidence, and fleet source truth. Provider routing remains a linked summary for the Routing workstation.

## Scope

Phase 5 replaces Fleet's duplicated generic `Systems` and `Operations and Packages` stack with one topology-first focused workstation. It wires the six existing AULË fleet projections into the HUD bundle and Fleet view model rather than replacing their producer authority.

Implemented surfaces:

- canonical `systems_health` Fleet module rendered by `FleetFocusedWorkstationView`;
- compact topology/rack line with selectable nodes;
- node index and selected-node evidence detail;
- Fleet-owned lower instrument based on node/backbone pressure;
- existing bundle refresh authority exposed as a contextual native button.

## Focused information architecture

The workstation has four bounded regions:

1. a Fleet posture header with projected and reachable node totals;
2. a compact topology/rack line with reachability encoded by text, symbol, and class;
3. a selectable node index;
4. selected-node detail for reachability, class, enrollment, hardware, expected models, and backbone role.

Provider routing is deliberately limited to a linked count and the instruction to open Routing for detail. Fleet no longer renders a provider table, lane-ownership table, package inventory, or generic setup/storage panel. `systems_health` returns only the Fleet owner module.

## Source truth

The bundle now loads the six existing AULË projection files and gives each an independent source-reference state:

| Family | Canonical path | Rendering contract |
|---|---|---|
| runtime | `core/state/fleet_runtime.json` | fresh, stale, missing, or unavailable from source timestamp/read state |
| nodes | `core/state/fleet_nodes.json` | same; owns node count and selection data |
| models | `core/state/fleet_models.json` | same |
| health | `core/state/fleet_health.json` | same |
| hardware | `core/state/fleet_hardware.json` | same |
| backbone | `core/state/fleet_backbone.json` | same; identifies the primary backbone node |

Projection freshness is assessed against bundle observation time. Missing/read-failed sources never render as live. The workstation distinguishes a missing node projection from a successfully loaded zero-node projection.

## Authority preservation

AULË remains the producer authority. The frontend reads projections only and does not write fleet state. The existing `refreshBundle` callback remains the Fleet action path and is exposed through a semantic native `Refresh Fleet` button.

No suspected duplicate Fleet artifact is deleted in this phase; deletion remains gated on Phase 9 ownership and indirect-reference proof.

Phase 9 subsequently retained this imported/tested scene renderer as the canonical Fleet owner, removed its unrelated duplicate floating-layout helpers, and retired the disconnected legacy Fleet card implementation. See [`ORPHAN_RETIREMENT.md`](ORPHAN_RETIREMENT.md).

## Lower instrument

The lower Fleet instrument derives its node population from Fleet projection totals, not provider totals. Provider routing data cannot inflate Fleet node pressure. The existing source-path, observed-at, and freshness contract remains intact.

## Tests and verification

Phase 5 adds contracts for:

- loading all six fleet projections into the bundle;
- deriving selectable nodes and backbone ownership;
- independent source truth and missing-source handling;
- topology-first selection and node evidence;
- unavailable versus loaded-zero-node states;
- exclusion of provider and lane tables from Fleet;
- provider totals not changing the lower Fleet node instrument.

Verification on 2026-08-16:

- standalone TypeScript project build: passed;
- full HUD Vitest suite: 137 files, 558 tests passed;
- production frontend build: passed; 2,616 modules transformed.
- documentation local-link audit: 57 links checked, 0 broken;
- `git diff --check`: passed.

Evidence logs:

- [`evidence/phase5-vitest-20260816.log`](evidence/phase5-vitest-20260816.log)
- [`evidence/phase5-build-20260816.log`](evidence/phase5-build-20260816.log)
- [`evidence/phase5-links-20260816.md`](evidence/phase5-links-20260816.md)

## Qualification

No fresh native screenshot, native pointer interaction, or newly launched Tauri binary is claimed. Visual/native acceptance is limited to deterministic component tests, semantic native controls, TypeScript, and the production build in this environment.
