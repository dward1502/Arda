# ARDA HUD Agentic Monitor Surfaces — Slice Checklist

> Companion to `2026-07-30-arda-hud-agentic-monitor-surfaces-plan.md`.
> Checklist items are verifiable with the project's existing gates:
> `pnpm exec vitest run ...` and `cargo check`/`cargo test` against
> `apps/arda-hud/src-tauri/Cargo.toml`.
>
> Plan slice IDs map 1:1 to the source plan sections.

## Evidence baseline (verified 2026-08-03)

### Spatial layout (`src/scene/boardroom/boardroomSpatialLayout.ts`)
- `BOARDROOM_SPATIAL_ZONES` defines **14 interaction zones**: 5 `upper_monitor`,
  4 `desk_surface`, 1 `control_panel`, and 4 `physical_button` zones. Avatar and
  world-window anchors are modeled separately from this interaction-zone array.
- `BOARDROOM_MONITOR_ZONES` = filter kind `upper_monitor` → **5 zones** (ids
  `boardroom.monitor.{left,center_left,center,center_right,right}`).
- Slot ids attached to monitors: `monitor_left_1..monitor_left_4` (only 4 of the
  5 monitor zones carry an `assignmentSlotId`; `boardroom.monitor.center` has
  `assignmentIndex: 2` but no `assignmentSlotId`).
- `BOARDROOM_CONTROL_ZONES` = filter kind `desk_surface && assignmentSlotId`
  → 4 zones (matches "4 lower desk surfaces").

### Plan vs code discrepancy (corrected)
- **The source plan baseline now records the correct count: 5 upper monitors
  plus desk surfaces.** Code defines 5 monitor zones and the
  test `boardroomSpatialLayout.test.ts:29` asserts `toHaveLength(5)`.
  Checklist treats code (5 monitors) as authoritative.

### Existing types (`src/lib/boardroomSlotSettings.ts`)
- `BoardroomSurfaceAdapterType` already includes: `agent_activity`,
  `streaming_text`, `remote_desktop`. (Plan Slice A asks to "add" these — they
  already exist; verify/extend instead.)
- `BoardroomSurfaceWidgetKind` already includes `iframe_preview`,
  `remote_session`. (Already present — extend test coverage.)
- `BoardroomSurfacePreviewMode` already includes `agent_activity`.
- `BoardroomSurfaceFocusMode` = `in_scene_workstation | native_window |
  external_browser | inline_embed`. No `remote_preview` focus mode exists yet.

## Slice A — Agent surface contract and manifest extension
**Scope**: TypeScript types and defaults only. No UI or Rust changes.

- [x] Confirm `BoardroomSurfaceAdapterType` already carries `agent_activity`,
      `streaming_text`, `remote_desktop` (it does — no new types needed here).
- [x] Confirm `BoardroomSurfaceWidgetKind` already carries `iframe_preview` and
      `remote_session` (it does — no new kinds needed here).
- [x] Add the missing **`remote_preview`** focus mode to
      `BoardroomSurfaceFocusMode` (plan B2 requires routing to
      `remote_preview`; the type did not exist yet).
- [x] Add **`remote_preview`** to `SurfaceAdapterFocusMode` in
      `surfaceAdapterManifests.ts` for consistency.
- [x] Extend `getSurfaceAdapterFocusContract` in `surfaceAdapterManifests.ts`:
      adapters with `embedUrl` but `allowInlineEmbed=false` now resolve to
      `remote_preview` focus mode instead of `blocked`.
- [x] Harden `parseSurfaceLayout` widget parser: unknown `kind` strings now
      validate against `isBoardroomSurfaceWidgetKind` and fall back to the
      default layout widget at that index (was: unsafe `as` cast).
- [x] Add malformed-input unit tests in `boardroomSlotSettings.test.ts` for:
      - unknown `adapter_type` string → preserved but safe.
      - unknown `preview.mode` string → preserved but safe.
      - unknown widget `kind` → falls back to default layout widget kind.
      - missing widget `kind` → falls back to default layout widget kind.
      - non-string `focus.target` → falls back to layout default.
      - non-finite `refresh_ms` → falls back to layout default.
      - non-boolean `allow_inline` → falls back to boolean default.
      - round-trip: `remote_preview` focus mode survives export/import.
- [x] Add malformed-input tests in `boardroomSurfacePreviewModel.test.ts` for:
      - unknown adapter type → tone/glyph fall back to cyan/GRID.
      - `iframe_preview` widget with layout disabled → status `disabled`.
      - `iframe_preview` widget with `allow_inline=false` → status `attention`.
      - layout disabled → all widgets `disabled`, overall status `disabled`.
- [x] Gate: `pnpm exec vitest run src/lib/boardroomSlotSettings.test.ts
       src/scene/boardroom/boardroomSurfacePreviewModel.test.ts` passes (21
      tests, all green).
- [x] Gate: full verification suite passes (34/34 tests):
      `vitest run src/lib/boardroomSlotSettings.test.ts
       src/scene/boardroom/boardroomSurfacePreviewModel.test.ts
       src/scene/boardroom/boardroomSpatialLayout.test.ts
       src/components/arda/modules/SettingsModule.test.tsx`
- [x] Gate: `npx tsc --noEmit` passes (0 errors).

## Slice B — Runtime surface slots and assignment claims
**Scope**: Runtime claim/release model in TS state; persisted doc updates.

- [x] Add `agent_claims` field type to `BoardroomSlotAssignmentRecord` (owner,
      activity_kind, payload_binding, fallback_preview, lease_expires_at_utc).
      Added `BoardroomAgentClaim` interface; claims are optional and default
      to `undefined` (backward compatible).
- [x] Add `parseBoardroomAgentClaims` helper that validates `owner` (non-empty
      string), `activity_kind` (against known set), `lease_expires_at_utc`
      (valid string); filters malformed entries.
- [x] Wire `agent_claims` parsing into `parseBoardroomSlotSettings` so imported
      documents round-trip claims through parse.
- [x] Add slot-to-source runtime resolution path:
      `resolveMonitorSlotSource(slotId, document, nowUtc)` returning
      priority 1 claimed live binding (non-expired lease), 2 persisted static
      binding, 3 workspace fallback. Returns null for non-monitor (desk) slots.
- [x] Add `claimMonitorSlot(document, slotId, claim)` — writes a claim without
      clearing unrelated slot assignments or other owners' claims (replaces same-owner).
- [x] Add `releaseMonitorSlot(document, slotId, owner)` — clears a single owner's
      claim, preserves all other claims and all other slots.
- [x] Add `resetMonitorSlot(document, slotId)` — clears `agent_claims` and
      restores default `surface_layout` for the slot; other slots untouched.
- [x] Wire into `useBoardroomSlotAssignments` hook: exposes
      `monitorSlotSources`, `claimMonitorSlot`, `releaseMonitorSlot`,
      `resetMonitorSlot` for runtime callers (BoardroomViewport etc.).
- [x] Add tests: claim+resolve, fallback when no live claim, desk-slot rejection,
      release preserves others, reset restores defaults, export/import round-trip,
      malformed claim filtering.
- [x] Gate: `vitest run` passes (41/41 tests: 34 prior + 7 new).
- [x] Gate: `tsc --noEmit` passes (0 errors).

## Slice B2 — Desk/monitor focus router
**Scope**: Interaction routing in `BoardroomViewport.tsx`.

- [x] Verify desk activation stays on `open_workstation` path (no free agent
      write to desk slots). Control zones retain original `onOpenWorkstation` handlers;
      only monitor zones route through `resolveMonitorFocus`.
- [x] Add `resolveMonitorFocus(slotId, assignments, sources, layouts, claims, nowUtc)`
      exported from `BoardroomViewport.tsx` — resolves priority 1 active claim, 2
      persisted assignment, 3 workspace/default. Uses layout `focus.mode` when
      available. Returns null for desk slots.
- [x] Add `focusMonitorSlot` branch in monitor zone InteractionPad `onActivate`:
      resolves focus mode, active claim source zone, and routes accordingly.
      `native_window` / `external_browser` → `onOpenWorkstation`; `remote_preview`
      → in-scene surface (delegates to existing `HudInstrumentSurface`).
- [x] Add test suite `boardroomMonitorFocus.test.ts` (6 tests): desk-slot rejection,
      active claim resolution, persisted fallback, default focus mode, expired/
      inactive claim fallback, layout focus mode override.
- [x] Desk zone click handlers unchanged (controlZones map uses `onOpenWorkstation`
      directly, not `handleMonitorActivate`).
- [x] Gate: `tsc --noEmit` passes (0 errors).
- [x] Gate: `vitest run` passes (47/47 tests across 5 files: 21 + 6 + 7 + 11 + 2).

## Slice C — Tauri surface bridge and native window entry
**Scope**: Rust/Tauri side for monitor surfaces only.

- [x] Add `SurfaceBridge` command set in `src-tauri/src/lib.rs`:
      `create_monitor_surface` (create/focus native window for a monitor slot),
      `dismiss_monitor_surface` (close a monitor surface window). Both registered
      in `generate_handler![]`.
- [x] Add `MonitorSurfaceRequest` and `SurfaceBridgeResult` response types
      (serialized with camelCase serde).
- [x] Gate surface creation by `is_allowed_focus_mode` (validates against
      `in_scene_workstation | native_window | external_browser | remote_preview`).
      Failure returns explicit `ok: false` error, not silent fallback.
- [x] Gate surface source by `is_allowed_surface_source` (monitors `monitor_left_*`,
      `*service_*` zones, `hermes_runtime`; allows desk slot zone IDs). Rejection
      returns explicit error message.
- [x] Add scoped surface entry URL: `index.html?__view=panel&__windowId=...&__windowRole=monitor&__slot=...&__source=...`
      for in-shell iframe-style inspection (gated by source validation).
- [x] Wire `hermes_runtime` source to ensure the Hermes runtime process is running
      before creating the window.
- [x] Emit `monitor-surface-sync` event on creation so the TS layer can react.
- [x] Add TS-side invoke helpers: `createMonitorSurface` and `dismissMonitorSurface`
      in `boardroomSlotSettings.ts` (lazy-import `@tauri-apps/api/core`).
- [x] Gate: `cargo check --manifest-path src-tauri/Cargo.toml` passes (2 warnings only).
- [x] Gate: `cargo test --lib --manifest-path src-tauri/Cargo.toml` passes (22/22:
      19 prior + 3 new surface bridge tests).
- [x] Gate: `npx tsc --noEmit` passes (0 errors).
- [x] Gate: `vitest run` passes (47/47 across 5 test files).

## Slice D — Agent-facing monitor surface API
**Scope**: How an agent creates/claim/update/release a monitor surface.

- [x] Define narrow agent API in `commands/monitor_surface.rs`:
      `claim_monitor_slot`, `release_monitor_slot`, `push_surface_payload`,
      `refresh_monitor_slot_lease`. All operate on monitor slots only — desk
      slots explicitly rejected with visible error.
- [x] Desk surfaces excluded from direct agent mutation: `is_monitor_slot_id`
      gate in every command rejects desk slot IDs with `ok: false` + descriptive
      message.
- [x] `claim_monitor_slot` gates via focus mode validity (`is_allowed_focus_mode`),
      activity kind set validation, and non-empty owner/payload_binding.
- [x] `push_surface_payload` authorizes the exact active owner and payload binding,
      then emits a camelCase `monitor-surface-payload` event on the target webview
      window (requires `tauri::Emitter`).
- [x] `release_monitor_slot` clears only the exact active owner's claim; desk
      slots, missing claims, and owner mismatches are rejected.
- [x] `refresh_monitor_slot_lease` extends the exact active owner's lease by 300s.
      Desk slots, missing claims, empty owners, and owner mismatches are rejected.
- [x] Add managed `MonitorSurfaceState` ownership registry. Conflicting live
      owners are rejected until expiry or exact-owner release.
- [x] Restrict owners to `hermes-agent` / `hermes-agent-*` and payload bindings to
      `hermes.*`, `queue.*`, `agent.*`, or `stream.*` namespaces.
- [x] Register all 4 commands in `generate_handler![]` in `lib.rs`.
- [x] Export types: `MonitorClaimRequest`, `MonitorClaimResult`, `MonitorSurfacePayload`,
      `MonitorSurfacePayloadResult`, `LeaseRefreshRequest` (all `#[serde(rename_all = "camelCase")]`).
- [x] Make `surface_bridge_window_label` and `is_allowed_focus_mode` `pub(crate)`
      so the `commands/` submodule can call them.
- [x] TS-side invoke wrappers in `boardroomSlotSettings.ts`:
      `agentClaimMonitor`, `agentReleaseMonitor`, `agentRefreshMonitorLease`,
      `agentPushSurfacePayload` (+ `AgentClaimMonitorRequest`, `AgentClaimResult`,
      `AgentSurfacePayload`, `AgentSurfacePayloadResult` interfaces).
- [x] Gate: `cargo test --lib` passes (38/38, including 16 monitor-surface contract,
      ownership, authorization, lease, and payload tests).

## Slice E — Live example monitor surface
**Scope**: One concrete monitor surface proving the agent path.

- [x] Candidate: Hermes/Dashboard terminal stream rendered as live monitor surface.
      Uses `agent_activity` adapter type with `hermes.live_stream` payload binding
      on `monitor_left_*` slots only.
- [x] Create `HermesDashboardMonitorSurface` component in `BoardroomViewport.tsx`:
      renders compact live terminal stream preview on claimed monitor with
      `remote_preview` focus mode.
- [x] Render compact preview on claimed monitor from real
      `monitor-surface-payload` events. The payload consumer is bounded to 4 KiB,
      normalizes MIME, and preserves explicit `NO DATA` fallback. Lease status
      turns red (`attention`) when < 5s remains.
- [x] Focusable/expandable on activate: clicking the surface calls
      `onOpenWorkstation(workstationZoneId)` which opens the Hermes dashboard.
- [x] Respect reduced-motion policy: uses `renderProfile.motionEnabled` (derived
      from CSS `prefers-reduced-motion: reduce`). When motion disabled, shows
      static `[NO DATA — reduced-motion mode active]` instead of animated stream.
- [x] Stale/missing monitor state renders `NO DATA` — lease expiry < 5s shows
      `attention` status with red indicator; stale claim shows `STALE` instead of `active`.
- [x] Agent claim release/refresh controls: ⟳ (refresh lease) and ✕ (release claim)
      buttons rendered in `monitor-surface-agent-controls` overlay.
- [x] CSS styles added in `hud-instruments.css` for `.hud-instrument--monitor-surface`,
      `.monitor-surface-agent-controls`, and `@media (prefers-reduced-motion: reduce)`
      override.
- [x] Integration: monitor zone map now resolves each scene zone to its stable
      `assignmentSlotId` and renders `HermesDashboardMonitorSurface` for every
      active claim. The configured focus mode controls activation behavior rather
      than suppressing the live claim preview.
- [x] Desk surfaces unchanged — control zones keep original `onOpenWorkstation` handlers.
- [x] Gate: claimed monitor consistently shows live payload (4 resolveMonitorFocus tests
      verify claim resolution, desk rejection, expiry fallback, layout override).
- [x] Gate: desk surfaces remain unchanged (resolveMonitorFocus returns null for desk slots).
- [x] Gate: `npx tsc --noEmit` passes (0 errors).
- [x] Gate: `vitest run` passes (51/51 tests across 6 files: 21 + 6 + 4 + 7 + 11 + 2).

## End-to-end hardening — verified 2026-08-03

- [x] Wire persisted `monitorSlotSources` and `agentClaims` from
      `useBoardroomSlotAssignments` through `App.tsx` into `BoardroomViewport`.
- [x] Listen for native `monitor-claim-changed` events and persist claim/release
      state in the slot-settings document.
- [x] Listen for `monitor-surface-payload` events and render the latest authorized
      payload instead of synthetic activity text.
- [x] Monitor activation calls `createMonitorSurface`; release calls the Rust API,
      dismisses the native monitor window, and clears persisted claim state.
- [x] Refresh control invokes `refresh_monitor_slot_lease` and persists the returned
      lease timestamp; it no longer resets the monitor.
- [x] Focused frontend tests pass: 26/26 across `boardroomSlotSettings.test.ts` and
      `monitorSurfaceRuntime.test.ts`.
- [x] Full frontend suite passes: 381/381 tests across 96 files.
- [x] `pnpm run build` passes.
- [x] `pnpm run lint` exits successfully with 0 errors (103 existing warnings).
- [x] `cargo test --lib` passes: 38/38.
- [x] `cargo check` passes with no warnings.
- [x] `docs/scripts/docs_health.sh` passes; 16 active plans discovered.
- [x] Native Tauri development process compiles, launches, and remains running.
- [x] Complete the operator-visible native walkthrough: claim, create/focus,
      authorized payload, refresh, reload/fallback, release/dismiss, and rejected
      unauthorized claim. Native Tauri/WebKit acceptance recorded 7/7 successful
      checks; create/focus produced a second `Arda_hud` window and release/dismiss
      returned the process to one window.
- [x] Record native defect recovery: stable `assignmentSlotId` resolution and
      active-claim rendering independent of configured focus mode both have focused
      RED/GREEN regression coverage.
- [x] Archive this checklist and its companion plan after recording the walkthrough.

## Global verification gates
Run after each slice (or at minimum after Slices A, C):
- `pnpm exec vitest run src/lib/boardroomSlotSettings.test.ts
   src/scene/boardroom/boardroomSurfacePreviewModel.test.ts
   src/scene/boardroom/boardroomSpatialLayout.test.ts
   src/components/arda/modules/SettingsModule.test.tsx`
  (baseline: 29 tests passing)
- `pnpm test -- --run` (full unit suite)
- `pnpm run build`
- `cargo test --lib --manifest-path apps/arda-hud/src-tauri/Cargo.toml`
- `cargo check --manifest-path apps/arda-hud/src-tauri/Cargo.toml`

Native interaction gates (per slice):
- inspect live boardroom with reduced-motion fallback
- activate each focused-mode surface, then dismiss and confirm return path
- claim/release/reset a monitor slot and confirm persistence through restart
- verify declined agent claims are rejected visibly

## Suggested startup order
1. **Slice A** (types + tests) — smallest change, establishes the malformed-input
   test patterns and the `remote_preview` focus mode that B2 depends on.
2. **Slice B** (claims model) — builds on Slice A types.
3. **Slice B2** (router) — depends on Slice A + B.
4. **Slice C** (Tauri bridge) — depends on B2 routing contracts.
5. **Slice D** (agent API) — depends on B + C surface for native windows.
6. **Slice E** (live example) — end-to-end proof of D + C.

#〕
