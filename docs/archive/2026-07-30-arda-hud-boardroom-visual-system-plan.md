# ARDA HUD Boardroom Visual System and Ecosystem Integration Plan

> **For Hermes:** Implement this plan in visually reviewable slices. Preserve the existing boardroom, compare every slice in the live renderer, and do not claim native behavior from browser-only evidence.

**Goal:** Turn the current flat boardroom prototype into a futuristic, wraparound command console whose physical controls work, whose monitor inserts show meaningful live ARDA state, and whose screens can be assigned and customized without rebuilding the scene.

**Product role:** The 3D boardroom is a navigable status and control shell for inspectable ARDA workflows. It must reveal provenance, freshness, state, failures, and approval boundaries—not merely decorate backend data.

**Status — complete and archived 2026-07-30:** Implementation is complete through Phase 6. Automated frontend, production-build, asset-budget, and focused Rust gates pass. The operator accepted Phase 6 and directed plan closeout; future native UX refinements are follow-up work rather than open requirements in this plan.

**Current visual contract:** Five upper monitors, four lower desk inserts, one center insert, a presence emitter/avatar, and dedicated physical controls. Clicking a monitor or insert opens its assigned detailed workstation.

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

## Phase 5 — Expanded physical controls — implemented

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

## Phase 6 — Authored visual fidelity and performance — implemented

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
- **Implemented — first Phase 5 physical-control and composition slice:**
  - retained the typed `service_health_status` action with explicit `read_only` authority, `systems_health` detail target, and `fleetViewModel.status` verification path;
  - replaced the floating labeled `HEALTH | SET | CLI | HERMES` rail with four compact unlabeled mesh buttons embedded in one recessed desk control panel; the existing Settings, CLI, Hermes dashboard, and conditional Systems Health callbacks remain attached;
  - added mesh-level hover, press/release travel, emissive state feedback, enlarged invisible hit volumes, and stable control names/user-data without reintroducing floating HTML labels;
  - expanded the upright physical display arc from four to seven monitors while retaining the four persisted configurable monitor assignments and using bounded auxiliary workstation fallbacks for the three added visual positions;
  - widened the wall/window framing, tightened the camera composition around the curved desk and dense display rows, and restricted the large avatar/mission labels to debug mode so the default wall-monitor view remains desk-first;
  - kept service-health activation fail-closed when fleet projection state is unavailable while preserving its existing read-only Systems Health destination.
- **Implemented — Phase 5 reference-derived visual direction:**
  - adopted a hybrid wall-monitor composition instead of continuing to approximate the complete room with low-detail primitives: the operator-supplied `desk.jpg` now supplies only a cropped, mirrored rainy-city atmosphere plate while every monitor, desk display, button, hit surface, and action remains live Three.js geometry;
  - the atmosphere derivative excludes the seated person, chair, keyboards, reference monitor row, labels, and fake controls, and is checked into the scene asset registry with internal provenance metadata;
  - removed the synthetic skyline bars and heavy cyan wash that competed with the functional foreground, then matched the atmosphere plane to its authored panoramic aspect ratio;
  - replaced the five visibly disconnected box desk segments with one beveled concave obsidian console shell plus continuous cyan and magenta physical rails;
  - compacted and lowered the four desk displays and command core so they read as inserts in the console instead of oversized floating cards;
  - hid the crude controlled-workstation placeholder GLB outside debug mode while preserving its center-desk activation through an invisible physical hit surface;
  - removed the persistent keyboard-hint pill from the boardroom view so it no longer obscures the recessed physical controls.
- **Implemented — authored Blender physical-stage replacement:**
  - authored one production GLB containing the continuous concave desk shell, seven framed monitor housings, a faceted rear equipment spine, rectangular cantilever supports, five enlarged recessed display wells, and the preserved four-button physical tray;
  - replaced the runtime's procedural desk/monitor shell stack with that single GLB while retaining the existing HTML-in-Three interaction surfaces and typed callbacks as independent overlays;
  - removed the toy-like orb/post support joints after reference comparison and tied every monitor into the rear spine; darkened and roughened the obsidian/graphite materials so cyan and magenta remain accents rather than coloring the complete desk surface;
  - versioned the local layout-override key so stale browser/WebKit drag positions cannot keep browser and native runtimes on different geometry coordinates;
  - retained the `.blend` source, deterministic Blender build/export script, GLB, render, and provenance metadata under the boardroom scene assets.
- **Implemented — five-monitor operator composition refinement:**
  - retained the five functional upper monitor slots and their existing source, preview, and click contracts;
  - moved the default camera from `[0, 3.35, 11.2]` to `[0, 3.65, 8.8]` instead of globally scaling the physical stage, and raised its target to `[0, 2.02, 0.12]` so the desk sits lower and the floor is framed out at the native 1920×1080 viewport;
  - enlarged the existing center presence-emitter footprint from `1.16 × 1.16` to `1.45 × 1.45`, with ring, base, core, and light reach derived from that spatial-zone contract rather than duplicated fixed radii;
  - preserved the open central atmosphere above the emitter for a future avatar; center-monitor response to avatar activation remains the explicitly deferred follow-up rather than being simulated in this slice.
- **Implemented — lower-desk insert edge fitting:**
  - preserved the center insert and applied mirrored, aperture-local corrections to the four surrounding authored inserts rather than lifting or flattening the complete desk bank;
  - raised desk 1's inner/right edge and desk 5's inner/left edge by up to `0.07` Blender units while leaving each opposite outer edge flush;
  - raised desk 2's inner/rear corner and desk 4's mirrored inner/rear corner by up to `0.055` Blender units, with the adjacent edges receiving the planar half-lift needed to avoid a twisted display surface;
  - parented each insert's glass, four bezel bars, and accent rail to one `SurfaceFit` transform so each display remains a cohesive recessed assembly instead of separating the live aperture from its physical frame.
- **Implemented — Phase 5 typed-control closeout:**
  - registered all nine currently rendered tray and command-core actions in one typed contract: Service Health, Settings, Hermes CLI, Hermes Dashboard, ARDA Control, Approval Queue, Emergency Stop/Cancel, Route Selector, and World View;
  - each action now declares an authority class, target surface, and concrete verification path; the STOP control is `approval_required` and opens the governance guardhouse instead of performing an unreviewed mutation;
  - all four recessed tray buttons and all five command-core targets dispatch through the typed resolver; Settings, CLI, and Hermes Dashboard retain separate callbacks rather than being conflated;
  - controls expose ready, attention, disabled, and error states. Missing fleet projection disables Service Health, renders `NO DATA`, and produces dismissible visible feedback rather than appearing inert;
  - the Phase 4 Settings surface remains the bounded scene/profile, timespan, and density configuration path, while Approval Queue and Route Selector open their existing detailed workstations rather than duplicating complex selectors on the desk.
- **Implemented — Phase 6 performance and regression closeout:**
  - added deterministic `full`, `low-power`, `reduced-motion`, and `inactive` render profiles that bound DPR, frame-loop behavior, shadows, environment lighting, and scene motion from activity, hardware capacity, and `prefers-reduced-motion`;
  - presence-emitter animation now stops in reduced-motion/low-power modes, and inactive boardroom rendering uses `frameloop: never`;
  - added an executable boardroom asset-budget gate covering 82 authored `.glb`, texture, atmosphere, and HDR assets with per-file and total limits;
  - added two checked-in, dimension-validated, distinct WebGL canvas baselines at the actual 1278×513 capture viewport for the default render profile and degraded-data state. The degraded baseline requires the fail-closed mesh state; dismissible Drei HTML feedback is verified separately because it is not part of a WebGL-only capture;
  - retained authored GLB geometry and restrained material/emissive effects without enabling a broad post-processing pass that would add cost or reduce telemetry legibility.
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
  - Phase 5 service-health RED — focused Vitest failed because `boardroomPhysicalControls` did not exist, proving the typed control contract was absent;
  - Phase 5 service-health focused GREEN — 2 files, 14 tests passed across the new control contract/state derivation and spatial non-overlap contract;
  - Phase 5 boardroom gate — 11 files, 54 tests passed;
  - Phase 5 full frontend gate — 79 files, 307 tests passed (existing non-failing React `act(...)` warnings remained);
  - Phase 5 build gate — `pnpm run build` passed, transformed 2,567 modules, and emitted both `dist/index.html` and `dist/terminal.html`.
- Phase 5 physical-control/composition RED — spatial-layout tests failed at four monitors and absent `BOARDROOM_KINETIC_CONTROL_PANEL`, proving the seven-monitor arc and embedded panel contract were not present;
- Phase 5 physical-control/composition GREEN — focused spatial and physical-control tests passed, followed by `pnpm run build` passing with 2,567 transformed modules and both application entries emitted;
- Phase 5 physical-control/composition full gate — `pnpm exec vitest run --reporter=dot` passed 79 files and 307 tests (existing non-failing React `act(...)` warnings remained); `git diff --check` passed;
- Phase 5 reference-derived visual gate — `pnpm exec vitest run --reporter=dot` passed 79 files and 307 tests; `pnpm run build` passed with 2,568 transformed modules and emitted the new 232.37 kB atmosphere plate; `git diff --check` passed;
- no `lint` script exists in `apps/arda-hud/package.json`; the successful TypeScript/Vite build remains the frontend compile gate.
- Authored-stage focused gate — `pnpm exec vitest run src/scene/boardroom/boardroomSpatialLayout.test.ts src/scene/boardroom/boardroomMonitorModels.test.ts src/scene/systems/sceneAssets.test.ts --reporter=dot` passed 3 files and 14 tests; `pnpm run build` passed with 2,568 transformed modules and emitted `boardroom_physical_stage-7fZTs863.glb` at 2.18 MB; `git diff --check` passed.
- Blender export evidence — source and export were regenerated by Blender 5.2.0 LTS; final GLB SHA-256 is `8fb3db35998aaddbcac0d7a3d488febe3b16d2f80ae4c30f696000c4dd77a040`; the clean Tauri Vite server returned the same hash.
- Five-monitor composition focused gate — `pnpm exec vitest run src/scene/boardroom/boardroomComposition.test.ts src/scene/boardroom/boardroomSpatialLayout.test.ts --reporter=dot` passed 2 files and 12 tests; `pnpm run build` passed with 2,569 transformed modules and emitted both application entries; `git diff --check` passed.
- Lower-desk insert regeneration — Blender 5.2.0 LTS regenerated the `.blend`, reference PNG, and production GLB; the resulting GLB is structurally valid at 1,374,160 bytes and records all five `DeskInsert.*.SurfaceFit` nodes, with the center node retaining zero translation and rotation. Source and live-served GLB SHA-256 both matched `b129e95935323edcf746eb8d51390a6116e83681f0925fe9ab9228bcb27ebf27`.
- Lower-desk insert focused gate — `pnpm exec vitest run src/scene/boardroom/boardroomComposition.test.ts src/scene/boardroom/boardroomSpatialLayout.test.ts src/scene/boardroom/boardroomMonitorModels.test.ts --reporter=dot` passed 3 files and 15 tests; `pnpm run build` passed with 2,569 transformed modules; `git diff --check` passed.
- Phase 5/6 focused gate — `pnpm exec vitest run src/scene/boardroom/boardroomPhysicalControls.test.ts src/scene/boardroom/boardroomPerformance.test.ts src/scene/boardroom/boardroomVisualRegression.test.ts --reporter=dot` passed 3 files and 14 tests.
- Phase 5/6 full frontend gate — `pnpm test -- --run --reporter=dot` passed 82 files and 321 tests; existing non-failing React `act(...)` warnings remained.
- Phase 5/6 build gate — `pnpm run build` passed, transformed 2,570 modules, and emitted `dist/index.html` plus `dist/terminal.html`.
- Phase 6 asset-budget gate — `pnpm run verify:boardroom-assets` passed 82 files at 30,134,580 bytes against the 37,748,736-byte total limit with no violations.
- Presence projection regression gate — `cargo test -p arda-aule presence_projection` passed 7 tests with 1 filtered out; two pre-existing unused/dead-code warnings remained in `presence_projection.rs`.
- Final `git diff --check` passed.
- **Visual evidence:** live Vite-mirror inspection now confirms seven inward-facing upper monitors, five compact populated lower inserts, one continuous curved obsidian desk shell, a reference-derived rainy panoramic city, and one visibly recessed four-button control panel without persistent labels. The seated person, chair, keyboards, fake skyline bars, giant HTML rail, keyboard-hint pill, crude center-workstation placeholder, and default avatar/mission overlays are absent. Browser-only inspection still correctly reports the unavailable core-state/`invoke` bridge, so this composition check is not counted as native action click-through evidence.
- **Authored-stage native visual evidence:** after terminating the orphaned Vite process and restarting one clean Tauri-owned process tree, an exact X11 capture of active fullscreen window `ARDA HUD` (native PID 724517) confirmed the same authored physical stage in the native WebKit renderer: seven framed monitors connected to the continuous rear equipment spine, one curved dark desk, five recessed lower wells, and the physical four-button tray. No legacy disconnected procedural desk boxes or independent monitor shells remained. The capture's live surfaces were dark while core-state was unavailable, which is honest degraded data state rather than missing physical geometry.
- **Five-monitor composition native visual evidence:** after a full Tauri process-tree restart, an exact 1920×1080 X11 capture of `ARDA HUD` (native PID 776287) confirmed five complete inward-facing upper monitors, the closer desk-first camera, the front fascia cropped by the bottom edge with no floor strip, and a larger centered emitter. The panoramic atmosphere retains a broad unobstructed central area above the emitter for the future avatar. The extreme monitor frames approach the horizontal edges but remain visible; no monitor, emitter, or lower insert is harmfully clipped.
- **Lower-desk insert native visual evidence:** after regenerating the binary stage and restarting the complete Tauri tree, an exact 1920×1080 X11 capture of `ARDA HUD` (native PID 782628) confirmed continuous visible bezels at desk 1's right/top-right, desk 2's top-right, desk 4's top-left, and desk 5's full left edge including both corners. Desk 3 retained its accepted center transform. No corrected insert visibly intersects the continuous bank or floats above its aperture.
- **Phase 2 visual evidence:** with the native development process running, Vite-mirror inspection confirmed a visibly wrapped five-section desk silhouette with distinct center, inner, and outer wings plus continuous illuminated front rails. All four upper monitors, five lower surfaces, command core, and `SET | CLI | HERMES` remained visible without shell-induced overlap. The expected browser-only unavailable-`invoke` warnings remained unrelated to scene geometry.
- **Phase 2 corrected alignment visual evidence:** direct localhost vision inspection first reproduced the reported defect: segment-wide rectangular housings and independently skewed display cards. After correction, all four lower displays and the center command core visibly share the authored housing transforms, face the operator, and remain readable; the upper monitor cards also remain seated within their model frames. `SET | CLI | HERMES` stays unobscured. This remains browser geometry evidence only; unavailable Tauri `invoke` warnings are not counted as native source/action evidence.
- **Phase 3 degraded-state visual evidence:** localhost inspection with the core-state/Tauri bridge unavailable confirmed Routing, Knowledge, Operations, Governance, Fleet, Human Realm, and Daily Command simultaneously render pink offline/`NO DATA` treatment instead of nominal telemetry. All upper and lower projections remain seated inside their authored model frames with no new clipping, overlap, floating panel, or render fault.
- **Phase 3 native interaction evidence:** the sustained native Tauri process remained healthy beyond one hour. AT-SPI inspection found all eight boardroom activation targets, no render-fault surface, and confirmed click-through to Routing and Communication, Knowledge and Reasoning, Operations and Packages, Governance Controls, Systems Health, Desk Right Human Realm Template, and Desk Aux Daily Command Template. The missing live presence ledger rendered the explicit fallback contract; fresh, stale, blocked, malformed, and missing source transitions are covered by deterministic adapter/status tests without mutating canonical runtime state.
- **Phase 4 native persistence evidence:** AT-SPI changed `monitor_left_2` from Routes to Topology without code changes. `core/state/arda_boardroom_slots.json` then contained `visualization.preset_id: topology` with medium density, a 15-minute timespan, and a null alert threshold. After a complete native Tauri restart, Settings reported `workspace · idle · loaded core/state/arda_boardroom_slots.json` and the selector remained on Topology. The temporary probe was then restored through the same native control to the source-compatible Routes preset, and the workspace document recorded the restoration at `2026-07-31T00:05:55.313Z`.
- **Operator-confirmed acceptance and closeout:** the embedded Settings, CLI, and Hermes buttons look and behave as physical controls and open their intended windows; the monitor row remains clickable. The native CLI accepted `ls`, displayed output, and supported terminal navigation. The prior Service Health inert-press gap is fixed in source and covered by deterministic/browser evidence. On 2026-07-30 the operator accepted Phase 6 as done and directed this plan to close; resize/reopen or additional native click-through refinements may be tracked separately if they become necessary.
