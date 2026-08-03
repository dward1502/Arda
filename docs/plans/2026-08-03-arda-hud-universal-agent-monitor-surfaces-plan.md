# ARDA HUD Universal Agent Monitor Surfaces Implementation Plan

> **For Hermes:** Use `subagent-driven-development` to execute this plan task-by-task. This is a corrective replacement for the underscoped 2026-07-30 agentic-monitor implementation. Do not archive or describe this plan as complete until every native visual and concurrent-agent acceptance gate below passes and the operator accepts the result in the running HUD.

**Status:** Active corrective plan — prior closeout invalidated

**Goal:** Turn all five authored upper 3D monitors into independent, full-aperture display surfaces that agents can claim concurrently, render with arbitrary supported visual content, and open into full workstation windows that preserve the same live session and content.

**Architecture:** Introduce a versioned monitor-session contract shared by Rust and TypeScript, a five-slot topology with one canonical slot per physical monitor, and a renderer registry that chooses texture, media, web, terminal, document, or remote-session rendering from a typed content descriptor. The boardroom monitor and focused workstation subscribe to the same session rather than constructing different content paths. Full-screen content is rendered inside the authored 3D aperture; ownership and lease controls stay outside the content area.

**Tech stack:** Tauri 2, Rust, React, TypeScript, React Three Fiber, Three.js textures, Drei HTML only where native WebKit acceptance proves it works, HTML media elements, Tauri events/commands, Vitest, Rust unit tests, CUA/AT-SPI native acceptance.

---

## 1. Non-negotiable product contract

### 1.1 Five physical monitors means five independent slots

Number the upper monitors left-to-right as `monitor_1` through `monitor_5`.

| Physical position | Canonical slot | Required behavior |
|---|---|---|
| outer left | `monitor_1` | independently claimable and renderable |
| center left | `monitor_2` | independently claimable and renderable |
| center | `monitor_3` | independently claimable and renderable |
| center right | `monitor_4` | independently claimable and renderable |
| outer right | `monitor_5` | independently claimable and renderable |

The current topology is invalid for this requirement: it exposes five physical zones but only four assignable IDs (`monitor_left_1..monitor_left_4`), and the center monitor has no slot. The implementation must migrate this mismatch rather than preserve it.

### 1.2 The authored screen aperture is the display

Each agent surface must fill the visible screen opening of its authored 3D monitor model.

Prohibited final presentation:

- a small text rectangle floating inside the monitor;
- a generic status card centered in unused black space;
- permanent claim/lease buttons covering content;
- duplicate HTML and mesh screens with different transforms;
- a click target whose visible pixels do not align with the 3D monitor aperture.

Required presentation:

- content fills at least 95% of the usable aperture in both dimensions;
- aspect handling is explicit (`contain`, `cover`, or native responsive layout);
- clipping occurs at the physical screen boundary;
- the whole screen is the activation target;
- ownership/health indicators live on the bezel, an external rail, or the focused workstation—not over the media.

### 1.3 Supported content classes

The session contract must support these first-class content descriptors:

1. **Web surface** — an embeddable site or web application.
2. **YouTube surface** — a canonical YouTube embed descriptor, not a raw page URL.
3. **Video surface** — local workspace media, approved remote media, HLS/WebM/MP4 where the runtime supports it.
4. **Image surface** — local workspace or approved remote images.
5. **Document surface** — PDF and bounded document/markdown rendering.
6. **Terminal surface** — a named PTY/session stream with terminal-specific state.
7. **Trusted component surface** — a registered ARDA/Hermes React renderer with typed props.
8. **Remote session surface** — frame/video stream for content that cannot be embedded directly, including sites that reject framing.
9. **Fallback/status surface** — explicit unavailable/offline state only when the requested content cannot currently render.

“Arbitrary” does not mean untyped HTML injection or unbounded binary payloads. Agents choose from the typed content classes and provide a URL, media reference, trusted renderer ID, or session ID. Large media is referenced or streamed; it is never copied into a JSON event.

### 1.4 Concurrent agents

- Each monitor has one exclusive active session owner at a time.
- Different monitors may be owned by different agents simultaneously.
- One agent may own more than one monitor when slots are available.
- A claim on one monitor must not modify, rerender, release, or focus any other monitor.
- If all five are occupied, another agent receives an explicit conflict/unavailable result; it does not overwrite an existing owner.
- Operator reassignment or release always wins and produces a visible audit event.
- Lease expiry restores only that monitor’s configured fallback.

### 1.5 Click-through workstation continuity

Clicking any occupied monitor opens or focuses a workstation window bound to the same `surfaceSessionId`.

The workstation must preserve:

- owner and monitor identity;
- content descriptor and latest revision;
- URL/navigation target or terminal session;
- media playback position and paused/playing state where technically supported;
- remote-session identity;
- document page/scroll state where supported;
- subsequent live updates and release/expiry events.

The workstation is not allowed to fall back to a generic source-zone panel when an active monitor session exists.

---

## 2. Verified current-state defects

The corrective implementation begins from these source-backed facts:

1. `boardroomSpatialLayout.ts` defines five `upper_monitor` zones, but `boardroomSlotSettings.ts` defines only four monitor slots.
2. `boardroom.monitor.center` has no `assignmentSlotId` and duplicates the `upper_monitor_3` binding used by center-right.
3. `HermesDashboardMonitorSurface` in `BoardroomViewport.tsx` renders a fixed `13.2rem` text/status card, not the requested display surface.
4. `MonitorSurfacePayloadEvent` carries only `{ content: string, mime: string }`; it has no typed web, video, image, document, terminal, component, playback, or remote-session contract.
5. Reduced-motion currently replaces valid payload content with `NO DATA`; motion preference must disable animation, not content.
6. `create_monitor_surface` routes Hermes to one external dashboard and other sources to a generic ARDA panel URL. It does not reopen the same monitor content session.
7. The claim registry can isolate claims by string slot ID, but the public gate accepts any `monitor_*` string rather than the canonical five-slot set.
8. The existing acceptance harness proves one text payload on one monitor. It does not prove five slots, arbitrary content, full-aperture rendering, workstation continuity, or concurrent agents.
9. The archived 2026-07-30 plan was therefore closed against an underscoped acceptance target and is not product-complete.

---

## 3. Target contracts

### 3.1 Canonical topology

Create a single topology authority used by persistence, rendering, native commands, and tests:

```ts
export const UPPER_MONITOR_SLOT_IDS = [
  'monitor_1',
  'monitor_2',
  'monitor_3',
  'monitor_4',
  'monitor_5',
] as const

export type UpperMonitorSlotId = typeof UPPER_MONITOR_SLOT_IDS[number]
```

Migration from the current schema:

```text
monitor_left_1 -> monitor_1
monitor_left_2 -> monitor_2
new center slot -> monitor_3
monitor_left_3 -> monitor_4
monitor_left_4 -> monitor_5
```

The parser may accept old IDs only as an import/migration boundary. Runtime APIs, events, tests, and new durable writes use canonical IDs.

### 3.2 Content descriptor

Create a discriminated union similar to:

```ts
export type MonitorContentDescriptor =
  | { kind: 'web'; url: string; title?: string; display: 'inline' | 'capture_stream'; sandboxProfile: string }
  | { kind: 'youtube'; videoId: string; startSeconds?: number; autoplay?: boolean; muted?: boolean }
  | { kind: 'video'; source: MonitorMediaSource; mime?: string; fit: 'contain' | 'cover'; loop?: boolean; autoplay?: boolean; muted?: boolean }
  | { kind: 'image'; source: MonitorMediaSource; fit: 'contain' | 'cover'; alt?: string }
  | { kind: 'document'; source: MonitorMediaSource; documentKind: 'pdf' | 'markdown' | 'text'; page?: number }
  | { kind: 'terminal'; sessionId: string; readOnly?: boolean; theme?: string }
  | { kind: 'component'; rendererId: string; props: Record<string, unknown> }
  | { kind: 'remote_session'; sessionId: string; streamUrl: string; transport: 'webrtc' | 'hls' | 'mjpeg' }
  | { kind: 'fallback'; reason: string; retryable: boolean }
```

Create one canonical session envelope:

```ts
export interface MonitorSurfaceSession {
  schemaVersion: 'arda.monitor-surface-session.v2'
  surfaceSessionId: string
  slotId: UpperMonitorSlotId
  owner: AgentSurfaceOwner
  content: MonitorContentDescriptor
  revision: number
  leaseExpiresAtUtc: string
  createdAtUtc: string
  updatedAtUtc: string
  playback?: MonitorPlaybackState
  navigation?: MonitorNavigationState
}
```

Rust request/result/event structures must serialize the same camelCase wire shape. Add fixture parity tests so the two sides cannot drift silently.

### 3.3 Agent API

Replace the text-only mental model with these operations:

- `claim_monitor_surface(slotId, owner, initialContent, ttl)`
- `set_monitor_surface_content(surfaceSessionId, owner, content, expectedRevision)`
- `patch_monitor_surface_state(surfaceSessionId, owner, playback/navigation patch, expectedRevision)`
- `refresh_monitor_surface_lease(surfaceSessionId, owner)`
- `release_monitor_surface(surfaceSessionId, owner)`
- `list_monitor_surfaces()`
- `get_monitor_surface(surfaceSessionId)`

Every mutation returns the complete new session envelope and revision. Stale revisions fail explicitly instead of racing.

### 3.4 Renderer registry

Use one renderer registry, not conditionals scattered through `BoardroomViewport.tsx` and workstation modules:

```ts
interface MonitorRendererDefinition<K extends MonitorContentDescriptor['kind']> {
  kind: K
  renderAperture: React.ComponentType<MonitorRendererProps<K>>
  renderWorkstation: React.ComponentType<MonitorRendererProps<K>>
  validate: (content: Extract<MonitorContentDescriptor, { kind: K }>) => ValidationResult
}
```

The boardroom and workstation resolve the same descriptor through this registry. Renderer-specific behavior stays in dedicated files.

---

## 4. Implementation phases

## Phase 0 — Correct the record and establish RED gates

**Objective:** Make it impossible to mistake the existing text card for completion.

**Files:**

- Modify: `docs/archive/2026-07-30-arda-hud-agentic-monitor-surfaces-plan.md`
- Modify: `docs/archive/2026-07-30-arda-hud-agentic-monitor-surfaces-checklist.md`
- Modify: `docs/archive/README.md`
- Create: `apps/arda-hud/src/scene/boardroom/universalMonitorAcceptance.test.ts`

**Tasks:**

1. Mark the previous closeout as invalidated by underscoped acceptance.
2. Link the archived record to this active corrective plan.
3. Add failing contract tests asserting:
   - five canonical assignable monitor slots;
   - every physical upper monitor maps 1:1 to a canonical slot;
   - the center monitor is assignable;
   - every content kind is represented in the union/validator;
   - multiple slots can hold different owners simultaneously;
   - an active session opens by `surfaceSessionId`, not source-zone fallback.
4. Run the focused test and record the expected failures before implementation.

**Gate:** This phase is complete only when failures describe the actual topology, renderer, and handoff gaps—not missing test setup.

## Phase 1 — Five-slot topology and persisted-state migration

**Objective:** Give all five authored monitors stable, independent runtime and persistence identities.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/monitorSurfaceTopology.ts`
- Create: `apps/arda-hud/src/scene/boardroom/monitorSurfaceTopology.test.ts`
- Modify: `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts`
- Modify: `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.test.ts`
- Modify: `apps/arda-hud/src/lib/boardroomSlotSettings.ts`
- Modify: `apps/arda-hud/src/lib/boardroomSlotSettings.test.ts`
- Modify: `core/state/arda_boardroom_slots.json` only through the tested migration path; preserve operator desk changes.

**Tasks:**

1. Write RED tests for five canonical slots and exact left-to-right mappings.
2. Define the canonical topology and aperture metadata for each monitor.
3. Give the center monitor its own slot and unique physical binding.
4. Add schema-v1-to-v2 migration preserving four existing assignments and claims while creating the center fallback.
5. Reject unknown runtime IDs after migration.
6. Verify import/export/reload round trips without changing desk assignments.

**Gate:** Five physical monitors, five canonical slots, five unique fallback assignments, no duplicated center binding, and no desk-state mutation.

## Phase 2 — Versioned multi-agent session registry

**Objective:** Replace string payload delivery with durable, revisioned monitor sessions.

**Files:**

- Create: `apps/arda-hud/src/lib/monitorSurfaceContract.ts`
- Create: `apps/arda-hud/src/lib/monitorSurfaceContract.test.ts`
- Create: `apps/arda-hud/src-tauri/src/commands/monitor_surface/contract.rs`
- Create: `apps/arda-hud/src-tauri/src/commands/monitor_surface/registry.rs`
- Create: `apps/arda-hud/src-tauri/src/commands/monitor_surface/tests.rs`
- Replace module body: `apps/arda-hud/src-tauri/src/commands/monitor_surface.rs` with `commands/monitor_surface/mod.rs` after parity tests exist
- Modify: `apps/arda-hud/src-tauri/src/commands/mod.rs`
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Modify: `apps/arda-hud/src/components/arda/hooks/useBoardroomSlotAssignments.ts`

**Tasks:**

1. Define TypeScript descriptors and validators.
2. Mirror the wire contract in Rust and add shared JSON fixture tests.
3. Store sessions by both canonical slot and `surfaceSessionId`.
4. Enforce one active session per slot while allowing five different owners concurrently.
5. Add revision checks to content/state updates.
6. Persist enough session metadata for reload recovery; do not persist large media bytes.
7. Emit complete session snapshots on claim/update/release/expiry.
8. Rehydrate all active sessions after main-window reload without claim loops.
9. Remove prefix-only slot acceptance and validate the canonical five-slot set.

**Gate:** Rust tests prove five simultaneous owners, isolated updates, conflict rejection, exact-owner release, revision conflict rejection, expiry isolation, and reload reconstruction.

## Phase 3 — Full-aperture renderer foundation

**Objective:** Replace the text rectangle with a renderer host fitted to the actual 3D screen opening.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/MonitorApertureSurface.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/MonitorApertureSurface.test.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/monitorApertureProjection.ts`
- Create: `apps/arda-hud/src/scene/boardroom/monitorApertureProjection.test.ts`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/registry.ts`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/FallbackMonitorRenderer.tsx`
- Modify: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- Modify: `apps/arda-hud/src/styles/scene/hud-instruments.css`
- Modify or create a focused stylesheet under `apps/arda-hud/src/styles/scene/` for full-aperture surfaces

**Tasks:**

1. Write RED tests that reject the fixed `13.2rem` card contract and require aperture dimensions from topology.
2. Put monitor content and the authored monitor housing under one transform authority.
3. Define the usable local aperture and clip content to it.
4. Make the full aperture the pointer target.
5. Move lease/release controls outside the content pixels.
6. Remove `HermesDashboardMonitorSurface` from the production monitor path.
7. Ensure reduced-motion disables animation only; it must never replace valid content.
8. Add debug-only aperture outlines for calibration, disabled in normal HUD.

**Gate:** Native and browser captures show a test pattern filling every monitor aperture with no small card, no overflow, and no duplicate visible screen.

## Phase 4 — Image, video, and generated-frame renderers

**Objective:** Prove real visual media on all five 3D monitors using GPU-backed planes where possible.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/renderers/ImageMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/VideoMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/CanvasMonitorRenderer.tsx`
- Create corresponding focused tests beside each renderer
- Modify: `apps/arda-hud/src/lib/weathertop.ts` only if a bounded media URL resolver is absent
- Modify: `apps/arda-hud/src-tauri/src/lib.rs` or create a scoped media command/protocol module for safe workspace media URLs

**Tasks:**

1. Render images with `contain` and `cover` behavior on the actual screen plane.
2. Render video using a shared HTML video element plus `THREE.VideoTexture` or an equivalent native-proven texture path.
3. Add playback-state synchronization to the session store.
4. Support generated canvas/frame content for trusted visualizations and remote previews.
5. Enforce bounded local paths and approved remote schemes.
6. Dispose textures, media elements, and subscriptions when content changes or a lease ends.

**Gate:** Five monitors concurrently show distinct visual content, including at least two playing videos or frame streams, without cross-slot replacement or leaked audio.

## Phase 5 — Web, YouTube, document, and terminal renderers

**Objective:** Support the non-text content specifically required by the operator.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/renderers/WebMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/YouTubeMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/DocumentMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/TerminalMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/RemoteSessionMonitorRenderer.tsx`
- Create corresponding focused tests
- Reuse/refactor from: `src/components/arda/modules/HermesDashboardModule.tsx`
- Reuse/refactor from: `src/components/arda/modules/MediaLibraryModule.tsx`
- Reuse/refactor from: `src/components/arda/modules/ServiceEmbedModule.tsx`
- Reuse the existing PTY/session path rather than creating a second terminal backend

**Tasks:**

1. Implement inline web rendering only for approved embeddable origins.
2. Implement YouTube through the canonical embed/player API with state synchronization.
3. Implement PDF/markdown/text display through bounded document adapters.
4. Implement terminal display by subscribing to an existing named terminal session.
5. Implement remote-session/frame-stream rendering for sites that reject iframe embedding.
6. Detect frame/CSP failure and switch to the declared capture-stream adapter; do not show a fake live page or silently open unrelated content.
7. Keep audio muted by default in the boardroom; focused workstations may expose explicit audio controls.

**Gate:** Native HUD acceptance demonstrates a website, YouTube playback, local/approved video, image, document, and terminal across separate monitors. A known frame-blocked website must render through the remote-session/capture path or report a precise unsupported state; it cannot be credited from an external browser opening.

## Phase 6 — Workstation window bound to the same session

**Objective:** Clicking a monitor opens a full workstation containing the exact same content/session.

**Files:**

- Create: `apps/arda-hud/src/components/arda/MonitorSurfaceWorkstation.tsx`
- Create: `apps/arda-hud/src/components/arda/MonitorSurfaceWorkstation.test.tsx`
- Modify: `apps/arda-hud/src/App.tsx`
- Modify: `apps/arda-hud/src/utils/multiWindow.ts`
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Modify: `apps/arda-hud/src/scene/boardroom/monitorSurfaceRuntime.ts`

**Tasks:**

1. Add a dedicated route such as `__view=monitor&__surfaceSession=<id>`.
2. Change monitor activation to pass only the canonical session identity plus window metadata.
3. Load the session from the registry in the focused window.
4. Render through the same renderer registry in workstation mode.
5. Synchronize content revisions and playback/navigation state between boardroom and workstation.
6. Focus an existing session window rather than spawning duplicates.
7. On release or expiry, close or visibly detach the focused window according to operator policy.
8. Preserve fallback workstation routing only for unclaimed monitors.

**Gate:** For every content class, clicking the physical monitor opens the same content. Changing content after the workstation opens updates both surfaces. Video/YouTube/terminal identity is preserved rather than restarted as an unrelated source panel.

## Phase 7 — Concurrent-agent orchestration and operator control

**Objective:** Prove the system behaves correctly under real parallel work.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/MonitorOwnershipRail.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/monitorConcurrency.test.ts`
- Modify: the monitor session hook/store and Rust registry from earlier phases
- Modify: Settings only for monitor fallback/default and operator release controls; do not alter desk ownership

**Tasks:**

1. Run five simultaneous sessions with five owners.
2. Update all five at independent rates.
3. Release the middle session and prove the other four continue unchanged.
4. Reclaim the released slot with a sixth agent.
5. Expire one lease and prove only that monitor restores fallback.
6. Add a compact external ownership rail or bezel indicator; keep content unobstructed.
7. Add operator focus/release/reassign controls with audit receipts.
8. Verify restart recovery for all still-valid sessions.

**Gate:** The native HUD visibly supports five concurrent agents with independent content and lifecycle behavior. No singleton “active monitor” state is allowed anywhere in the path.

## Phase 8 — Performance, security, and failure hardening

**Objective:** Make five live surfaces stable enough for routine use.

**Files:**

- Add focused tests to each renderer and registry module
- Modify Tauri capabilities/scopes only as required by the exact media and URL contracts
- Add performance instrumentation under existing boardroom render-profile tooling

**Tasks:**

1. Bound update rates and payload descriptor size.
2. Keep binary media out of Tauri JSON events.
3. Validate URL schemes, local path scopes, trusted component IDs, and iframe sandbox profiles.
4. Dispose resources on content replacement, release, expiry, window close, and app shutdown.
5. Pause or throttle hidden video/frame renderers without losing session state.
6. Respect reduced-motion, but continue rendering static frames and valid content.
7. Test malformed descriptors, unreachable URLs, unsupported codecs, blocked embeds, expired sessions, and renderer crashes.
8. Ensure one renderer failure cannot blank the boardroom or other monitors.

**Performance gate:** At the native target resolution, five occupied monitors remain responsive to pointer activation and workstation opening. Record frame timing, memory before/after repeated claim cycles, and media resource cleanup evidence. Do not invent an FPS number in advance; measure and document the accepted baseline.

## Phase 9 — Native visual acceptance and documentation closeout

**Objective:** Prove the product requirement in the real Tauri/WebKit HUD and only then update completion records.

**Files:**

- Replace the one-slot text acceptance harness with a five-slot scenario harness under a dev-only gate
- Modify: `apps/arda-hud/src/scene/boardroom/README.md`
- Modify: `apps/arda-hud/README.md`
- Modify: `apps/arda-hud/BREAKDOWN.md` if present
- Modify: `docs/archive/README.md` only after acceptance
- Keep this plan in `docs/plans/` until operator acceptance

**Required native walkthrough:**

1. Launch the normal native HUD and keep it available for operator inspection.
2. Claim all five monitors with at least three different agent owners.
3. Display, simultaneously:
   - monitor 1: website/web application;
   - monitor 2: YouTube or network video;
   - monitor 3: local/approved video or generated visual stream;
   - monitor 4: image or document;
   - monitor 5: terminal or trusted agent UI.
4. Capture the native boardroom and verify every display fills its authored aperture.
5. Click each of the five screens and verify a workstation opens with the same session/content.
6. Update content while focused and verify boardroom/workstation synchronization.
7. Release one monitor and prove only that monitor returns to fallback.
8. Reassign it to another agent while the remaining sessions continue.
9. Reload/restart and verify valid sessions recover.
10. Exercise a blocked embed and prove the remote-session/capture or explicit unsupported path.
11. Leave the normal HUD running for operator inspection.
12. Obtain operator visual acceptance before moving this plan to the archive.

**No-closeout rule:** Unit tests, build success, Rust tests, one claimed monitor, a text payload, a generic child window, or an acceptance panel reporting green are not sufficient to close this plan.

---

## 5. Verification commands

Run focused RED/GREEN tests after each phase, then the full gates:

```bash
cd /var/home/mythos/Eregion/Arda/apps/arda-hud
pnpm exec vitest run src/scene/boardroom src/lib/monitorSurfaceContract.test.ts src/components/arda/MonitorSurfaceWorkstation.test.tsx
pnpm test
pnpm run lint
pnpm run build

cd /var/home/mythos/Eregion/Arda
cargo test --lib --manifest-path apps/arda-hud/src-tauri/Cargo.toml
cargo check --manifest-path apps/arda-hud/src-tauri/Cargo.toml
bash docs/scripts/docs_health.sh
git diff --check
```

Native acceptance must additionally use the real Tauri application and fresh CUA/AT-SPI captures. Browser/Vite rendering is diagnostic evidence only.

---

## 6. Completion checklist

This plan remains active until every statement is true:

- [ ] Five physical upper monitors map to five canonical assignable slots.
- [ ] The center monitor is independently claimable.
- [ ] Each occupied screen fills its authored 3D aperture; no small card remains.
- [ ] Website rendering is demonstrated natively.
- [ ] YouTube rendering is demonstrated natively.
- [ ] Video rendering is demonstrated natively.
- [ ] Image rendering is demonstrated natively.
- [ ] Document rendering is demonstrated natively.
- [ ] Terminal or trusted custom-agent UI rendering is demonstrated natively.
- [ ] Frame-blocked web content has a working capture/remote-session path or an explicit unsupported result that is not falsely credited.
- [ ] At least five concurrent sessions with multiple owners are demonstrated.
- [ ] Updates, release, expiry, and reassignment are isolated per monitor.
- [ ] Clicking every monitor opens the same session in a full workstation window.
- [ ] Playback/navigation/session state remains synchronized where supported.
- [ ] Reload/restart recovers unexpired sessions.
- [ ] Desk surfaces remain operator-owned and unchanged.
- [ ] Focused/full HUD tests, lint, build, Rust tests, Cargo check, docs health, and diff check pass.
- [ ] The normal native HUD is left running for operator inspection.
- [ ] The operator explicitly accepts the visual and interaction result.

Only after all items pass may this plan be archived and described as complete.
