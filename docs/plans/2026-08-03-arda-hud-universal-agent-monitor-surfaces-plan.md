# ARDA HUD Universal Agent Monitor Surfaces Implementation Plan

> **For Hermes:** Use `subagent-driven-development` to execute this plan task-by-task. This is a corrective replacement for the underscoped 2026-07-30 agentic-monitor implementation. Do not archive or describe this plan as complete until every native visual and concurrent-agent acceptance gate below passes and the operator accepts the result in the running HUD.

**Status:** Active corrective plan — implementation substantially converged; native multi-content walkthrough, hardening evidence, and operator acceptance remain open

**Goal:** Turn all five authored upper 3D monitors into independent, full-aperture display surfaces that agents can claim concurrently, render with arbitrary supported visual content, and open into full workstation windows that preserve the same live session and content.

## Current implementation evidence

The physical-display foundation is now integrated across the five upper monitors, five lower desk apertures, and command core. Boardroom display pixels render through CanvasTexture-backed WebGL planes under the authored transforms; the prior native-safe-but-screen-space DOM cards and HTML control overlays are no longer the production boardroom path. The exported stage, runtime lower-desk rig quaternions, nested surface-fit transforms, and widened camera composition are aligned.

On 2026-08-07, the running native Tauri HUD visibly showed all five upper and all five lower displays fitted inside their authored apertures, with the complete outer console in frame. The focused boardroom suite passed 96/96 tests, the production frontend build passed, and lint completed with zero errors (105 pre-existing repository warnings). Reduced-motion mode now keeps valid payload content visible and marks it static instead of replacing it with `NO DATA` or an offline state.

That run closed only the physical CanvasTexture integration slice; pointer interaction was not accepted because the desktop-action approval timed out. Later evidence below supersedes the then-open typed registry and same-session workstation items.

On 2026-08-07, Phase 2 contract parity advanced: `apps/arda-hud/src/lib/monitorSurfaceContract.ts` now defines the canonical five-slot set, typed content/workstation-handoff descriptors, and claim/refresh/release/active registry helpers; `apps/arda-hud/src/lib/monitorSurfaceContract.test.ts` exercises parsing, five-slot claim/refresh/release, and canonical-slot enforcement; Rust `contract`/`registry` tests prove five simultaneous owners, isolated updates, conflict rejection, exact-owner release, revision conflict rejection, expiry isolation, snapshot/restore, and stale-schema rejection. Focused vitest + Rust `cargo test --lib` gates are green after these changes.

On 2026-08-07, Phase 6 implementation and its bounded native lifecycle gate completed. A monitor session now opens a deterministic native workstation keyed by `surface_session_id`; repeated opens focus the existing window instead of creating a duplicate. The workstation resolves the exact authoritative registry record, renders through the same `BoardroomApertureSurface`, receives revision/lifecycle updates through the Tauri registry event plus a cross-window persisted-registry fallback, and changes to an explicit unavailable state after release or expiry. Native Tauri evidence showed the same README document in `monitor_1` and its workstation, a revision refresh from the same session, two repeated opens with only one workstation window, and the already-open workstation transitioning to `Monitor session unavailable` after exact-session release. The all-content-class walkthrough remains part of Phase 9 rather than being inferred from this document-only lifecycle proof.

On 2026-08-10, the monitor convergence slice closed the remaining machine-verifiable registry gaps. The TypeScript persistence gate now round-trips all five canonical sessions without collapsing slot, owner, revision, descriptor, or workstation identity. Barrier-synchronized Rust tests prove competing same-slot claims serialize to exactly one owner and a release/reclaim race leaves zero or one valid replacement, never duplicate or corrupted ownership. The canonical `arda.operator-projection.v1` is now published by `arda-engine`, consumed by the HUD, and registered as a read-only trusted component renderer through the existing typed monitor session and same-session workstation path. Focused HUD convergence tests passed 46/46, focused Tauri monitor-surface tests passed 34/34, the complete HUD suite passed 483/483, TypeScript and the production build passed, the complete `arda-engine` suite passed, and strict `arda-engine` library Clippy passed. A controlled native Tauri restart preserved the accepted five-upper/five-lower composition. These results do not substitute for the still-open five-content native walkthrough or explicit operator acceptance.

### 2026-08-10 live browser-stream corrective slice

**First new-session rule:** no visual substitute may satisfy a runtime capability gate. Screenshots, downloaded video, static browser frames, acceptance-panel labels, and placeholders are never evidence for browser, YouTube, terminal, navigation, input, or same-session continuity. A failed stream remains an explicit failed stream.

Live audit found a typed, revisioned five-slot registry, same-session workstation identity, CanvasTexture aperture rendering, and HLS/MJPEG descriptor support, but no browser process launcher, capture producer, MJPEG/WebRTC server, browser navigation/input authority, or capture lifecycle manager. The renderer registry therefore correctly reports that aperture web and YouTube content require a capture stream; the prior showcase bypassed that gate with a downloaded trailer, a browser screenshot, and an ASCII screenshot. Those temporary controls and media were removed. The later Phase 4-only claim controls, claim generator, and two local WebM fixtures were also removed after reference tracing proved that they were acceptance-only and no longer had a production consumer; the legitimate media renderer, mute, and disposal fixes and their focused tests remain. No HUD, Tauri, Vite, CDP, or capture process remained after cleanup.

The smallest complete vertical slice is bounded to one real browser session before navigation, input, or multi-browser expansion:

1. Add RED Rust tests for browser launch planning, loopback-only stream URLs, unique per-session profile/ports, forced audio mute, owner/session lifecycle isolation, changing-frame publication, and honest startup/stream failure.
2. Add an owned native browser-capture state that launches one browser process with an isolated profile, consumes its continuous DevTools screencast, and publishes a loopback MJPEG stream. Keep frame bytes out of Tauri JSON and monitor persistence.
3. Add start/status/stop Tauri commands. Starting returns the real `remote_session` identity and stream URL only after a changing frame stream is available; failure returns an error and never synthesizes media.
4. Reuse the existing `remote_session` renderer and typed monitor claim path so the physical CanvasTexture aperture and same-session workstation consume the exact stream/session identity.
5. Prove focused tests and one native browser stream with visibly changing frames, process ownership, mute state, release cleanup, and workstation continuity. Only then extend the same manager with owner/revision-guarded navigation and input, a second isolated browser, YouTube, and the live ARDA terminal/generated-frame session.

**Implementation evidence — 2026-08-10:** The RED-GREEN backend slice now launches an isolated direct/Flatpak Brave runtime, forces mute, waits for two real CDP screencast revisions, publishes loopback MJPEG, exposes owner-checked start/status/stop commands, tears down the process/profile, and reports failure rather than substituting imagery. The ignored installed-browser gate passed with one real process and changing revisions (`1 passed`, `2.33s`). The first failure was traced to Chromium keeping `/json/list` alive; the client now completes the response at its declared `Content-Length` instead of waiting for EOF. The aperture MJPEG path now redraws the live image into its `CanvasTexture` every animation frame and closes the stream on disposal. A frontend orchestration boundary starts capture first, rejects unmuted or fewer-than-two-frame descriptors, claims the typed `remote_session` only after that gate, and stops capture if the authoritative claim fails. The frontend suite passes `496` tests and the production TypeScript/Vite build passes. Native visual observation remains open, so this is implementation evidence—not acceptance.

**Navigation/input expansion — 2026-08-11:** The same capture manager now exposes Tauri-native navigation and pointer-click commands. Every mutation requires the exact session owner and current control revision; a successful mutation advances that revision, so stale or cross-owner commands fail before CDP input is sent. Navigation accepts only HTTP(S), and pointer coordinates must be finite and inside the captured viewport. Frontend wrappers carry the current descriptor identity/revision rather than reconstructing authority. Focused Rust and TypeScript tests cover command shape and stale/wrong-owner rejection, while the installed-browser gate exercised real `Page.navigate` and `Input.dispatchMouseEvent` calls against the same isolated process before proving changing-frame delivery and process/profile cleanup.

Transport choice for this slice is loopback MJPEG fed by Chromium DevTools `Page.startScreencast`: the current renderer already consumes MJPEG as an image texture, current CSP allows loopback HTTP, and the installed Brave Flatpak exposes a Chromium CDP endpoint. WebRTC would add an unrelated signaling/media stack before one honest stream exists. The capture manager remains transport-bounded so WebRTC can be added later without changing monitor ownership contracts.

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

The original topology exposed five physical zones but only four assignable IDs
(`monitor_left_1..monitor_left_4`), leaving the center without a slot. That
migration is now implemented: runtime and durable writes use the canonical five,
while old IDs remain import-boundary aliases only.

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
7. **Trusted component surface** — a registered ARDA React renderer with typed props.
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

## 2. Verified pre-correction baseline defects

The corrective implementation began from these source-backed facts. Items 1–8 are retained as the historical RED baseline, not as claims about the current tree. Phases 0–7 below record their implemented corrections; the remaining product gap is native all-content/concurrency acceptance plus Phase 8 hardening evidence.

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

**Status — complete (2026-08-11):** The archived 2026-07-30 plan and checklist identify the underscoped closeout and link to this corrective authority. `universalMonitorAcceptance.test.ts` establishes the five-slot, content-kind, multi-owner, session-handoff, and unique physical-binding contract gates. The stale duplicate-center expectation was corrected before the Phase 1 binding repair, producing the intended RED failure and then passing against the repaired topology.

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

**Status — complete (2026-08-11):** Runtime and durable writes use `monitor_1..monitor_5`; old IDs are migration aliases only. The center monitor has its own canonical assignment, canonical-slot validation rejects unknown runtime IDs, and slot-setting round-trip tests preserve lower-desk ownership and configuration. The five authored zones now bind uniquely and monotonically to `upper_monitor_1..upper_monitor_5`, with matching assignment indices `0..4`. The corrected RED tests failed against the duplicate `upper_monitor_3`/shifted-right baseline, then the four focused topology/model suites passed `29/29` after the production repair.

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

**Status — machine-verifiable implementation complete:** TypeScript and Rust share the v2 registry shape, the Tauri commands emit/restore complete records, and active sessions rehydrate from persisted state. The 2026-08-10 additions prove all-five local persistence plus barrier-synchronized claim and release/reclaim races. Native restart/operator acceptance remains in Phase 9 rather than reopening this contract phase.

## Phase 3 — Full-aperture renderer foundation

**Objective:** Replace the text rectangle with a renderer host fitted to the actual 3D screen opening.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/monitorApertureGeometry.ts`
- Create: `apps/arda-hud/src/scene/boardroom/monitorApertureProjection.test.ts`
- Modify: `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- Modify: `apps/arda-hud/src/styles/scene/hud-instruments.css`

**Tasks:**

1. Write RED tests that reject the fixed `13.2rem` card contract and require aperture dimensions from topology.
2. Put monitor content and the authored monitor housing under one transform authority.
3. Define the usable local aperture and clip content to it.
4. Make the full aperture the pointer target.
5. Move lease/release controls outside the content pixels.
6. Remove `HermesDashboardMonitorSurface` from the production monitor path.
7. Ensure reduced-motion disables animation only; it must never replace valid content.
8. Add debug-only aperture outlines for calibration, disabled in normal HUD.

**Implemented:**

- Aperture geometry now derives dimensions from slot topology/rotation instead of a fixed card size.
- `monitorApertureProjection.test.ts` validates monitor-surface and desk-surface apertures.
- `BoardroomApertureSurface` renders the full-aperture CanvasTexture-backed surface and is wired into `BoardroomViewport.tsx` for active claims.
- Legacy `HermesDashboardMonitorSurface` removed from the production monitor path; active claims no longer route through it.
- Debug-only aperture outlines render behind the content surface when `debug` is enabled.
- `hud-instruments.css` fixed `13.2rem` monitor-surface card block removed; aperture sizing is topology-driven.

**Gate — complete:** Vitest + build + lint + Cargo checks passed, and the 2026-08-07 native Tauri run verified all five occupied upper surfaces fitted within the authored apertures with the full console composition visible.

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

**Status — implementation complete; native gate open:** `BoardroomApertureSurface` renders local or HTTP(S) images and video, MJPEG, HLS-backed streams, and owner-scoped `generated_frame` content into its CanvasTexture with explicit `contain`/`cover`. Local files resolve through Tauri's `protocol-asset` feature with the canonical workspace as the only configured asset scope; traversal and non-HTTP(S) remote schemes fail closed. Playback state now survives the TypeScript/Rust registry boundary and is mutated through an owner-, session-identity-, and revision-guarded Tauri command; boardroom video remains muted. The media lifecycle tracker disposes image callbacks, video sources, and animation frames idempotently, and its focused test returns every active counter to zero after 25 replacement cycles. The development-only native acceptance harness can claim five distinct deterministic visual sessions, including two local autoplay WebM streams, patch their authoritative playback state, and record exact active image/video/frame and unmuted-video counts. Phase 4 remains open only because no qualifying native operator receipt yet proves the five-aperture/two-stream gate on the authored scene; harness availability and automated lifecycle evidence are not substituted for that receipt.

## Phase 5 — Web, YouTube, document, and terminal renderers

**Objective:** Support the non-text content specifically required by the operator.

**Files:**

- Create: `apps/arda-hud/src/scene/boardroom/renderers/WebMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/YouTubeMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/DocumentMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/TerminalMonitorRenderer.tsx`
- Create: `apps/arda-hud/src/scene/boardroom/renderers/RemoteSessionMonitorRenderer.tsx`
- Create corresponding focused tests
- Extract only still-valid generic embed behavior from
  `src/components/arda/modules/HermesDashboardModule.tsx`; do not preserve its
  legacy ownership/name or localhost launcher as monitor architecture
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

**Status — contract and bounded adapters implemented; native multi-content gate open:** The renderer registry covers all nine descriptor kinds, rejects unsafe paths/schemes, limits sandbox profiles and trusted component IDs, requires capture streams for web/YouTube aperture pixels, and reports unsupported WebRTC honestly. Markdown/text documents render through the CanvasTexture path; PDF and unavailable terminal sessions report explicit limitations; HLS/MJPEG provide declared capture adapters; `operator_projection` is the first registered trusted component. On 2026-08-11, one real muted Chromium/CDP session rendered visibly changing public Three.js browser pixels inside physical Monitor 1 through the exact typed `remote_session`; native WebKit's multipart-image limitation was handled by revisioned Tauri IPC frame delivery into the same CanvasTexture. The manager now also performs owner/revision-guarded HTTP(S) navigation and pointer clicks against that exact browser process, with live installed-browser coverage. This proves the single website/capture/control vertical slice only. YouTube, PDF, named PTY, second-browser concurrency, and the simultaneous five-content walkthrough remain open.

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

**Status — session lifecycle complete; all-content gate open:** Stable window identity, exact-registry lookup, duplicate-window focus, cross-window revision updates, persisted-registry fallback, and release/expiry detachment are implemented and were exercised natively with one markdown document session. Per-content playback/navigation continuity remains open with the Phase 4/5 renderer work and Phase 9 walkthrough.

## Visual reset checkpoint — agent-native command instruments

**Objective:** Establish the visual language before Phase 7 adds more orchestration UI. The upper bank remains five practical agent canvases. The five lower desk apertures are ambient machine instruments, not web dashboards or miniature workstations.

**Non-negotiable lower-instrument contract:**

1. Normal desk presentation is nearly textless: no prose, cards, lists, scrollbars, panel titles, freshness labels, or module dumps.
2. Each lower aperture has a unique learned motion language derived from cyberpunk command instrumentation, 1980s phosphor/vector systems, ASCII signal fields, radar, waveforms, scanning geometry, and controlled interference.
3. Motion is performative but not arbitrary. Real runtime state modulates amplitude, cadence, coherence, density, route paths, distortion, and color. Reduced-motion keeps a meaningful static frame.
4. The five roles are Governance/Decisions, Systems/Fleet, fixed Command Core/Now, Routing/Communications, and Human/Business. They must not reuse one generic graph with different labels.
5. Detailed text is progressive disclosure in a deliberately opened workstation. The physical desk remains glanceable, soothing, and legible through repeated exposure.
6. Unclaimed upper monitors are visually quiet. Claimed upper monitors display the agent's real content rather than departmental fallback charts.

**Prototype gate — accepted and propagated (2026-08-08):** The normal native Tauri HUD now shows the accepted fixed-center Command Core with no visible words, no generic dot graph, continuous motion, state-derived behavior, and correct aperture fit. After operator acceptance, the remaining four lower apertures received distinct—not recolored—signal languages:

- Governance/Decisions: converging decision lattice, quorum diamond, chosen-path pulses, and pressure arcs.
- Systems/Fleet: segmented reactor spine, subsystem columns, health pulses, and degraded-state interference.
- Routing/Communications: orbital radar field, curved routes, moving packets, endpoints, and sweep beam.
- Human/Business: breathing organic contours, paired living wave strands, and orbiting presence points.

All five render as CanvasTexture-backed WebGL meshes in the normal native HUD. `LowerInstrumentScreen.tsx` contains no title/card/list rendering; `lowerInstrumentSignal.ts` derives bounded activity, pressure, coherence, cadence, palette, and deterministic motion from each surface's current `HudInstrumentModel`. Reduced-motion and deterministic render profiles produce meaningful static frames. Focused Vitest coverage validates stable physical-role mapping, visual-language uniqueness, bounded state projection, degradation behavior, and deterministic sample output. Native captures confirm all five distinct instruments render simultaneously in their authored apertures; existing click activation remains on each R3F mesh.

**Unused upper-monitor ambient state (2026-08-08):** Each canonical upper slot now owns one distinct, nearly textless CanvasTexture ambient identity: aurora veil, constellation mesh, signal mandala, vector rain, or dream horizon. These are idle pixels only. The production render branch resolves priority as typed monitor session → active agent claim → ambient identity, so agent/session content immediately replaces the ambient surface rather than competing with or overlaying it. Ambient animation is capped at 24 texture updates per second, honors reduced-motion/deterministic profiles, and returns automatically when no live session or claim remains. An idle ambient surface is intentionally non-interactive; only an occupied session or claim can open a workstation.

**Idle activation and stale-assignment correction (2026-08-08):** Legacy upper-monitor defaults (`Warp`, `Routing Providers`, `Knowledge + Memory`, and `Queue + Plans`) were retired from both the default contract and persisted boardroom state. Typed-session activation now resolves the exact registry record before any legacy source fallback. This removes the accidental generic `PanelWindow` path where `create_monitor_surface` opened an ARDA monitor URL without `__section`, causing `SectionNavigator` to fall through to the first section (`Sovereign World`). Routing ownership and generated source-map authority now use `manwe_aule`; stale `charon_hermes`/`manwe_hermes` owner labels and Hermes source paths were removed from the live projections and their Aule generators. Evidence: 33 focused frontend tests, production build (2,604 modules), 8 Aule library tests, 5 focused Manwe driver tests, zero targeted lint errors, zero remaining `charon_hermes|manwe_hermes` matches, and native Tauri inspection showing the five ambient displays with no extra workstation window present.

**Documentation synchronization (2026-08-08):** The HUD README, BREAKDOWN, boardroom README, boardroom contract, merged scene contract, and slot-component contract now describe the live five-slot CanvasTexture architecture, idle non-interactivity, exact-session workstation continuity, external ownership rails, and distinct lower command instruments. They also record the remaining generic-panel Hermes dashboard/CLI compatibility path honestly as a retirement item rather than treating it as monitor authority or renaming it cosmetically.

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

**Phase 7 implementation evidence (updated 2026-08-10):** Task 6 has a production `MonitorOwnershipRail` mounted outside every upper display aperture. Idle rails remain dormant; an occupied rail derives a stable owner-color fingerprint, distinguishes typed session authority from a legacy claim, displays healthy/expiring/expired lease states without text, and animates at a bounded 15 Hz only while occupied. The rail receives each slot's own session/claim directly inside the existing five-slot map, so it introduces no singleton ownership state and never overlays agent content. All-five in-memory and persisted cardinality now pass; barrier-synchronized Rust tests prove same-slot conflict serialization and atomic release/reclaim; existing registry tests cover wrong-owner rejection, isolated expiry selection, revision conflicts, and snapshot restoration. Focused HUD convergence tests passed 46/46 and focused Tauri monitor-surface tests passed 34/34. Native idle geometry and the five-upper composition are verified, but the Phase 7 product gate remains open until one native run visibly exercises five owners, middle release, sixth-agent reclaim, expiry isolation, and recovery together.

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
9. Retire the active generic-panel Hermes dashboard/CLI compatibility module,
   physical hotspots, settings presets, and localhost `:9119` launcher after
   mapping any still-required capability to explicit Manwë/Aulë ownership.

**Performance gate:** At the native target resolution, five occupied monitors remain responsive to pointer activation and workstation opening. Record frame timing, memory before/after repeated claim cycles, and media resource cleanup evidence. Do not invent an FPS number in advance; measure and document the accepted baseline.

**Status — partially implemented:** URL/path/sandbox/component validation, muted boardroom media, explicit unsupported states, reduced-motion/static rendering, bounded ownership-rail cadence, CanvasTexture disposal, and per-renderer failure messages are implemented and covered by focused tests. The generic Hermes dashboard/CLI compatibility path is documented as non-authoritative but has not yet been fully retired. Native five-surface frame timing, memory/resource-cycle measurements, hidden-renderer throttling, malformed/live failure isolation, and shutdown cleanup evidence remain open.

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

**Current Phase 9 position (2026-08-11):** The normal native HUD has been restarted successfully with the accepted five-upper/five-lower composition intact, and a one-session markdown lifecycle was previously exercised end-to-end. The canonical operator projection is mounted through the typed descriptor path and the dev-only acceptance control can claim it and open its authoritative workstation. A real muted Chromium/CDP session now provides visibly changing website pixels inside physical Monitor 1, with revisioned native-frame delivery used where WebKit cannot consume multipart MJPEG directly. This advances—but does not close—the native walkthrough. The required simultaneous five-content/five-owner run, every-monitor activation, content mutation, release/reassign/expiry/restart sequence, blocked-embed exercise, measured performance baseline, and explicit operator acceptance remain open.

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

- [x] Five physical upper monitors map to five canonical assignable slots.
- [x] The center monitor is independently claimable.
- [x] All five physical model bindings are unique; the center and center-right no longer duplicate `upper_monitor_3`.
- [x] Each occupied screen fills its authored 3D aperture; no small card remains.
- [x] Website rendering is demonstrated natively.
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
- [x] Desk surfaces remain operator-owned; their accepted visual reset does not
  transfer ownership to monitor sessions.
- [ ] Focused/full HUD tests, lint, build, Rust tests, Cargo check, docs health, and diff check pass.
- [ ] The normal native HUD is left running for operator inspection.
- [ ] The operator explicitly accepts the visual and interaction result.

Only after all items pass may this plan be archived and described as complete.

**Implementation status summary (updated 2026-08-11):** Phases 0–3 are complete. Phase 6 same-session lifecycle is complete for the bounded markdown proof. Phase 7's registry, persistence, race-safety, and ownership-rail implementation is machine-verified, while its combined native orchestration gate remains open. The first real-browser website/capture slice is natively demonstrated on Monitor 1. Phases 4, 5, and 8 remain partial; Phase 9's simultaneous multi-content walkthrough and the other unchecked completion items remain archive blockers.
