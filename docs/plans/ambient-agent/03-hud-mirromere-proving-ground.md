---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-17

# Phase 3: HUD and Mirromere Proving Ground Implementation Plan

> **For Hermes:** Use the `subagent-driven-development` skill to implement this plan task-by-task. Preserve the ARDA HUD's agent-native visual language and native acceptance requirements.

**Goal:** Use one governed presentation contract to render Mirromere applications both inside an existing ARDA HUD monitor aperture and on the operator's physical second monitor, proving future mirror behavior without turning the HUD World View into a workflow surface.

**Architecture:** A backend-owned `arda.mirromere.surface.v1` projection describes scene/application state. Two thin consumers render it: an in-world HUD aperture for rapid visual testing and a separate native Tauri display window positioned on a selected monitor. The same provenance, privacy, freshness, and interaction rules apply to both.

**Tech stack:** React 19, TypeScript, Vitest, Three.js/react-three-fiber, Tauri 2 multiwindow/monitor APIs, Rust source projection, existing ARDA workstation and boardroom components.

---

## Product boundary

The boardroom/HUD remains the operator's desktop embodiment. The HUD World View is display-only and intentionally sparse; it is not acceptance for application workflows. Mirromere is a calm display/application outpost. The useful two-for-one is to reuse the same Mirromere surface contract in:

1. one HUD screen/aperture as a simulation and design lens; and
2. one separate native window on the second physical monitor as operational acceptance.

Do not duplicate application logic in a Three.js texture and a second React page. Both renderers consume the same typed model and interaction ids.

## Current source baseline

- `apps/arda-hud/src/components/arda/core/SceneWorkstation.tsx` and `PanelWorkspace.tsx` implement existing workstation presentation paths.
- `apps/arda-hud/src-tauri/src/lib.rs` already contains native window creation machinery and workstation request/result types.
- `apps/arda-hud/src-tauri/tauri.conf.json` currently declares one fullscreen transparent native window.
- `apps/arda-hud/src/lib/ardaSource.ts` owns backend-sourced HUD projections.
- Existing source tests distinguish fixture/fallback/runtime modes; preserve that honesty.
- The current visual branch is the correct implementation lineage for HUD changes; isolate shared schemas before parallel renderer work.

## Mirromere surface contract

Create `arda.mirromere.surface.v1` with:

- surface/outpost id and intended display role;
- scene/application id, version, and human-readable purpose;
- slots with typed content: status, text, media reference, vector/radar/wave field, conversational presence, or registered app view;
- data source and evidence references;
- freshness/expiry and explicit unavailable state;
- privacy class and visibility ceiling;
- interaction ids from a backend allowlist;
- accessibility description, reduced-motion behavior, and urgency;
- transition policy and attention budget;
- no arbitrary HTML, JavaScript, URL, shell command, or unsanitized remote media.

First registered scenes:

- `ambient.idle`;
- `system.starting` / `system.degraded`;
- `conversation.presence`;
- `continuity.handoff-ready`;
- `research.focus` using Varda provenance;
- `privacy.veil`;
- `offline.local`.

## Task 1: Inventory and freeze the HUD integration seam

**Files:**
- Read/trace: `apps/arda-hud/src/App.tsx`
- Read/trace: `apps/arda-hud/src/components/arda/core/BoardroomStage.tsx`
- Read/trace: `SceneWorkstation.tsx`, `PanelWorkspace.tsx`, `types.ts`
- Read/trace: `apps/arda-hud/src/lib/ardaSource.ts`
- Read/trace: `apps/arda-hud/src/utils/multiWindow.ts`
- Create: `apps/arda-hud/src/features/mirromere/INTEGRATION.md`

**Steps:**
1. Identify the exact upper-monitor aperture that can host a display-only preview without replacing World View acceptance.
2. Identify the existing native window query/routing convention.
3. Record current source-mode, interaction, monitor, and reduced-motion contracts.
4. Select one existing scene/workstation path; do not add a parallel window manager.
5. Run existing focused tests before edits and record the baseline command/results.
6. Commit the seam inventory separately.

## Task 2: Define strict shared surface types and fixtures

**Files:**
- Create schema/types in the existing cross-boundary contract location selected after source trace
- Create: `apps/arda-hud/src/features/mirromere/types.ts`
- Create: `apps/arda-hud/src/features/mirromere/fixtures.ts` for tests only
- Test: Rust strict-deserialization and TypeScript parser/type-guard tests

**Steps:**
1. Write failing cases for valid idle/degraded/handoff scenes, unknown field, expired scene, privacy escalation, unknown interaction, arbitrary URL/HTML, oversized slot, and missing evidence source.
2. Implement strict backend types and bounded frontend decoding.
3. Ensure fixture mode is visibly and structurally distinct from live runtime mode.
4. Generate or validate TypeScript against the canonical schema rather than hand-maintaining divergent enums.
5. Run focused Rust and Vitest suites.
6. Commit: `feat(hud): define Mirromere surface contract`.

## Task 3: Add backend-owned Mirromere projection

**Files:**
- Extend the current HUD backend/source projection path; exact module chosen from Task 1 trace
- Modify: `apps/arda-hud/src/lib/ardaSource.ts`
- Test: backend projection and source-mode tests

**Steps:**
1. Write failing tests for runtime, stale, unavailable, privacy veil, and fixture isolation.
2. Compose surface state from Phase 1 lifecycle and Phase 2 continuity references; do not read random files directly from React.
3. Add a bounded projection endpoint/Tauri command following the existing source pattern.
4. Preserve source timestamps and evidence links.
5. Run backend tests and direct consumer checks.
6. Commit: `feat(hud): project governed Mirromere scenes`.

## Task 4: Render the in-world HUD aperture

**Files:**
- Create: `apps/arda-hud/src/features/mirromere/MirromereAperture.tsx`
- Modify: the selected boardroom monitor component from Task 1
- Add styles/shaders only under the existing HUD feature/style convention
- Test: `MirromereAperture.test.tsx` plus affected boardroom tests

**Steps:**
1. Write failing tests for idle, lifecycle degraded, handoff-ready, privacy veil, stale, and reduced-motion views.
2. Build a nearly textless vector/radar/wave presentation consistent with the lower/upper screen role being used; avoid generic dashboard cards and repeated labels.
3. Make provenance/state inspectable through the existing workstation or detail path without filling the scene with text.
4. Route interactions through existing activation callbacks and backend allowlisted ids.
5. Verify no polling/render-loop FPS regression.
6. Run focused tests, full HUD tests, lint, and build.
7. Commit: `feat(hud): render Mirromere proving aperture`.

## Task 5: Add a native second-monitor Mirromere window

**Files:**
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Modify: `apps/arda-hud/src/utils/multiWindow.ts`
- Create: native Mirromere window route/component under existing app routing convention
- Test: Rust monitor-selection unit tests and frontend route tests

**Steps:**
1. Write failing pure tests for requested display id present, absent, disconnected, primary-display fallback disabled, and geometry change.
2. Enumerate monitors through Tauri; persist a stable operator selection separately from transient coordinates.
3. Open one borderless native Mirromere window on the selected second monitor. Do not steal primary-display focus after initial operator configuration.
4. On disconnect, close or veil the surface and report unavailable; never silently move private content to the primary display.
5. Reuse the exact `MirromereSurface` renderer/model from Task 4 with an environment adapter for native geometry.
6. Run Rust tests, Vitest, lint, build, then `pnpm run tauri build`.
7. Commit: `feat(hud): project Mirromere to selected display`.

## Task 6: Add scene registration and guarded interaction

**Files:**
- Create: `apps/arda-hud/src/features/mirromere/sceneRegistry.ts`
- Modify backend projection/commands selected earlier
- Test: registry and interaction-policy tests

**Steps:**
1. Write failing tests rejecting unknown scene ids, unregistered interactions, privacy mismatch, and expired action.
2. Register only the seven initial scenes listed above.
3. Keep read-only scene switching automatic when low risk; require explicit operator action for conversation handoff or any mutation.
4. Record bounded receipts for interaction requests; UI does not mint success.
5. Commit: `feat(mirromere): register bounded ambient scenes`.

## Task 7: Visual, native, and performance acceptance

**Run:**
1. Launch the packaged native HUD through Phase 1.
2. Verify the selected HUD aperture renders the live Mirromere scene and is clearly in-world/display-only.
3. Open the native Mirromere surface on the physical second monitor.
4. Drive identical idle, starting, degraded, handoff-ready, privacy, stale, and offline states through the backend contract.
5. Compare both consumers for semantic equivalence, not pixel identity.
6. Disconnect/reconnect the second monitor and verify safe veil/recovery.
7. Exercise reduced motion and keyboard/escape control.
8. Run the existing HUD performance acceptance; reject any meaningful FPS/frame-time degradation.
9. Confirm browser preview or static screenshots are not used as native proof.
10. Record a real operator session using the second monitor for conversation presence and one Varda research visualization.

## Phase gate

Phase 3 is **proven** only when one backend-owned scene contract drives both the in-world HUD aperture and a packaged native window on the physical second monitor, including privacy veil, disconnect recovery, source freshness, and no accepted FPS regression. The HUD aperture alone is a useful testbed but not physical Mirromere acceptance.
