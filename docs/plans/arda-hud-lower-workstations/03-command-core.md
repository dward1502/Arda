---
soterion:
  sigil: "SCROLL"
  role: "control_surface_audit"
  owner: "HERMES"
  status: "implemented_phase_2"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Command Core audit | owner: HERMES | status: Phase 2 implemented | reviewed: 2026-08-16

# Lower Surface 03 — Command Core

## Audit status

> Implementation update: Phase 2 relocated Settings, Hermes CLI, and Hermes Dashboard to the Command Core front plate and retired the detached utility row. See [`COMMAND_CORE_CONTROLS.md`](COMMAND_CORE_CONTROLS.md). The audit evidence below records the pre-implementation state and remains useful as provenance.

- Investigation date: 2026-08-16.
- Authority standard: current code, persisted state, current projections, and actual event routing.
- This section is the pre-implementation audit that authorized the bounded Phase 2 change.

## Physical role

```text
boardroom.control.center
  -> desk_3
  -> command_core_now
  -> fixed, not slot-configurable
```

Unlike the four lower monitors, the center is intentionally a physical intervention console. It has:

- one animated command-core signal screen;
- four tactile control buttons beside it;
- no persisted workstation slot assignment;
- no ordinary tab strip until one of its controls opens another focused workstation.

This distinction should be preserved in the later design pass.

## What the center surface displays

`CommandCoreInstrumentScreen.tsx` draws a 512×256 canvas texture containing:

- concentric phosphor/radar grid rings;
- sixteen radial spokes;
- animated ASCII glyph columns;
- rotating sweep and segmented rings;
- two synthesized waveforms;
- a central heartbeat;
- warning-colored glitches when derived attention is high.

The drawing is structurally aligned with the intended near-textless lower-instrument doctrine. It is not a DOM dashboard.

## Data encoded by the signal

The command-core `HudInstrumentModel` is built from only two numeric values:

```text
commandLanes
  = number of top-level keys in bundle.operationsFlow

attentionLanes
  = reviewGateItems whose status is neither approved nor rejected
```

Those become:

- glyph: `attentionLanes/commandLanes`;
- pressure: `commandLanes / 8`, clamped;
- warning count: up to two pending-review warnings;
- animation seed and cadence.

The animated instrument then derives:

- intensity;
- attention;
- coherence;
- cadence;
- colors and waveform shape.

The animation does **not** directly encode task urgency, appointments, agent activity, household state, failures, or operator priorities. It is a decorative transformation of two broad counts.

## Source path and freshness

The daily-command provenance family searches for:

- `operations_flow`;
- `operator_actions`;
- `arda_snapshot`.

Fallback paths are:

- `core/state/operations_flow.json` — currently missing;
- `core/state/operator_actions.json` — present, generated 2026-07-14.

`bundle.operationsFlow` can still exist because `ardaSource.ts` derives a fallback operations-flow object from queue summary when the dedicated file is absent. Therefore the command core can show nonzero “lanes” while its declared daily-command source family is partially missing.

The provenance adapter reduces all matching records to the worst freshness state. It can correctly mark the family missing/stale, but the physical animation does not visibly explain which source is absent or that the operations flow is derived.

## Declared Sovereign World sources

Clicking the main command screen routes to `sovereign_world`. The current persisted source map declares:

| Source | State on 2026-08-16 | Generated timestamp |
|---|---|---|
| `core/state/world.json` | Present | No generated timestamp in document |
| `core/state/system_manifest.json` | Present | 2026-07-14 |
| `core/state/system_control.json` | Present | 2026-07-14 |
| `core/state/active_ruleset.json` | Present | Selected/expiry fields rather than generated time |
| `core/state/autonomy_runtime.json` | Present | Updated 2026-07-14 |

The persisted source map and the source-code fallback blueprint disagree:

- persisted map panels: `3d_world`, `executive_overview`;
- fallback blueprint panels: `executive_overview`, `systems`;
- first-level workstation profile adapts Sovereign World to `executive_overview`, `systems`.

This is another authority overlap to resolve later.

## What clicking the main screen opens

The `open_command_core` control is read-only and targets `sovereign_world`.

The focused-workstation builder has a special branch for `sovereign_world` that replaces the generic `systems` module with the inline `FleetFocusedWorkstationView`, then adds supplemental manifest modules such as Executive Overview.

The practical result is approximately:

1. Fleet
2. Executive Overview

That is not a dedicated “Now” or intervention workstation, and it makes clicking the command core unexpectedly enter another fleet/system surface.

The global World View itself is a separate scene transition through the `enter_world` button. The main command screen's `sovereign_world` workstation route and the World View scene therefore use the same domain name for two different experiences.

## Physical control buttons

| Control | Authority label | Actual behavior | Mutation performed? |
|---|---|---|---|
| Approval Queue / GO | `read_only` | Opens `planning_and_queue` workstation | No |
| Gated Stop / Cancel Review / STOP | `approval_required` | Opens `governance_guardhouse` workstation | No |
| Route Selector / ROUTE | `operator_confirmed` | Opens `routing_and_comms` workstation | No |
| World View / WORLD | `operator_confirmed` | Enters the World View scene | Scene transition only |

The controls call `deriveBoardroomPhysicalControlState()` and `resolveBoardroomPhysicalControlInteraction()`. Except for Service Health elsewhere on the console, known controls are not disabled based on live capability state:

- `approval_required` becomes an enabled `CONFIRM` visual state;
- other controls become enabled `READY` states;
- the interaction immediately dispatches the route/open action.

No confirmation dialog or approval receipt is created by pressing STOP. This is safe in the narrow sense that no stop mutation occurs, but the control label and authority imply a capability that does not exist at that surface.

## Current truth classification

| Element | Classification |
|---|---|
| Command-core wave/radar animation | Live animation from snapshot/derived counts |
| Operations-flow lane count | Derived projection when dedicated file is absent |
| Pending review count | Bundle snapshot from review-gate derivation |
| Source freshness | Available in the model, not legible on the physical surface |
| GO | Navigation shortcut |
| STOP | Navigation shortcut, not stop/cancel execution |
| ROUTE | Navigation shortcut, not route selection |
| WORLD | Real scene navigation |
| Main screen | Opens mismatched Sovereign/Fleet workstation |

## Existing strengths

1. The center is correctly non-configurable.
2. It is visually distinct from the four lower monitor instruments.
3. It uses animated canvas/WebGL presentation rather than a DOM card.
4. It provides tactile shortcuts to bounded domains rather than embedding large text panels.
5. The button contracts declare authority and verification-path metadata.

## Current defects

1. The visual signal is semantically too weak: only two broad counts drive it.
2. Derived, stale, and missing daily-command sources are not legible at the console.
3. The main screen opens a Fleet/Executive workstation rather than a coherent command surface.
4. `sovereign_world` means both a focused workstation domain and a separate World View scene.
5. STOP does not stop or cancel; it only navigates to Governance.
6. ROUTE does not select a route; it only navigates to Routing.
7. Approval Queue targets Planning and Queue rather than the Governance approval list audited in Monitor 1.
8. “READY” and “CONFIRM” describe static action contracts, not verified target readiness.
9. Source-map, fallback-blueprint, and first-level profile definitions disagree about Sovereign World panels.

## Intended information responsibility to retain

The center console should remain a tactile intervention surface rather than another information workstation. Its likely responsibilities are:

- what needs attention now;
- current command/agent posture;
- explicit navigation to approvals, routing intervention, and world observation;
- a safe stop/cancel entry point whose actual mutation remains governed elsewhere;
- unmistakable unavailable/stale/blocked states;
- concise receipts after intervention.

It should not duplicate full Governance, Fleet, Routing, Planning, or World View detail.

## Cross-reference candidates

Do not resolve until all records exist:

- GO destination versus Governance review/approval ownership.
- Fleet health summary versus Fleet workstation.
- Route intervention versus Routing workstation.
- STOP/cancel review versus Governance and task execution controls.
- Daily-command/Now data versus Planning, Human/Personal, and agent canvases.
- `sovereign_world` workstation versus display-only World View.
- Executive Overview versus the current command-core information responsibility.

## Evidence anchors

- `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts`
- `apps/arda-hud/src/scene/boardroom/CommandCoreInstrumentScreen.tsx`
- `apps/arda-hud/src/scene/boardroom/commandCoreSignal.ts`
- `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- `apps/arda-hud/src/scene/boardroom/boardroomPhysicalControls.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudSourceAdapters.ts`
- `apps/arda-hud/src/App.tsx`
- `apps/arda-hud/src/lib/ardaSource.ts`
- `core/state/arda_source_map.json`
- source files listed above

## Verification required after later approved changes

- Command signal semantic tests for every encoded state.
- Missing/stale/derived visual-state tests.
- Physical-control route and capability tests.
- Proof that STOP labels cannot imply an unavailable mutation.
- A test distinguishing World View scene navigation from focused workstation navigation.
- Native hit-target and visual acceptance at the authored console geometry.
- FPS/regression verification for the animated instrument.
