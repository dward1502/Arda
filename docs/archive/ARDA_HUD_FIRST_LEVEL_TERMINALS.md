<!-- sigil: REPAIR -->
# ARDA HUD First-Level Embedded Terminals

**Audit timestamp:** 2026-08-03T00:29:54-07:00
**Scope:** the five lower, first-level embedded desk terminals in `apps/arda-hud`, numbered `desk_1` through `desk_5` from left to right.
**Out of scope:** all upper monitors, their claims, and their agent-preview behavior.

## Executive finding (pre-implementation audit)

The boardroom visually presents five lower terminals, but the runtime does **not** model them as five equivalent configurable slots:

- four lower surfaces are assignment-backed `desk_surface` zones;
- the center terminal is a separately hard-coded `CommandCoreSurface` and has no entry in `core/state/arda_boardroom_slots.json`;
- the assignment document describes preview widgets, but the lower surfaces are rendered from a separate hard-coded `boardroomHudInstruments` map rather than from those widget bindings;
- workstation opening depends on a source-zone manifest derived from `core/state/arda_source_map.json`, with a scene-slot template fallback when the manifest cannot be resolved;
- unknown source-map `arda_panels` values are silently filtered out because they are not valid HUD `ModuleId` values.

That last behavior explained the visible empty `desk_1` Governance and Guardhouse workstation at audit time. The workstation resolved, but its source-map panels—`security_posture`, `edge_guardhouse`, and `policy_authority`—were not registered HUD modules, so `buildWorkstationModules()` returned an empty list.

The audited five-terminal arrangement was therefore partly connected, partly fallback-driven, and semantically misaligned on the right side. The implementation below supersedes those audited runtime findings.

## Implementation update — 2026-08-03

The five first-level roles are now explicit in `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts`:

```text
desk_1  Governance and Decisions       configurable: view_desk_l
desk_2  Systems and Fleet              configurable: view_desk_control_panel
desk_3  Command Core and Now           fixed: no assignment slot
desk_4  Routing and Communications     configurable: view_desk_r
desk_5  Human and Business             configurable: view_desk_aux
```

Implemented contracts:

- `FIRST_LEVEL_TERMINALS` formalizes four configurable desk slots and one fixed center core; the center was intentionally not added to `BOARDROOM_CONTROL_SLOT_IDS`.
- `resolveWorkstationProfile()` is the explicit `source section -> workstation profile -> ModuleId[]` adapter boundary. Source-map taxonomy is no longer copied directly into workstation modules.
- Unmapped source panel labels are retained as `rejected_panel_ids` and render a visible **Module Adapter Missing** workstation module rather than producing an unexplained empty window.
- `surface_layout.preview.widgets[*].data_binding` now selects the typed compact-preview adapter. The widget declarations are therefore runtime preview authority rather than ignored metadata.
- Workspace/default assignments now place `fleet_and_backbone` on `desk_2`, `routing_and_comms` on `desk_4`, and `human_business_personal` on `desk_5`.
- Daily Command/Now moved off `desk_5` into the fixed command core. The core screen now projects live Now attention, fleet health, and route summary values from the same instrument models used by the desks.
- STOP remains approval-gated navigation and is named **Gated Stop / Cancel Review**; it does not imply an immediate mutation.
- The Human and Business workstation intentionally includes human, business, and personal document/file connections; `personal_growth` remains optional detail rather than a third primary module.

Verification evidence:

- focused contract suites: **42/42 passed**;
- full HUD production build: `pnpm run build` passed (`tsc && vite build`, 2,576 modules transformed);
- HUD lint: completed with **0 errors** and 111 pre-existing/concurrent warnings;
- full HUD suite: **363/363 passed**; the one failure is the concurrent upper-monitor adapter expectation in `surfaceAdapterManifests.test.ts` (`native_window` expected while the live implementation returns `remote_preview`), outside this first-level terminal scope.

### Native QA follow-up — visible at-rest previews

CUA verification against the running Tauri/WebKit window found that the first-
level click contracts were active while their preview content appeared blank.
The same state rendered correctly in the browser. The cause was the native
WebKitGTK compositor collapsing Drei's perspective-transformed HTML to nearly
zero-sized layers; a second transparent native hit target preserved activation
but could not preserve visible content.

`BoardroomViewport.tsx` now routes all physical preview surfaces through
`resolveBoardroomSurfaceRenderStrategy()`:

- browser: perspective-transformed HTML is retained;
- native Tauri/WebKit: the visible card itself uses non-transformed screen-space
  HTML and remains the accessible activation target;
- the redundant transparent native target is removed;
- fleet cards use bounded monitor/desk heights so their live metrics remain
  inside the physical aperture.

Native CUA acceptance confirmed visible content on the five upper monitors and
all five first-level terminal zones, then opened and closed the Warp workstation
through its visible card. Current verification evidence:

- `boardroomSurfaceRendering.test.ts`: **3/3 passed**;
- full HUD suite: **384/384 passed** across 97 files;
- `pnpm run build`: passed with TypeScript and Vite production output (2,579
  modules transformed);
- `pnpm run lint`: **0 errors**, 103 pre-existing/concurrent warnings.

World View's current sparse, low-contrast presentation is intentional and is
not part of this first-level-terminal corrective scope.

## Audited physical numbering and code identities

| Operator number | Physical zone | Physical label | Assignment slot | Audited assigned source | Audited runtime classification |
|---|---|---|---|---|---|
| `desk_1` | `boardroom.lower.left_wrap` | Governance Console | `view_desk_l` | `governance_guardhouse` | Assignment-backed; workstation resolves but module list is empty |
| `desk_2` | `boardroom.lower.left_inner` | Systems Console | `view_desk_control_panel` | `sovereign_world` | Assignment-backed; opens a scene-slot fallback template |
| `desk_3` | `boardroom.control.center` | Control Core | none | hard-coded command targets | Fixed `CommandCoreSurface`, not assignment-backed |
| `desk_4` | `boardroom.lower.right_inner` | Network Console | `view_desk_r` | `human_realm` | Assignment-backed; opens the Human fallback template despite the Network label |
| `desk_5` | `boardroom.lower.right_wrap` | Human Console | `view_desk_aux` | `now_command` | Assignment-backed; opens the Daily Command fallback template despite the Human label |

Physical definitions: `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts:135-204`.
Assignment authority: `core/state/arda_boardroom_slots.json:247-496`.

## Pre-implementation architecture

### 1. Bundle and source-map layer

`apps/arda-hud/arda_hud.settings.json` points the HUD at the workspace state projections, including:

- `core/state/arda_source_map.json`;
- governance, human, business, personal, queue, operator, and routing projections.

`apps/arda-hud/src/lib/ardaSource.ts` loads the source map and derives one workstation manifest per source-map section. The manifest copies `section.arda_panels` directly into `manifest.module_ids` (`ardaSource.ts:742-750`).

This creates an implicit contract that every `arda_panels` value must also be a valid frontend `ModuleId`. That contract does not currently hold. The frontend module registry only accepts the identifiers declared in `apps/arda-hud/src/components/arda/core/types.ts:7-23`.

### 2. Slot-assignment layer

`apps/arda-hud/src/lib/boardroomSlotSettings.ts` defines:

- four upper-monitor slot IDs;
- four lower-control slot IDs;
- assignment metadata;
- preview/focus/embed layout metadata;
- browser-local, workspace, and fallback persistence modes.

The current lower assignments are:

```text
view_desk_l             -> governance_guardhouse
view_desk_control_panel -> sovereign_world
view_desk_r             -> human_realm
view_desk_aux           -> now_command
```

There is no assignment ID for the center `boardroom.control.center` terminal.

### 3. Preview layer

`App.tsx` derives seven slot-specific HUD instruments in `deriveBoardroomHudInstruments()` and passes them to `BoardroomViewport` (`App.tsx:596-650`). For the four lower configurable surfaces, the preview sources are:

- `view_desk_l`: governance review counts;
- `view_desk_control_panel`: fleet health;
- `view_desk_r`: human documents and notes;
- `view_desk_aux`: command-lane and pending-review counts.

`BoardroomViewport` renders those models through `HudInstrumentSurface` (`BoardroomViewport.tsx:1362-1402`). The `surface_layout.preview.widgets[*].data_binding` records in `core/state/arda_boardroom_slots.json` are not what drives these visible lower-screen values.

The current architecture therefore has two competing preview contracts:

1. declarative `surface_layout` widget metadata in state;
2. hard-coded slot-to-instrument derivation in TypeScript.

The TypeScript map is the effective renderer today.

### 4. Activation and workstation layer

For each assignment-backed lower surface, `BoardroomViewport`:

1. looks for a workstation manifest whose `sourceZoneId` matches the persisted assignment;
2. uses that manifest if found;
3. otherwise converts the physical slot to `scene_slot:<slot_id>`;
4. resolves a fallback manifest from `sceneSlotWorkstationTemplates.ts`;
5. asks `App.tsx` to create a floating workstation;
6. filters the manifest module IDs against the frontend module registry.

Relevant paths:

- assignment lookup: `BoardroomViewport.tsx:385-405`;
- source-zone filtering: `BoardroomViewport.tsx:472-477`;
- fallback templates: `scene/workstations/sceneSlotWorkstationTemplates.ts:18-59`;
- floating workstation creation: `App.tsx:2055-2107`;
- module resolution/filtering: `App.tsx:1683-1716`.

The filter is safe against unknown module IDs, but it produces a completely empty workstation without explaining why.

## Pre-implementation terminal-by-terminal audit

## `desk_1` — Governance Console

### Intended/current identity

- physical label: Governance Console;
- assignment: `governance_guardhouse`;
- assignment title: Review Gates;
- declared modules in slot state: `governance_controls`, `section_focus`;
- preview: governance review-gate instrument.

### What is connected

- the physical zone and click target exist;
- the slot assignment is loaded from workspace state;
- the compact preview is connected to `reviewGateItems` and pending-review counts;
- a `governance_guardhouse_workstation` manifest is derived from the source map;
- the source map contains a Governance and Guardhouse section.

### What is not connected

The resolved workstation uses the source-map panel list, not the assignment record's declared module list. The active source map declares:

```text
security_posture
edge_guardhouse
policy_authority
```

None is a valid HUD `ModuleId`, so all three are removed by `buildWorkstationModules()`. The result is the empty workstation currently visible to the operator.

The current source map also declares four primary sources for this section. At audit time:

- missing: `core/state/warden_guardhouse.json`;
- missing: `core/state/warden_policy_authority.json`;
- present: `core/state/warden_edge_contract.json`;
- present: `core/state/warden_nightly_doctrine.json`.

### Assessment

**Partially connected, functionally empty when opened.** The preview works, the source-zone manifest resolves, and the failure is at the source-panel-to-HUD-module boundary. The empty state is not because the physical terminal lacks an assignment.

## `desk_2` — Systems Console

### Intended/current identity

- physical label: Systems Console;
- assignment: `sovereign_world`;
- assignment title: Command Podium;
- declared modules: `executive_overview`, `systems`;
- preview: fleet-health instrument.

### What is connected

- all three primary `sovereign_world` source-map files are present:
  - `core/state/world.json`;
  - `core/state/system_manifest.json`;
  - `core/state/system_control.json`;
- the visible compact preview receives fleet-health data;
- the fallback scene-slot template supplies `operating_surface`, `executive_overview`, and `systems`, all valid frontend modules.

### What is not directly connected

`BoardroomViewport` deliberately removes `sovereign_world` from the list used to resolve physical slot assignments. As a result, this desk does not open the actual `sovereign_world_workstation`; it opens `scene_slot:view_desk_control_panel` and receives the generic Desk Control Command template.

The displayed fleet preview and the opened workstation are therefore related by convention, not by one authoritative source-zone manifest.

### Assessment

**Usable but fallback-driven and overloaded.** This is one of the surfaces where several broad modules have been placed together. It behaves more like a generic command workstation than a bounded Systems terminal.

## `desk_3` — Command Core

### Current identity

The center is not a boardroom slot. It is a fixed `CommandCoreSurface` with one screen and four hard-coded controls:

- main screen / CONTROL -> `sovereign_world`;
- GO -> `planning_and_queue`;
- STOP -> `governance_guardhouse`;
- ROUTE -> `routing_and_comms`;
- WORLD -> scene transition.

Definitions: `boardroomPhysicalControls.ts:76-119`; rendering: `BoardroomViewport.tsx:903-955`.

### What is connected

- the main screen opens the actual Sovereign World workstation;
- GO opens Planning and Queue;
- ROUTE opens Routing and Communications;
- WORLD transitions to world mode;
- all actions pass through authority/status metadata and show local feedback.

### What is not connected

- the center screen's `mode / health / routes` display is decorative, not populated from live metrics;
- STOP does not execute or stage an emergency stop. It opens the Governance workstation and is marked `approval_required`;
- the center has no slot assignment, `surface_layout`, visualization profile, or persistence record;
- the code and state schema do not clearly say whether this fixed/non-configurable behavior is intentional.

### Assessment

**Action-connected but not a live terminal surface.** It is best understood as a fixed command launcher, not the fifth member of the configurable slot system.

## `desk_4` — physically Network, currently Human

### Intended/current identity

- physical label: Network Console;
- assignment: `human_realm`;
- assignment title: Human + Business;
- declared modules: `human_realm`, `business`;
- fallback template adds `personal_growth`;
- preview: human documents/notes instrument.

### What is connected

- `human_context.json`, `business_runtime.json`, and `personal_runtime.json` are all present and loaded independently by the HUD bundle;
- the human preview receives document and note counts;
- the scene-slot fallback template resolves to valid Human, Business, and Personal Growth modules.

### What is not aligned

- the physical zone says Network while every rendered and opened surface is Human-oriented;
- the active source map has no `human_realm` section. It instead uses `human_business_personal`, `business_ops`, and `personal_growth` sections;
- because no `human_realm` workstation manifest exists, the terminal depends on the physical-slot fallback template rather than a source-map workstation.

### Assessment

**Functionally connected through fallback, semantically assigned to the wrong physical terminal.** This is the clearest right-side label/assignment mismatch.

## `desk_5` — physically Human, currently Daily Command

### Intended/current identity

- physical label: Human Console;
- assignment: `now_command`;
- assignment title: Daily Command;
- declared modules: `operating_surface`, `executive_overview`;
- preview: operating-surface lane count and pending-review count.

### What is connected

- the compact preview is derived from bundle operations flow and review-gate state;
- the fallback template opens valid Operating Surface and Executive Overview modules;
- when `core/state/operations_flow.json` is absent, `ardaSource.ts` derives an operations-flow fallback from queue summary state.

### What is not aligned

- the physical Human label does not match Daily Command;
- the active source map has no `now_command` section or workstation manifest;
- `core/state/operations_flow.json` is absent at audit time, so the display is using derived rather than direct operations-flow authority;
- the terminal relies on the `view_desk_aux` fallback template.

### Assessment

**Functionally connected through derived/fallback paths, semantically assigned to the wrong physical terminal.** The current arrangement displaces the Human surface to `desk_4` and puts Now/Command content on `desk_5`.

## Pre-implementation connection matrix

| Terminal | Physical surface | Preview data | Source-zone manifest | Opened modules | Semantic alignment | Overall |
|---|---|---|---|---|---|---|
| `desk_1` Governance | connected | connected | connected | **empty: invalid module IDs** | aligned | partial/broken |
| `desk_2` Systems | connected | connected | bypassed by filter | fallback: 3 valid modules | broadly aligned | usable/fallback-heavy |
| `desk_3` Command Core | connected | decorative screen; connected buttons | direct action targets | target-dependent | aligned | action-connected |
| `desk_4` Network | connected | connected to Human | no `human_realm` manifest | fallback: 3 valid Human modules | **misaligned** | usable/wrong role |
| `desk_5` Human | connected | connected to derived Daily Command | no `now_command` manifest | fallback: 2 valid command modules | **misaligned** | usable/wrong role |

## Recommended first-level role split

The physical labels already suggest a coherent five-terminal topology. Keep that topology and reduce each terminal to one first-level responsibility:

### `desk_1` — Governance and Decisions

Always-visible content:

- pending review/approval count;
- blocked/attention gates;
- autonomy/governance readiness;
- latest decision-receipt status;
- source freshness and missing-authority warning.

Focused workstation:

- `governance_controls`;
- a filtered approval/review view from `operating_surface` or `section_focus`;
- no generic system or planning dashboard.

### `desk_2` — Systems and Fleet

Always-visible content:

- live/total/offline nodes;
- routing-provider availability summary;
- highest-severity health fault;
- runtime drift/freshness.

Focused workstation:

- the existing focused Fleet view;
- `systems` as the primary module;
- `operations_and_packages` only as secondary detail when needed.

Do not also make this the full command, governance, queue, and settings surface.

### `desk_3` — Command Core and Now

Always-visible content:

- current mode;
- active objective/current work;
- attention count;
- route summary;
- health summary.

Controls:

- GO: open approval/work queue;
- STOP: open a clearly named gated stop/cancel review flow, not imply immediate mutation;
- ROUTE: open routing workstation;
- WORLD: transition to world mode.

This should remain fixed rather than freely assignable. If that is accepted, the schema and docs should explicitly define **four configurable desks plus one fixed command core**, rather than implying five equivalent slots.

### `desk_4` — Routing and Communications

Always-visible content:

- selected provider/model per lane;
- provider pressure/headroom;
- Hermes/Manwe command-channel state;
- interruption or communication-adapter alerts;
- explicit degraded state when `manwe_router.json` is absent.

Focused workstation:

- existing `systems` and `operations_and_packages` components filtered to routing;
- or one bounded routing-focused module if the existing components cannot provide a clean view.

This restores the physical Network Console meaning.

### `desk_5` — Human and Business

Always-visible content:

- human-context freshness;
- personal/business attention items;
- document/note summary;
- accessibility-aware reminders or operator context;
- no clinical interpretation or measurement claims.

Focused workstation:

- `human_realm`;
- `business`;
- `personal_growth` as an optional secondary tab, not an always-visible dense surface.

This restores the physical Human Console meaning. The current `now_command` role should move into the fixed center Command Core instead of occupying a separate outer terminal.

## Accepted contract decisions

### Decision 1: formalize four configurable desks plus one fixed core — accepted and implemented

Recommended. Keep `desk_3` fixed and action-oriented. Do not add it to `BOARDROOM_CONTROL_SLOT_IDS` merely to make the counts symmetrical. Instead, make the architecture explicit:

```text
first-level surfaces = 4 configurable desk previews + 1 fixed command core
```

### Decision 2: separate source-map panel taxonomy from frontend module IDs — accepted and implemented

Recommended. `arda_panels` currently contains architectural/domain labels as well as actual HUD module IDs. Do not continue copying these strings directly into workstation modules.

Introduce or document an explicit adapter boundary:

```text
source section -> workstation profile -> valid HUD ModuleId[]
```

Unknown labels should produce a visible "module adapter missing" state with the rejected IDs, not an empty workstation.

### Decision 3: choose one preview authority — accepted and implemented with typed widget adapters

Recommended: make a typed preview-adapter registry authoritative and project its resolved widgets into the boardroom. Either:

- actually render `surface_layout.preview.widgets` through registered data-binding adapters; or
- narrow `surface_layout` so it honestly describes the existing hard-coded instrument adapters.

Do not keep declarative widget bindings that are ignored by the visible runtime.

### Decision 4: align right-side assignments to physical meaning — accepted and implemented

Recommended target:

```text
view_desk_r   -> routing_and_comms
view_desk_aux -> human realm profile
```

The Human profile should resolve the active source-map IDs (`human_business_personal`, `business_ops`, and `personal_growth`) through a deliberate workstation profile rather than depending on the `scene_slot:` fallback.

### Decision 5: bound module density — accepted and implemented for first-level profiles

Recommended first-level rule:

- one primary responsibility per terminal;
- one compact preview;
- at most two primary focused modules;
- optional detail behind a secondary tab;
- no terminal should become a miscellaneous container because a source section lists many panel labels.

## Implemented order

1. Mapped Governance source panels to valid HUD modules and added explicit adapter-missing state for unknown IDs.
2. Aligned `desk_4` with Routing and Communications and `desk_5` with Human and Business.
3. Folded Daily Command/Now status into the center Command Core.
4. Made the center's visible mode/health/routes values live while preserving its fixed action role.
5. Enforced widget data bindings as the preview-adapter authority.
6. Added focused tests for five physical roles, four configurable slot IDs, the fixed center contract, source taxonomy adaptation, and non-empty focused workstation modules.

## Source evidence index

- physical lower zones: `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts:135-204`
- four configurable lower slot IDs: `apps/arda-hud/src/lib/boardroomSlotSettings.ts:15-17`
- default/current lower assignments: `apps/arda-hud/src/lib/boardroomSlotSettings.ts:117-126`
- assignment metadata and declared modules: `apps/arda-hud/src/lib/boardroomSlotSettings.ts:214-241`
- workspace assignment authority: `core/state/arda_boardroom_slots.json:247-496`
- source-map-derived workstation manifests: `apps/arda-hud/src/lib/ardaSource.ts:742-750`
- active source map: `core/state/arda_source_map.json`
- valid HUD module IDs: `apps/arda-hud/src/components/arda/core/types.ts:7-23`
- scene-slot fallback profiles: `apps/arda-hud/src/scene/workstations/sceneSlotWorkstationTemplates.ts:18-59`
- assignment lookup and fallback zone resolution: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx:385-405`
- source-zone exclusion for scene slots: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx:472-477`
- hard-coded lower preview rendering: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx:1362-1402`
- center command rendering: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx:903-955`
- center control targets: `apps/arda-hud/src/scene/boardroom/boardroomPhysicalControls.ts:76-119`
- slot-specific instrument map: `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.ts:401-410`
- module filtering: `apps/arda-hud/src/App.tsx:1683-1716`
- floating workstation creation: `apps/arda-hud/src/App.tsx:2055-2107`
