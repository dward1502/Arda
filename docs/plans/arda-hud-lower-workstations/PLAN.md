---
soterion:
  sigil: "FORGE"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 🔨 Lower workstation convergence plan | owner: HERMES | status: active | reviewed: 2026-08-16

# ARDA HUD Lower Workstations — Convergence Implementation Plan

## Objective

Converge the five audited lower surfaces into distinct, source-grounded instruments and focused workstations without introducing a new dashboard system or inventing backend capabilities.

The implementation must:

- relocate Settings, Terminal, and Hermes Dashboard controls onto the Command Core front plate;
- remove their detached bottom-row presentation after behavioral and accessibility parity is proven;
- preserve the existing Command Core command bank;
- assign each information class to one focused workstation owner;
- wire existing ARDA state and projection families before adding new data sources;
- render missing or unavailable connections honestly;
- keep lower desk screens concise, low-text tactical instruments;
- give dense workflows task-appropriate focused layouts;
- retain real governance and intervention controls;
- preserve native runtime performance.

This plan follows:

- [inventory and audit index](README.md);
- [cross-reference and ownership split](CROSS_REFERENCE.md);
- [visual and interaction research](DESIGN_REFERENCES.md);
- the five monitor-specific audit records in this directory.

## Non-negotiable boundaries

1. `core/projects/tasks/queue.jsonl` remains task authority.
2. `core/state/queue_active.json` and `core/state/queue_summary.json` remain untouched legacy projections.
3. Command Core remains fixed at `boardroom.control.center`.
4. World View remains display-only.
5. Existing source files are surfaced through adapters; no fabricated “live” values.
6. An absent service or unwired file is rendered as unavailable, missing, or projected—not as zero and not as healthy.
7. Lower apertures remain Three.js/canvas instruments rather than DOM cards.
8. Focused workstations may use DOM where reading, selection, forms, or evidence review require it.
9. No new visualization dependency is planned.
10. No orphan deletion occurs until dynamic-reference search, focused tests, build, and native smoke verification pass.
11. No commit or push is part of an agent slice unless explicitly requested.

## System design

### Presentation levels

Every lower domain has three levels:

1. **Lower instrument** — one primary signal, one or two supporting states, source/freshness cue, click target.
2. **Focused workstation** — task-shaped navigation, selection, detail, action, and receipts.
3. **Evidence detail** — paths, timestamps, hashes, raw records, policy authority, and terminal receipts.

A datum should not be rendered in full at more than one focused workstation. Other domains may show a linked summary.

### Truth contract

Reuse `ArdaSourceProvenance`, existing freshness models, and existing adapter types. Do not create a parallel provenance framework.

Every view-model field must carry or inherit:

- source path or runtime endpoint;
- observed/generated timestamp when present;
- classification: live, snapshot, projected, stale, unavailable, missing, loaded-but-unused, or hard-coded;
- owning domain;
- action authority where interactive.

### Component restraint

Before creating a component, agents must search for and evaluate:

- `BoardroomInstrumentScreen` and current lower-instrument renderers;
- `PhysicalControlButtonSurface`;
- `LineList`, source badges, refresh affordances, and existing module primitives;
- current floating workstation shell and list/detail patterns;
- current action adapters and receipt displays.

Create a new component only when the interaction model cannot be expressed clearly by an existing primitive. Prefer extracting an existing inline implementation over adding a sibling duplicate.

## Workstream ownership

The phases are ordered by dependency. Parallel implementation is permitted only where a phase explicitly identifies independent slices.

| Workstream | Primary area | Must not modify concurrently |
|---|---|---|
| composition authority | manifests, slot profiles, settings layout | domain agents consuming those contracts |
| Command Core | boardroom physical geometry/actions | other edits in `BoardroomViewport.tsx` |
| Governance | approval/Guardhouse adapters and workstation | shared action contracts without coordination |
| Fleet | node/backbone adapters and workstation | Routing ownership code |
| Routing | lane/provider adapters and native refresh action | Fleet ownership code |
| Human/Business/Personal | continuity adapters and workstation | shared source-bundle types without coordination |
| visual substrate | instrument truth/motion primitives | simultaneous boardroom renderer refactors |

## Phase 0 — Freeze baseline and contracts

**Status:** complete with recorded native limitations; see [`BASELINE.md`](BASELINE.md).

### Goal

Capture current behavior and establish regression gates before restructuring ownership.

### Tasks

1. Re-read `AGENTS.md` and all audit records.
2. Run `git status --short` and preserve user-owned modifications.
3. Record current source-manifest output for the four configurable lower slots.
4. Record current physical control IDs, labels, target zones, authority, and verification paths.
5. Record current accessibility-control names for Settings, Hermes CLI, and Hermes Dashboard.
6. Run the existing focused tests:

```bash
pnpm --dir apps/arda-hud test -- \
  src/scene/boardroom/boardroomPhysicalControls.test.ts \
  src/scene/boardroom/boardroomSpatialLayout.test.ts \
  src/scene/boardroom/BoardroomAccessibilityControls.test.tsx \
  src/scene/boardroom/boardroomHudSourceAdapters.test.ts \
  src/scene/boardroom/boardroomHudInstruments.test.ts \
  src/scene/boardroom/commandCoreSignal.test.ts
```

7. Run `pnpm --dir apps/arda-hud build`.
8. Start or attach to `pnpm run tauri dev` and capture the current native composition and frame-rate evidence using the existing acceptance path.

### Deliverables

- baseline test output;
- native screenshot/acceptance artifact paths;
- source/interaction contract snapshot in the implementation session notes;
- no application changes.

### Exit criteria

- current behavior is reproducible;
- any pre-existing failure is isolated and explicitly recorded;
- the agent can identify which files are user-owned and must not be touched.

## Phase 1 — Converge composition authority

**Status:** implemented and verified; see [`COMPOSITION_AUTHORITY.md`](COMPOSITION_AUTHORITY.md).

### Goal

Make one runtime contract authoritative for which focused workstation each lower slot opens, while retaining explicit adapters for settings and fallback profiles.

### Primary files

- `apps/arda-hud/src/lib/ardaSource.ts`
- `apps/arda-hud/src/lib/boardroomSlotSettings.ts`
- `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts`
- `apps/arda-hud/src/lib/settingsLayout.ts`
- `apps/arda-hud/src/scene/systems/sceneSlotWorkstationTemplates.ts`
- `apps/arda-hud/src/scene/systems/workstationRoles.ts`
- `apps/arda-hud/src/App.tsx`

### Tasks

1. Trace every consumer of the six competing composition sources listed in `CROSS_REFERENCE.md`.
2. Choose the source-derived workstation manifest plus persisted slot assignment as runtime authority unless live evidence demonstrates a stronger existing owner.
3. Convert terminal profiles, settings layouts, templates, and role contracts into named adapters or metadata readers; do not allow them to independently select runtime modules.
4. Encode the approved ownership split:
   - Governance/Guardhouse;
   - Fleet/Backbone;
   - Routing/Communications;
   - Human/Business/Personal.
5. Remove the current Fleet/Routing duplication in manifest output.
6. Ensure `sovereign_world` no longer selects Fleet detail merely because an inline view recognizes its key.
7. Preserve persisted user slot assignments and import/export behavior.
8. Add table-driven tests covering all default slot assignments and every adapter projection.

### Acceptance

- each physical slot resolves to exactly one focused workstation contract;
- settings display the same assignments the boardroom actually uses;
- no adapter silently changes module selection;
- existing persistence tests pass;
- full HUD build passes.

## Phase 2 — Relocate Command Core utility controls

**Status:** implemented and verified on 2026-08-16; see [`COMMAND_CORE_CONTROLS.md`](COMMAND_CORE_CONTROLS.md).

### Goal

Move existing Settings, Terminal, and Hermes Dashboard launch controls from the detached lower row to a dedicated utility bank on the Command Core front plate. Preserve the existing command bank.

### Current behavior to reuse

- `App.tsx` already owns `handleOpenHermesDashboard` and `handleOpenHermesCli`.
- `BoardroomViewport.tsx` already owns Settings, Terminal, and Dashboard activation wrappers.
- `boardroomPhysicalControls.ts` already defines authority and verification paths.
- `BoardroomAccessibilityControls.tsx` already exposes all three functions to non-visual input.
- `SettingsModule.tsx` and `HermesDashboardModule.tsx` already provide the focused surfaces.

No duplicate launcher or settings implementation should be created.

### Primary files

- `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomPhysicalControls.ts`
- `apps/arda-hud/src/scene/boardroom/BoardroomAccessibilityControls.tsx`
- associated tests in the same directories

### Geometry and interaction

1. Keep the top command bank:
   - approval/GO;
   - emergency/STOP;
   - route selector/ROUTE;
   - world transition/ENTER.
2. Add a physically separate **utility bank** to the forward-facing plate of `boardroom.control.center`:
   - SETTINGS;
   - TERMINAL;
   - HERMES.
3. Reuse the existing action objects and callbacks; do not create new action IDs unless existing IDs cannot represent the relocated targets.
4. Remove the scene-level detached Settings, Hermes CLI, and Hermes Dashboard `InteractionPad` instances after the plate bank is functional.
5. Remove the detached row's Service Health presentation. Keep service health available through Fleet/Backbone and feed its state into the Command Core health instrument where already supported.
6. Retire obsolete spatial zones only after layout persistence normalization has a migration/filter path for old overrides.
7. Preserve accessible controls as semantic parity, not as a visually duplicated row.
8. Preserve disabled, blocked, hover, press, and feedback behavior.

### Tests

- `boardroomPhysicalControls.test.ts`
- `boardroomSpatialLayout.test.ts`
- `BoardroomAccessibilityControls.test.tsx`
- `boardroomComposition.test.ts`
- `boardroomVisualRegression.test.ts`
- targeted tests proving each plate button calls the existing handler exactly once
- test old position overrides containing detached-row zone IDs are ignored or migrated safely

### Native acceptance

- click SETTINGS and verify the real settings workstation opens;
- click TERMINAL and verify the native Hermes terminal window command is invoked;
- click HERMES and verify the Hermes dashboard window/surface opens;
- verify GO/STOP/ROUTE/ENTER remain targetable;
- verify the detached row is no longer rendered;
- verify keyboard/accessibility controls still expose the same functions;
- verify no double dispatch;
- verify frame rate does not regress beyond the existing acceptance threshold.

### Exit criteria

The change is a relocation and consolidation of existing controls, not a second launcher system.

## Phase 3 — Establish the shared instrument truth substrate

**Status:** implemented and verified on 2026-08-16; see [`SOURCE_TRUTH.md`](SOURCE_TRUTH.md).

### Goal

Allow every lower instrument to render honest source state before redesigning its domain visualization.

### Primary files

- `apps/arda-hud/src/scene/boardroom/boardroomHudSourceAdapters.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.ts`
- `apps/arda-hud/src/scene/boardroom/BoardroomInstrumentScreen.tsx`
- existing provenance/freshness files under `apps/arda-hud/src/lib/`

### Tasks

1. Extend existing view models rather than create a second status model.
2. Map live, snapshot, projected, stale, unavailable, and missing into the instrument renderer.
3. Add non-color cues for every state.
4. Ensure a missing connection leaves the instrument frame and source name visible.
5. Ensure stale state cannot look identical to live state.
6. Ensure loaded-but-unused data is only exposed in diagnostics/evidence, never presented as runtime authority.
7. Keep motion bounded and data-driven.
8. Preserve reduced-motion and software-renderer profiles.

### Tests

- table-driven adapter tests for every truth state;
- deterministic render-model tests;
- reduced-motion tests;
- boardroom performance and visual-regression tests.

### Exit criteria

Domain agents can connect existing sources without inventing ad hoc badges or fallback text.

## Phase 4 — Governance/Guardhouse decision chamber

### Operator question

“What requires a human decision or policy intervention now, and what evidence authorizes that decision?”

### Existing sources to wire

Use the audited governance, autonomy, augmentation, Arandur, Review Gate, Warden, and task-authority source families. Include existing Warden projections even when no direct connection exists; classify each correctly.

### Focused layout

- top posture rail: policy, autonomy, Guardhouse state, source freshness;
- left selectable record list: approvals, incidents, review gates, recommendations;
- right selected-record detail: summary, rationale, affected scope, evidence, authority, timestamps, receipts;
- bottom contextual action strip: only actions valid for the selected record.

### Tasks

1. Extract a Governance focused view-model from existing source-bundle derivations.
2. Preserve append-only and recommendation-scoped approval rules.
3. Reuse existing approval controls and adapters.
4. Integrate Arandur records into left-list/right-detail interaction.
5. Expose Warden edge contract, nightly doctrine, Guardhouse, and policy authority as connected, snapshot, projected, or unavailable.
6. Remove generic stacked module-card duplication once parity is tested.
7. Build a concise lower Guardhouse instrument from incident/decision pressure, not arbitrary document counts.
8. Add source and receipt detail behind progressive disclosure.

### Acceptance

- a user can select one record without scanning all record bodies;
- approve/reject/defer controls retain existing authority and append behavior;
- every decision shows evidence and authority before dispatch;
- empty state distinguishes “no pending records” from “source unavailable.”

## Phase 5 — Fleet/Backbone topology workstation

### Operator question

“Which physical/runtime node or backbone link needs attention, and what is installed or drifting there?”

### Existing sources to wire

- fleet runtime;
- nodes;
- models;
- health;
- hardware;
- backbone;
- Manwë snapshots where they represent node/runtime state;
- existing fleet view-model and preview surface.

### Focused layout

- topology/rack-line field as the primary visualization;
- selectable node/link index;
- selected detail for hardware, models, reachability, health, drift, source age;
- contextual maintenance actions only where an adapter already exists.

### Tasks

1. Consolidate the active inline Fleet focused view, `FleetPreviewSurface`, generic Systems Fleet content, and audited orphan implementations into one owner.
2. Prefer extracting the active implementation over creating a fourth Fleet component.
3. Remove provider routing and lane ownership from Fleet detail.
4. Connect all six existing AULË-produced Fleet projections through one adapter family.
5. Render a producer-present/consumer-unwired condition explicitly during intermediate slices.
6. Give the lower Fleet instrument a node/backbone signal rather than provider summaries.
7. Preserve existing Fleet actions and source freshness.

### Acceptance

- one selected node has one coherent detail view;
- provider routing detail is absent except for a linked summary;
- all six Fleet projection files are accounted for;
- zero nodes is distinct from unavailable node data;
- no fourth Fleet renderer is introduced.

## Phase 6 — Routing/Communications flow workstation

### Operator question

“Where is work routed, why was that provider/model selected, and is the route healthy and within policy?”

### Existing sources to wire

- lane ownership;
- provider/model catalog;
- headroom;
- fitness;
- capability and policy data;
- routing receipts/history where present;
- existing routing action adapters.

### Focused layout

- directed lane/provider/model flow field;
- lane/route selector;
- selected route detail for policy, capacity, fitness, health, and change history;
- contextual refresh/reroute action area.

### Tasks

1. Fix the confirmed native command mismatch:
   - frontend currently invokes `run_manwe_provider_intelligence_refresh`;
   - backend registers `run_charon_provider_intelligence_refresh`.
2. Align the action ID and native command with the actual backend authority.
3. Replace the test's frontend-only mock with a registration-aware contract test.
4. Remove Fleet hardware, package inventory, setup, and audit duplication from Routing.
5. Build the lower Routing instrument from real lane/provider transitions.
6. Mark unconnected communication sources unavailable rather than generating activity.
7. Keep route changes gated and receipt-backed.

### Acceptance

- native refresh reaches the registered backend command;
- a selected lane shows its route and governing evidence;
- routing and fleet focused workstations no longer share the same modules;
- a dispatched change is not shown as complete until a receipt proves it.

## Phase 7 — Human/Business/Personal continuity workstation

### Operator question

“What personal, human, or business commitment needs attention next, and what is its current evidence-backed state?”

### Existing sources to wire

- `human_context.json`;
- `business_runtime.json`;
- `personal_runtime.json`;
- existing Human and Business module derivations;
- referenced project/client paths where they exist.

### Focused layout

- shared current-focus rail;
- Human horizon for relationships, context, notes, and commitments;
- Business horizon for opportunities, engagements, projects, and realized value;
- Personal horizon for routines, wellbeing, home, and life continuity;
- list/detail only for dense collections;
- timeline/constellation instrument for change and continuity, not generic cards.

### Tasks

1. Add the currently omitted Personal runtime to the assigned focused workstation.
2. Make Business and Personal contribute to the lower instrument.
3. Validate every referenced client/project path.
4. Render absent referenced files as missing.
5. Separate planned/opportunity value from realized value.
6. Reuse existing Human and Business derivations and only extract shared selection primitives when necessary.
7. Keep sensitive detail in the focused workstation, not on the glanceable lower monitor.

### Acceptance

- Human, Business, and Personal are all present and distinguishable;
- missing referenced artifacts cannot appear as completed work;
- lower surface is concise and privacy-respecting;
- focused collections support selection without rendering all detail at once.

## Phase 8 — Visual convergence without homogenization

### Goal

Apply the shared physical and truth grammar while preserving distinct domain identities.

### Tasks

1. Define shared tokens for console material, vector line weight, focus state, truth state, minimum text size, and motion budget.
2. Apply domain-specific compositions:
   - Governance: queue/list and decision detail;
   - Fleet: topology/rack line;
   - Routing: directed flow field;
   - Human/Business/Personal: continuity horizons;
   - Command Core: coherence field plus tactile control banks.
3. Remove unnecessary nested cards and duplicate headings.
4. Keep source markers in a consistent corner without turning them into a persistent text rail.
5. Verify color contrast and non-color state cues.
6. Verify all surfaces with reduced motion.
7. Compare native screenshots at the actual boardroom camera distance.

### Acceptance

The surfaces look like one ARDA machine, but no two focused workstations are merely recolored copies.

## Phase 9 — Cleanup and authority retirement

### Goal

Delete only artifacts proven obsolete after all runtime owners are established.

### Candidate files

- `apps/arda-hud/src/lib/providerRouting.ts`
- `apps/arda-hud/src/scene/workstations/fleetWorkstationView.tsx`
- `apps/arda-hud/src/components/arda/modules/fleet/FleetWorkstation.tsx`

### Tasks

1. Repeat repository-wide static and dynamic reference searches.
2. Confirm no generated manifest, test harness, script, or lazy import refers to a candidate.
3. Delete one candidate at a time.
4. Run focused tests and build after each deletion.
5. Consolidate or document the surviving composition adapters.
6. Handle Tauri backup artifacts as a separate repository-hygiene slice; do not mix them into HUD convergence.
7. Update audit records from candidate to retired/retained with evidence.

### Acceptance

- no duplicate Fleet owner remains;
- no empty routing placeholder remains;
- surviving contract files have explicit ownership comments or docs;
- build and native runtime remain healthy.

## Phase 10 — Whole-product verification and documentation closeout

### Automated gates

```bash
pnpm --dir apps/arda-hud test
pnpm --dir apps/arda-hud lint
pnpm --dir apps/arda-hud build
pnpm --dir apps/arda-hud run verify:boardroom-assets
pnpm --dir apps/arda-hud run soterion:unicode
pnpm run tauri build --no-bundle
```

Use repository-root `pnpm run tauri dev` for native acceptance, per `AGENTS.md`.

### Native acceptance matrix

For each lower surface:

- click lower instrument and verify the correct focused owner opens;
- verify the primary reading maps to a real source field;
- verify live, stale, missing, and unavailable fixtures/states;
- verify source and timestamp detail;
- verify keyboard/accessibility entry;
- verify reduced motion;
- verify no unexpected polling while hidden;
- record native screenshot and frame-rate evidence.

For Command Core:

- verify SETTINGS, TERMINAL, and HERMES on the front plate;
- verify detached utility row absent;
- verify GO, STOP, ROUTE, ENTER preserved;
- verify blocked and receipt states;
- verify no duplicate launch.

For Governance:

- verify list/detail selection;
- verify append-only approval behavior;
- verify evidence visible before action.

For Fleet and Routing:

- verify content ownership separation;
- verify native provider refresh command;
- verify node and route missing states.

For Human/Business/Personal:

- verify all three horizons;
- verify missing referenced files;
- verify privacy-appropriate lower summary.

### Documentation closeout

1. Update all audit records with implementation status and real evidence.
2. Update `CROSS_REFERENCE.md` ownership outcomes.
3. Move this plan from active to completed only when every required acceptance gate is evidenced.
4. Archive the directory only after it ceases to be active planning authority.
5. Do not claim complete, live, wired, or proven without the corresponding runtime evidence.

## Agent handoff template

Every delegated slice must include:

```text
Scope:
Owned files:
Files explicitly excluded:
Authoritative sources:
Current truth classification:
Existing components/actions to reuse:
Required behavior:
Focused tests:
Native acceptance steps:
Documentation evidence to update:
Known user-owned worktree changes:
Stop conditions:
```

### Stop conditions

An agent must stop and report rather than guess when:

- a source has two plausible authorities;
- an action name differs between frontend and backend;
- a proposed edit touches user-owned queue state;
- a missing file might be generated by an untraced producer;
- a new component would duplicate an existing implementation;
- native behavior cannot be verified;
- frame rate regresses;
- an approval action would weaken append-only or policy constraints.

## Recommended execution order

1. Phase 0 baseline.
2. Phase 1 composition authority.
3. Phase 2 Command Core utility relocation.
4. Phase 3 truth substrate.
5. Phases 4 and 7 may proceed in parallel after Phase 3 if they do not edit shared bundle types concurrently.
6. Phases 5 and 6 should remain coordinated and usually serialize because their current ownership overlaps.
7. Phase 8 visual convergence after functional layouts are proven.
8. Phase 9 cleanup only after all runtime owners are established.
9. Phase 10 whole-product verification and documentation closeout.

## Definition of done

The lower-workstation program is done only when:

- each lower instrument has one focused owner;
- each focused owner has a distinct task-appropriate interaction model;
- existing ARDA sources are either connected or truthfully marked unavailable/missing;
- Command Core owns Settings, Terminal, and Hermes Dashboard access on its front plate;
- the detached utility row is gone;
- Fleet and Routing are no longer duplicate workstations;
- Personal is no longer omitted;
- Governance supports dense list/detail decisions with real authority;
- known native action mismatches are fixed;
- obsolete artifacts are retired with evidence;
- focused tests, full tests, lint, build, Tauri build, native interactions, and performance gates pass;
- documentation reflects what is actually wired and proven.
