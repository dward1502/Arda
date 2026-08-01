# ARDA HUD Agentic Monitor Surfaces Plan

> **For Hermes:** This is a narrow plan slice for the arda-hud boardroom. Execute in bounded tranches, keep stable slot IDs and workspace assignments intact, and verify native runtime/build gates after each slice.

## Goal
Make the boardroom a clear dual-surface interface: **desk surfaces show stable, user-curated visualization of important ARDA state and preferences, and monitors are open agent workspaces**. The operator configures desk surfaces in Settings; agents use desk surfaces for read-only context and use monitors for live work. Which source triggered an action—dashboard, voice, or remote Hermes—is not the main distinction; the surface type is.

## Monitors vs desks
- **Monitors** (`upper_monitor`): agent live-action surfaces. Agents can claim, render, stream, expand, and release monitor surfaces. They are for doing, testing, rendering, and presenting ongoing agent work.
- **Desk surfaces** (`desk_surface`): operator-curated reference surfaces. They visualize selected ARDA state and user preferences. The user changes what appears on desk surfaces in Settings. Agents can read/pull up the related workstation for context, but they do not override desk assignments unless the operator explicitly allows it.

This keeps the live action stream on monitors while preserving familiar configuration authority on desk surfaces.

## Inspiration and bounds
- CNVS shows a useful pattern: multiple parallel agent surfaces on one canvas, each with distinct content types and focus behavior; we want the same logical interaction, not the same bundle target.
- CNVS is explicitly macOS/native Swift only; we will not adopt that bundle target or try to replicate native Mac view hierarchy.
- Arda's cross-platform boundary is Tauri + Three.js + HTML panels. That is the viable renderer and interaction surface for this pattern.

## Current baseline
- `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts` defines 7 upper monitors, 4 lower desk surfaces, 1 control core, and physical controls.
- `apps/arda-hud/src/lib/boardroomSlotSettings.ts` carries slot assignments, role profiles, preview/focus modes, widget lists, and persisted documents.
- `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx` renders HTML-in-Three previews, click activation, drag layout overrides, and fleet/status surfaces.
- `apps/arda-hud/src/lib/surfaceAdapterManifests.ts` lists declared adapters and focus contracts, but the runtime bridge is still mostly external/native-window oriented.

## Desired state
- Desk surfaces are stable, user-configurable in Settings, and shown via compact visualization/widget previews by default.
- Monitors are agent-assignable workspaces with live contracts and explicit focus behavior.
- The same Hermes action stream—whether triggered from the dashboard, voice, or remote Hermes—can land on a monitor surface when an agent claims it.
- The operator can focus a monitor surface to inspect/take over its contents without losing boardroom context.
- Sources remain bounded and parsable; the renderer never parses raw ledgers on the render thread.

## Slices

### Slice A — Agent surface contract and manifest extension
Scope: TypeScript types and defaults only. No UI or Rust backend changes.

Tasks
- Add focused types in `boardroomSlotSettings.ts` for agent-live surface bindings:
  - `agent_activity`, `streaming_text`, `iframe_preview`, `remote_session`
  - inclusive refresh contract and fallback behavior when the source is unavailable
- Add a typed agent monitor manifest shape describing owner, stream kind, fallback preview, and focus target.
- Extend `BoardroomSurfaceLayout` parsing/defaulting so new adapter kinds validate and degrade safely.
- Add dedicated unit tests for each new kind with malformed input.

Acceptance
- Tests compile and pass under vitest.
- Default slots continue to load when the new fields are absent.
- Unavailable live streams fail closed into explicit offline previews, not fake telemetry.

### Slice B — Runtime surface slots and assignment claims
Scope: Runtime claim/release model in TypeScript state; persisted assignment document updates.

Tasks
- Extend slot settings API so a claimed agent assignment can be written into the persisted document without clearing unrelated slot assignments.
- Add slot-to-source runtime resolution path that prefers the live claimed binding, then the persisted static binding, then the workspace fallback.
- Add a reset/recover action for a single monitor slot; operator settings remain authoritative.
- Preserve existing workspace-assignment behavior and slot workstation routing when no agent is claiming the slot.

Acceptance
- Operator Settings or a focused debug panel can change a monitor's source and preserve that change after reload.
- Monitors without a live binding still fall back to assigned workspace modules.
- No change to slot IDs, `assignmentSlotId` routing, or existing profile import/export behavior.

### Slice B2 — Desk/monitor focus router
Scope: Interaction routing in `BoardroomViewport`; keeps scene as the host shell.

Tasks
- Keep desk activation on explicit `activate`/`open_workstation` paths that open the detailed workstation; do not treat desk surfaces as free agent writable slots.
- Add monitor focus behavior when a monitor surface contract indicates `native_window`, `inline_embed`, or future `remote_preview`.
- Add Settings affordances for desk source/preset selection that do not disturb monitor claims.
- Add a dismissible focused-monitor preview path that returns to the shell with one action.

Acceptance
- Clicking a monitor routes to its declared focus target when available; clicking a preview-only monitor stays in-scene.
- Clicking a desk surface opens its configured detailed workstation.
- The operator can return from focused monitor mode back to the boardroom shell without restarting.
- All existing destination tests for Routing, Knowledge, Operations, Governance, Fleet, Human Realm, and Daily Command still pass.

### Slice C — Tauri surface bridge and native window entry
Scope: Rust/Tauri side for monitor surfaces only.

Tasks
- Add a window/surface-bridge command set that can create or focus a Tauri webview/window for a monitor slot surface.
- Add a scoped surface entry URL for in-shell iframe-style inspection when allowed, without changing desk behavior.
- Do not bypass security policy; inline embed stays disabled by default for both desks and monitors.
- Expose required state through existing `surfaceAdapterManifests` focus contracts.

Acceptance
- Native dev/test run can open a surface-backed window for a claimed monitor slot.
- Failure paths return explicit surface errors, not silent fallbacks to unrelated content.
- Desk surfaces remain read-only reference surfaces with no live-bridge requirement.
- The terminal and dashboard surface paths remain independent.

### Slice D — Agent-facing monitor surface API
Scope: How an agent creates/claim/update/release a monitor surface.

Tasks
- Define a narrow agent API surface limited to monitors: claim monitor slot, push a bounded surface payload, release slot, refresh lease.
- Desk surfaces are explicitly excluded from direct agent mutation; agent access is through exposed read-only contexts/workstation data.
- Use existing adapter-manifest readiness categories as gatekeepers; do not expose full system surfaces automatically to every agent.
- Add authority and tag checks so only permitted agent/provider combinations may drive operator-facing monitors.

Acceptance
- A scoped agent can claim a monitor and update its preview payload.
- Claim expires or overrides under operator authority.
- Agents cannot change desk surface assignments; they can open the assigned workstation view.
- Unpermitted agents cannot claim operator-facing monitors.

### Slice E — Live example monitor surface
Scope: One concrete monitor surface to prove the agent path.

Candidate: Hermes/Dashboard terminal stream, queue heartbeat, or agent presence feed.
- Use `agent_activity` or `streaming_text` on a monitor surface only, with bounded refresh ms.
- Render as a compact preview on the claimed monitor, and as a focusable/expandable surface when activated.
- Respect reduced-motion policies.

Acceptance
- One claimed monitor consistently shows agent activity/live text.
- Focus opens the same payload in full/focused form.
- Desk surfaces remain unchanged and user-configured.
- Stale/missing monitor state renders `NO DATA`, not fresh-looking motion.

## Settings ownership contract
- **Desk surfaces**: user owns source selection, visualization preset, density/timespan, and profile save/reset. Agents cannot override these values.
- **Monitors**: user sets default claimed/fallback assignments in Settings; agents can claim, release, and refresh live sessions against monitors while respecting those defaults.
- Conflict behavior: if an agent claims a monitor and the user edits defaults in Settings, the user action wins the next ownership boundary and the agent lease is released or downgraded.

## Verification gates
Use the existing gates and add focused tests for this slice family:
- `pnpm exec vitest run src/lib/boardroomSlotSettings.test.ts src/scene/boardroom/boardroomSurfacePreviewModel.test.ts src/scene/boardroom/boardroomSpatialLayout.test.ts src/components/arda/modules/SettingsModule.test.tsx`
- `pnpm test -- --run`
- `pnpm run build`
- `cargo test --lib --manifest-path apps/arda-hud/src-tauri/Cargo.toml`
- `cargo check --manifest-path apps/arda-hud/src-tauri/Cargo.toml`

Native interaction gates for each slice:
- inspect live boardroom with reduced-motion fallback;
- activate each focused-mode surface, then dismiss and confirm return path;
- claim/release/reset a monitor slot and confirm persistence through restart;
- verify declined agent claims are rejected visibly.

#〕
