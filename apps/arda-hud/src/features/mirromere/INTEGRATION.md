---
soterion:
  sigil: "REPAIR"
  role: "integration_contract"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: REPAIR integration_contract | owner: HERMES | status: active | reviewed: 2026-08-17

# Mirromere HUD Integration Seam

This note freezes the Phase 3 integration seam before the shared Mirromere contract and its two renderers are added. It records current source truth only; it does not claim a Mirromere projection, native second-monitor placement, or physical acceptance.

## Live-source discrepancy

The plan names `src/components/arda/core/BoardroomStage.tsx`, but that file does not exist. The live boardroom owner is `src/scene/boardroom/BoardroomViewport.tsx`. Phase 3 must extend that path rather than creating a parallel boardroom stage.

## Selected in-world aperture

Use canonical upper slot `monitor_3`, rendered by the `BOARDROOM_MONITOR_ZONES` loop in `BoardroomViewport.tsx`, as the Mirromere proving aperture.

The selected seam is the existing ambient branch:

- `BoardroomViewport.tsx` resolves each physical upper monitor to canonical `monitor_1` through `monitor_5`.
- A typed monitor session has first priority, an active agent claim has second priority, and the ambient renderer is used only when neither owns the slot.
- The current ambient branch mounts `UpperAmbientMonitorScreen`; Phase 3 may substitute `MirromereAperture` for `monitor_3` only when no session or claim owns that slot.
- Existing session/claim priority and monitor ownership rails remain authoritative.
- The aperture remains passive in-world presentation. It must not become World View workflow acceptance and must not intercept monitor activation unless a backend-allowlisted Mirromere interaction exists.

`monitor_3` is selected because its stable ambient identity is already `signal_mandala`, which is the nearest existing upper-screen vector/radar language. No monitor slot, ownership registry, or parallel renderer manager is introduced.

## Existing scene and workstation path

Phase 3 reuses these live paths:

1. `App.tsx` loads the backend-sourced bundle and passes boardroom projections into `BoardroomViewport`.
2. `BoardroomViewport.tsx` owns upper-monitor arbitration and the passive in-scene aperture.
3. Existing inspect/focus behavior continues through `spawnFloatingWorkstation`, `SceneWorkstation`, and `PanelWorkspace`.
4. Native workstation routes continue through `windowManager.open`, `open_workstation_window`, and the query parameters parsed by `App.tsx`.

Do not add a second boardroom stage, floating-workstation manager, monitor registry, or window manager for Mirromere.

## Native window and route convention

`src/utils/multiWindow.ts` is the frontend authority for managed secondary windows:

- `WindowConfig` carries stable id, role, workstation identity, source zone, presentation mode, dimensions, URL, and placement intent.
- Workstation-role windows invoke Rust command `open_workstation_window` with one nested `request` object.
- Browser fallback uses the same route keys: `__windowId`, `__windowRole`, `__workstation`, `__section`, `__anchor`, and `__presentation`.
- `App.tsx` parses `__windowId`, `__windowRole`, `__workstation`, `__section`, and `__view` before composition, while the existing route builder also carries `__anchor` and `__presentation`; persisted `WorkstationBridgeState` is reused through `workstation-sync`.
- `monitorSessionWorkstationRoute.ts` demonstrates stable session-derived window identity and returns the existing `WindowConfig` shape.

The native Mirromere surface must extend this convention and `WindowManager`; it must not add a parallel JavaScript window registry. Physical monitor selection and fail-closed disconnect behavior belong in the Rust/Tauri window command because browser screen enumeration currently exposes only the active screen and silently falls back to primary.

## Frozen source-mode contract

- Runtime data enters the HUD through `createCoreStateSource()` and the `ArdaBundle` path.
- Existing tests use explicit imported JSON/TypeScript fixtures. Mirromere fixtures must remain test-only and carry a structural `source_mode: fixture` distinction from runtime surfaces.
- Runtime Mirromere data must come from a bounded backend projection/Tauri command and carry `source_mode: runtime`, timestamps, freshness, and evidence references.
- React must not discover lifecycle, continuity, or arbitrary files directly.
- `detectArdaRuntimeMode()` currently reports environment/shell context, not projection provenance, and cannot substitute for Mirromere source mode.

## Frozen interaction contract

- Passive ambient upper monitors are non-interactive: `isUpperMonitorInteractive('ambient')` is false.
- Existing monitor activation remains session/claim-first and routes through `onOpenMonitorSession`, `onOpenMonitorSurface`, or the existing workstation activation callback.
- Mirromere may expose only interaction ids present in its backend allowlist.
- Inspect/provenance may reuse the existing workstation/detail path.
- Conversation handoff and every mutation require explicit operator action and a backend receipt; the UI never mints success.

## Frozen monitor contract

- Canonical boardroom monitor ids are `monitor_1` through `monitor_5`.
- Session records and active claims outrank passive ambient/Mirromere pixels.
- The HUD aperture is a simulation/design lens only.
- Physical Mirromere acceptance requires a separate packaged Tauri window on an explicitly selected non-primary monitor.
- Selection must be stable across coordinate changes and must not silently fall back to primary.
- Disconnect must close or privacy-veil the native surface and report unavailable.

## Frozen reduced-motion contract

`BoardroomViewport.tsx` observes `(prefers-reduced-motion: reduce)` and passes the result through `resolveBoardroomRenderProfile`. Existing upper ambient rendering also independently suppresses animation under that media query.

Mirromere renderers must consume an explicit reduced-motion behavior from the shared surface contract and combine it with the operator media preference. Reduced motion freezes or simplifies motion without hiding status, provenance, privacy state, or controls. The HUD aperture and native window must resolve the same semantic state.

## Product and evidence boundary

- HUD World View remains display-only and is not an application-workflow acceptance surface.
- Browser preview, jsdom, fixtures, and screenshots cannot prove physical Mirromere behavior.
- The aperture and second-monitor window must consume the same typed surface model and interaction ids.
- Native proof requires the packaged HUD, selected physical display, privacy veil, disconnect/reconnect, reduced motion, keyboard/escape behavior, and measured frame-time/FPS evidence.

## Task 1 baseline

Executed from `apps/arda-hud` before this file was created:

```text
pnpm exec vitest run \
  src/utils/multiWindow.test.ts \
  src/scene/boardroom/monitorSessionWorkstationRoute.test.ts \
  src/scene/boardroom/upperAmbientSignal.test.ts \
  src/components/arda/core/SceneWorkstation.test.tsx \
  src/scene/world/WorldTerminalSurfacePreview.test.tsx
```

Result: **5 test files passed; 12 tests passed; exit 0** on 2026-08-17. This proves the selected seam's existing focused contracts only. It does not prove Mirromere behavior or native second-monitor acceptance.
