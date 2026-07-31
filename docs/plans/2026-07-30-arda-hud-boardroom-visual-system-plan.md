# ARDA HUD Boardroom Visual System and Ecosystem Integration Plan

> **For Hermes:** Implement this plan in visually reviewable slices. Preserve the existing boardroom, compare every slice in the live renderer, and do not claim native behavior from browser-only evidence.

**Goal:** Turn the current flat boardroom prototype into a futuristic, wraparound command console whose physical controls work, whose monitor inserts show meaningful live ARDA state, and whose screens can be assigned and customized without rebuilding the scene.

**Product role:** The 3D boardroom is a navigable status and control shell for inspectable ARDA workflows. It must reveal provenance, freshness, state, failures, and approval boundaries—not merely decorate backend data.

**Current visual contract:** Four upper monitors, four lower desk inserts, one center insert, a presence emitter/avatar, and dedicated physical controls. Clicking a monitor or insert opens its assigned detailed workstation.

---

## Design principles

1. **Visual truth matches click truth.** Every visible surface has one clear identity and one destination.
2. **Preview first, detail on activation.** The 3D surface shows a compact, legible projection; click opens the complete workstation using the same data contract.
3. **Data and visualization are separate slots.** A user can assign both the ARDA source and a compatible visualization preset.
4. **No decorative false telemetry.** Motion may indicate activity, flow, freshness, or state only when backed by a real source; unavailable data is visibly unavailable.
5. **Physical controls are real controls.** Buttons, toggles, knobs, and selectors expose bounded actions with clear hover, active, disabled, and error states.
6. **Progressive visual fidelity.** Build procedural geometry and reusable contracts first; introduce authored GLB assets after dimensions and interactions stabilize.
7. **Accessible motion.** Animated displays honor reduced-motion preferences and remain understandable when static.

---

## Architecture

### A. Scene shell

- Curved/wraparound desk housing built from modular center, inner, and wing segments.
- Angled upper monitors facing the operator/camera.
- Recessed screen inserts with frames, glass, bezels, illumination, and control rails.
- Reusable physical controls mounted independently from workstation hit areas.

### B. Surface assignment contract

Each assignable screen stores:

- stable scene slot ID;
- source/workstation zone ID;
- visualization preset ID;
- compact configuration and thresholds;
- presentation mode;
- freshness and provenance policy.

Persisted assignments remain authoritative. TypeScript defaults are recovery defaults only.

### C. Preview model boundary

ARDA ecosystem readers project raw state into bounded visual models such as:

- node graph and route topology;
- event/data stream;
- waveform or pulse history;
- segmented bars and capacity meters;
- queue lanes and progress tracks;
- approval/gate matrix;
- freshness radar;
- agent presence constellation;
- cost/resource telemetry;
- knowledge/research evidence flow.

The 3D renderer consumes these visual models rather than reading arbitrary files directly.

### D. Detail navigation

Activating a screen opens the corresponding workstation with:

- the same source identity and snapshot timestamp;
- expanded records and provenance;
- available safe actions;
- explicit stale, unavailable, or degraded state;
- approval controls where mutation is possible.

### E. Customization

Boardroom Settings supports:

- screen-to-source assignment;
- visualization preset selection;
- color/accent theme;
- density, timespan, and threshold options;
- layout preview and reset;
- saved profiles without changing canonical source data.

---

## Phase 0 — Repair interaction truth

### Tasks

- Use the dedicated Settings button zone instead of mapping the entire center console to Settings.
- Place Settings, Hermes Dashboard, and Hermes CLI as distinct side-by-side physical controls.
- Add and register `open_hermes_terminal_window`.
- Create/focus a native window loading the terminal entry.
- Include the terminal entry in production Vite output.
- Remove the fallback that disguises terminal failure by opening Hermes Dashboard.
- Reassign the duplicate pink Hermes desk to a distinct ARDA workstation in a guided follow-up.

### Acceptance

- Center insert and Settings never compete for the same hit target.
- Dashboard and CLI open different native windows.
- CLI accepts a harmless command, displays output, resizes, closes, and reopens.
- A failed terminal launch is reported as a terminal failure.

---

## Phase 1 — Legible animated monitor previews

### Tasks

- Add reusable preview primitives: grid, scanline, bars, pulse/wave, node graph, stream marks, status glyph, persistent title, and freshness indicator.
- Derive a stable preview preset from each source/workstation assignment.
- Animate with deterministic low-cost motion; stop or reduce motion for inactive/reduced-motion scenes.
- Angle all monitor planes inward and slightly downward toward the operator.
- Keep previews visible inside authored monitor models and procedural fallbacks.

### Acceptance

- Every monitor has persistent identity without hover.
- Screens no longer appear as hazy single-color rectangles.
- Preview type differs meaningfully across routing, planning, memory, and world/system sources.
- Clicking any preview opens the corresponding detailed workstation.
- Unavailable data does not resemble live telemetry.

---

## Phase 2 — Curved command-console geometry

### Tasks

- Replace the single flat desk box with center, inner-wing, and outer-wing structural segments.
- Create raised housings and recessed inserts that wrap around the operator.
- Add edge illumination, structural seams, fasteners, vents, rails, and control banks.
- Preserve stable slot anchors so visual models can later be replaced by GLB assets.
- Define collision-free zones for knobs, buttons, toggles, and displays.

### Acceptance

- Desk silhouette visibly wraps around the operator from the default camera.
- Insert surfaces read as embedded displays rather than colored blocks floating above a slab.
- No visible or clickable controls overlap.
- The scene remains usable at the minimum supported window size.

---

## Phase 3 — Ecosystem data adapters

**Status: Complete — 2026-07-30.** Compact previews, detailed-workstation navigation, presence provenance, bounded source windows, and honest degraded-state contracts are implemented and verified below.

### Initial source map

| Surface | ARDA source | Suggested visual presets | Detailed destination |
|---|---|---|---|
| Governance | approvals, gates, policy receipts | gate matrix, decision lanes | Governance/Guardhouse |
| Systems | health, service, fleet, resource state | node topology, pulse bars | Systems Health |
| Routing | Manwë/provider route and capability receipts | route graph, moving packets | Routing and Communication |
| Knowledge | ATHENA/MNEMOSYNE evidence and freshness | evidence stream, constellation | Knowledge and Reasoning |
| Planning | queue, run graph, blocked decisions | queue lanes, critical path | Planning/Workbench |
| Human realm | reminders, personal operations, business state | timeline, bounded gauges | Human Realm |
| Presence | agent presence ledger | constellation, activity pulse | Presence detail |

### Tasks

- Define typed compact preview models and source freshness metadata.
- Add Tauri/native readers or existing API adapters for each source.
- Normalize loading, unavailable, stale, degraded, blocked, and live states.
- Ensure preview and detail surfaces share IDs and timestamps.
- Add rate limits and bounded record windows to protect rendering performance.

### Acceptance

- No preview invents values.
- Each displayed value or state has a traceable source path/API and observed timestamp.
- Missing or malformed state fails visibly without breaking the boardroom.
- Rendering does not continuously parse large ledgers on the animation thread.

---

## Phase 4 — User-selectable visualization slots

**Status: Complete — 2026-07-31.** Typed presets, source compatibility, persisted per-slot configuration, recoverable profile controls, and native restart persistence are implemented and verified below.

### Tasks

- Add a visualization preset registry with compatible input kinds.
- Extend persisted scene assignments with preset/configuration fields.
- Add Settings UI for source and preset selection with live preview.
- Validate incompatible source/preset combinations.
- Add import/export/reset for boardroom profiles.

### Acceptance

- A user can change a screen visualization without code changes.
- Assignment survives restart.
- Invalid configuration fails closed and retains the last valid layout.
- Default profiles remain recoverable.

---

## Phase 5 — Expanded physical controls

Candidate reusable controls:

- service health/status button;
- approval queue button;
- local/cloud route selector;
- alert acknowledge control;
- scene/profile selector;
- timeframe knob for compatible graphs;
- data-density knob;
- focus/zoom toggle;
- emergency stop/cancel control with confirmation and authority checks.

Each control requires a typed action, an explicit authority class, visible disabled/error states, and a real verification path. Decorative controls must be visually distinguishable from actionable controls.

---

## Phase 6 — Authored visual fidelity and performance

- Replace stabilized procedural housings with optimized GLB assets where useful.
- Bake or atlas materials; budget draw calls and texture memory.
- Add restrained post-processing and emissive effects.
- Test low-power and reduced-motion modes.
- Add visual regression captures for the default profile and degraded-data state.

---

## Verification gates

Focused gates per slice:

```bash
pnpm --dir apps/arda-hud exec vitest run src/scene/boardroom
pnpm --dir apps/arda-hud run lint
pnpm --dir apps/arda-hud run build
cargo test --manifest-path apps/arda-hud/src-tauri/Cargo.toml
cargo check --manifest-path apps/arda-hud/src-tauri/Cargo.toml
```

Native visual/interaction gate:

1. Capture default boardroom frame.
2. Verify persistent monitor identity and distinct preview patterns.
3. Activate every monitor and record destination.
4. Activate Settings, Hermes Dashboard, and CLI separately.
5. Run an interactive terminal round trip.
6. Resize and reopen the main and terminal windows.
7. Simulate missing/stale source state and capture the honest degraded rendering.
8. Compare with the operator before the next geometry tranche.

---

## Implementation status — 2026-07-30

- Current visual/source/click contract audited against the live renderer, persisted slot state, scene geometry, and native command registration.
- Confirmed: center Settings hitbox overlap, duplicate pink Hermes assignment, missing terminal launcher command, hidden monitor identity, and flat single-color previews.
- **Implemented — Phase 0 code path:**
  - center command core no longer routes to Settings;
  - `SET`, `CLI`, and `HERMES` are separate, readable mesh controls on one front rail;
  - duplicate command-core controls were removed;
  - `open_hermes_terminal_window` is registered and focuses/reuses a bundled Tauri terminal window;
  - `terminal.html` is a first-class Vite build entry;
  - the auxiliary desk now defaults to `now_command` / Daily Command instead of duplicating Hermes Dashboard.
- **Implemented — first Phase 1 slice:**
  - all monitor and desk inserts render reusable HTML-in-Three.js HUD instruments;
  - live ARDA instrument models resolve by spatial zone or assignment slot;
  - unconnected displays render explicit offline standby schematics instead of false nominal telemetry;
  - rings, sweep, and node activity animate with a reduced-motion fallback;
  - upper monitors angle inward/down toward the operator;
  - projected HTML surfaces are scaled to remain within their 3D inserts.
- **Implemented — Phase 1 source-family preview slice:**
  - shared instruments now provide `topology`, `routes`, `lanes`, `constellation`, `pulse`, and `standby` SVG grammars rather than one repeated radial schematic;
  - fleet, routing, queue/planning, knowledge/memory, world/command, and unavailable/external sources resolve to meaningfully different preview families;
  - persisted slot assignments preserve monitor identity and destination even when workstation metadata is unavailable, so the upper surfaces remain `Warp`, `Routing`, `Memory + Continuity`, and `Planning + Queue` instead of generic monitor numbers;
  - offline sources remain dim, stationary, and explicitly labelled `NO DATA`; inactive-scene and reduced-motion CSS also stop instrument motion;
  - SET, CLI, HERMES, command-core, and monitor targets remain readable and non-overlapping.
- **Implemented — first Phase 2 command-console shell slice:**
  - replaced the single 6.8-unit flat desk slab with typed center, inner-left/right, and outer-left/right structural segments;
  - mirrored segment positions and progressively stronger wing angles create a wraparound operator silhouette while preserving all stable screen and control anchors;
  - each segment has an illuminated operator-side rail and paired metallic seam ribs, with source-family accents aligned to the embedded lower inserts;
  - the shell layout contract is independently testable and remains suitable for later procedural or authored-asset replacement.
- **Implemented — corrected Phase 2 model/insert alignment slice:**
  - removed the segment-wide raised rectangles that did not share transforms with the actual monitor slots;
  - upper monitors, all four lower displays, and the command core now render their authored GLB housings inside the same position/rotation group as their HTML display content;
  - lower HTML projections rotate into the local X/Z screen plane, and lower slot pitch now tilts the complete model-plus-display assembly toward the operator instead of skewing the display away from its housing;
  - generic translucent box hit meshes are suppressed on modeled surfaces while parent/model pointer activation remains intact;
  - the five-section shell retains its wrap, illuminated rails, vents, fasteners, and stable slot anchors without competing with the authored insert housings.
- **Implemented — first Phase 3 source-adapter slice (fleet health):**
  - fleet preview values continue to derive from the bounded fleet/operator runtime projection rather than parsing raw state in the renderer;
  - the fleet preview model now carries the selected provenance domain, source path list, observed/generated timestamp, and freshness state;
  - freshness and observation time are persistently projected in the fleet instrument footer, with exact source paths and timestamp exposed in its source label tooltip;
  - missing provenance fails visibly as `missing --:--Z` and retains `core/state/operator_runtime_status.json` as the expected source contract instead of implying fresh telemetry.
- **Implemented — Phase 3 compact-preview source adapters:**
  - added one bounded adapter contract for fleet, queue/planning, knowledge, routing, governance, human realm, and Daily Command source families; adapters match generated provenance by source path rather than assuming fixed provenance domain IDs;
  - each adapter combines at most eight unique source paths, selects the newest valid observed/generated timestamp, and projects the worst matched freshness state without parsing source ledgers in the render loop;
  - corrected semantic slot routing to match persisted assignments: Routing, Knowledge, and Operations occupy upper slots 2–4, while Governance, Fleet/Systems, Human Realm, and Daily Command occupy the four lower workstation slots;
  - all seven source-backed compact previews now carry source paths, observed time, and freshness; absent source families retain canonical expected paths and visibly render `NO DATA`/offline rather than a nominal state;
  - malformed or absent source state remains isolated to the affected instrument and does not break boardroom rendering.
- **Implemented — Phase 3 source/detail parity and presence closeout:**
  - compact source contracts now retain the bounded canonical provenance-domain ID list as well as source paths, with the primary ID selected from the same newest provenance record that supplies the displayed timestamp;
  - the presence projection now carries the canonical `data/prometheus/arda_presence_events.jsonl` source path and projects path, observation time, freshness, valid-row count, malformed-row count, and fallback reason without parsing the ledger in the render loop;
  - fresh/live, stale, blocked, malformed/unknown, and missing/fallback states have explicit test contracts; unsafe states resolve to watch/offline treatment instead of nominal telemetry;
  - source-backed monitor activation reaches the corresponding Routing, Knowledge, Operations, Governance, Fleet, Human Realm, and Daily Command detailed workstations;
  - fixed the no-manifest click path so persisted Human Realm and Daily Command slot identities fall back to functional slot workstation templates instead of silently doing nothing; the auxiliary fallback now uses Daily Command modules rather than the retired Hermes duplicate.
- **Implemented — Phase 4 visualization-slot closeout:**
  - added typed `standby`, `topology`, `routes`, `lanes`, `constellation`, and `pulse` presets with explicit compatible source families and fail-closed selection resolution;
  - persisted each slot's preset plus bounded density, timespan, and alert-threshold configuration while retaining backward compatibility with assignments that omit visualization data;
  - added source and preset selectors, compatible configuration controls, live preview, and recoverable profile export/import/reset actions to Settings;
  - projected persistence mode, save state, and diagnostics in an accessible Settings status surface;
  - corrected the native `write_scoped_file` argument contract (`numenorPath`) and covered it with a regression test, so browser-local fallback can promote to durable workspace persistence after a successful edit.
- **Automated evidence:**
  - `pnpm test -- --run` — 71 files, 266 tests passed;
  - `pnpm exec vitest run src/scene/boardroom/boardroomSpatialLayout.test.ts src/lib/boardroomSlotSettings.test.ts` — 17 tests passed after final control-rail adjustment;
  - `cargo test --lib` in `apps/arda-hud/src-tauri` — 14 tests passed;
  - `pnpm exec vitest run src/scene/boardroom/boardroomHudInstruments.test.ts src/scene/boardroom/boardroomSpatialLayout.test.ts src/lib/boardroomSlotSettings.test.ts` — 3 files, 24 tests passed after the source-family preview slice;
  - Phase 2 RED: focused spatial-layout test failed with no console-shell segments (`expected []`), proving the new shell contract was absent;
  - Phase 2 GREEN: `pnpm exec vitest run src/scene/boardroom` — 6 files, 27 tests passed;
  - Phase 2 alignment + Phase 3 fleet-source GREEN: `pnpm exec vitest run src/scene/boardroom` — 7 files, 34 tests passed;
  - `pnpm test -- --run` — 73 files, 278 tests passed after the alignment and fleet-source slices (existing React `act(...)` warnings remained non-failing);
  - Phase 3 compact-source GREEN: `pnpm exec vitest run src/scene/boardroom` — 8 files, 39 tests passed;
  - `pnpm test -- --run` — 74 files, 283 tests passed after compact adapters (existing React `act(...)` warnings remained non-failing);
  - `pnpm run build` — passed after the source-family preview slice, transformed 2,562 modules, and emitted `dist/terminal.html` plus the main HUD bundle;
  - `pnpm run build` — passed after the Phase 2 shell slice, transformed 2,562 modules, and emitted both application entries;
  - `pnpm run build` — passed after the Phase 2 alignment and Phase 3 fleet-source slices, transformed 2,563 modules, and emitted `dist/index.html` and `dist/terminal.html`;
  - `pnpm run build` — passed after the compact adapter slice, transformed 2,564 modules, and emitted both application entries;
  - Phase 3 closeout focused gate — 5 files, 37 tests passed across source adapters, compact instruments, presence projection/status, and slot workstation fallback contracts;
  - `pnpm test` — 75 files, 294 tests passed after the complete Phase 3 closeout (existing React `act(...)` warnings remained non-failing);
  - `pnpm build` — passed after the complete Phase 3 closeout, transformed 2,565 modules, and emitted `dist/index.html` and `dist/terminal.html`;
  - `cargo test --lib` — 14 tests passed; `cargo check` passed for the native HUD after loading the repository runtime build environment;
  - `git diff --check` — passed;
  - no `lint` package script exists; the successful build supplied the TypeScript compile gate.
  - Phase 4 focused gate — `pnpm exec vitest run src/lib/weathertop.test.ts src/scene/boardroom/boardroomVisualizationPresets.test.ts src/lib/boardroomSlotSettings.test.ts src/components/arda/modules/SettingsModule.test.tsx src/scene/boardroom/boardroomHudInstruments.test.ts` — 5 files, 31 tests passed;
  - Phase 4 full gate — `pnpm test -- --run` — 78 files, 303 tests passed (existing non-failing React `act(...)` warnings remained);
  - Phase 4 build gate — `pnpm run build` — passed, transformed 2,566 modules, and emitted both application entries;
  - Phase 4 native gate — `cargo test --lib` passed 14 tests and `cargo check` passed after loading the repository runtime build environment;
  - Phase 4 final `git diff --check` — passed.
- **Visual evidence:** with native `pnpm run tauri dev` sustained, Vite-mirror inspection confirmed four angled populated upper monitors with distinct Warp/standby, Routing/routes, Memory + Continuity/constellation, and Planning + Queue/lanes compositions. Five lower inserts and the readable non-overlapping `SET | CLI | HERMES` rail remained intact. Browser click dispatch reached the monitor buttons, but browser-only workstation-open parity remains unasserted because the non-Tauri page correctly reports the unavailable core-state/`invoke` bridge; this is not counted as native click-through evidence.
- **Phase 2 visual evidence:** with the native development process running, Vite-mirror inspection confirmed a visibly wrapped five-section desk silhouette with distinct center, inner, and outer wings plus continuous illuminated front rails. All four upper monitors, five lower surfaces, command core, and `SET | CLI | HERMES` remained visible without shell-induced overlap. The expected browser-only unavailable-`invoke` warnings remained unrelated to scene geometry.
- **Phase 2 corrected alignment visual evidence:** direct localhost vision inspection first reproduced the reported defect: segment-wide rectangular housings and independently skewed display cards. After correction, all four lower displays and the center command core visibly share the authored housing transforms, face the operator, and remain readable; the upper monitor cards also remain seated within their model frames. `SET | CLI | HERMES` stays unobscured. This remains browser geometry evidence only; unavailable Tauri `invoke` warnings are not counted as native source/action evidence.
- **Phase 3 degraded-state visual evidence:** localhost inspection with the core-state/Tauri bridge unavailable confirmed Routing, Knowledge, Operations, Governance, Fleet, Human Realm, and Daily Command simultaneously render pink offline/`NO DATA` treatment instead of nominal telemetry. All upper and lower projections remain seated inside their authored model frames with no new clipping, overlap, floating panel, or render fault.
- **Phase 3 native interaction evidence:** the sustained native Tauri process remained healthy beyond one hour. AT-SPI inspection found all eight boardroom activation targets, no render-fault surface, and confirmed click-through to Routing and Communication, Knowledge and Reasoning, Operations and Packages, Governance Controls, Systems Health, Desk Right Human Realm Template, and Desk Aux Daily Command Template. The missing live presence ledger rendered the explicit fallback contract; fresh, stale, blocked, malformed, and missing source transitions are covered by deterministic adapter/status tests without mutating canonical runtime state.
- **Phase 4 native persistence evidence:** AT-SPI changed `monitor_left_2` from Routes to Topology without code changes. `core/state/arda_boardroom_slots.json` then contained `visualization.preset_id: topology` with medium density, a 15-minute timespan, and a null alert threshold. After a complete native Tauri restart, Settings reported `workspace · idle · loaded core/state/arda_boardroom_slots.json` and the selector remained on Topology. The temporary probe was then restored through the same native control to the source-compatible Routes preset, and the workspace document recorded the restoration at `2026-07-31T00:05:55.313Z`.
- **Operator-confirmed Phase 0 acceptance:** native `CLI` and `HERMES` controls independently opened their intended windows successfully.
- **Remaining Phase 0 acceptance:** execute a harmless command in the native CLI window, verify displayed output, resize it, close/reopen it, and independently activate Settings.
- **Next guided tranche:** begin Phase 5 with expanded physical controls. Keep minimum-window and later visual-regression captures open as cross-phase native verification gates.
