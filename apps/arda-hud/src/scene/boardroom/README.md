<!-- sigil: REPAIR -->
# Boardroom Scene

Primary ARDA operating scene.

Contains:
- desk geometry and layout logic
- monitor anchor logic
- control panel anchor logic
- permanent hologram/avatar emitter logic
- phase-driven particle presence and support-agent markers
- optional read-only Vairë persona influence with neutral fallback
- city window / world gate anchor

The boardroom is the home scene, not a dashboard wrapper.

## Presence implementation

- `BoardroomViewport.tsx` mounts the permanent emitter and passes the current
  presence state plus canonical Arda root into `AvatarPresenceLayer.tsx`.
- `AvatarPresenceLayer.tsx` loads the latest matching canonical persona
  projection through `statefulPersona.ts` and orchestrates the representation.
- `ParticleOrb.tsx` renders the progressive particle body.
- `particlePresence.ts` owns deterministic phase targets and bounded transition
  stepping.
- `presenceState.ts` remains the pure lifecycle/visual authority; optional
  persona data can tune bounded visual outputs but cannot replace phase,
  scenario, urgency, or alert precedence.

See `../systems/PRESENCE_CONTRACT.md` for the complete runtime, fallback, and
performance contract.

## Agentic monitor surfaces

- Five upper-monitor zones are scene anchors; four expose assignable monitor slots
  (`monitor_left_1..monitor_left_4`). Desk surfaces remain operator-owned,
  read-only reference surfaces.
- `useBoardroomSlotAssignments.ts` persists monitor claims and resolves live claim,
  static assignment, then workspace fallback precedence.
- `App.tsx` synchronizes native `monitor-claim-changed` and
  `monitor-surface-payload` events with boardroom state.
- `monitorSurfaceRuntime.ts` validates and bounds live payloads before they reach
  `BoardroomViewport.tsx`; scene zones resolve through their stable
  `assignmentSlotId` before entering the monitor contract.
- Every active claim gets the live claim preview. The operator-configured focus
  mode controls activation behavior (`native_window`, `remote_preview`, and so on)
  but does not suppress the claimed monitor content.
- Monitor activation and dismissal use the Tauri surface bridge; Rust ownership in
  `commands/monitor_surface.rs` enforces one active owner, bounded leases, allowed
  Hermes identities, and exact payload-binding authorization.
- Missing, expired, or malformed live state renders explicit fallback/`NO DATA`
  content rather than synthetic telemetry.

## Native monitor rendering

- Browser rendering keeps Drei's perspective-transformed HTML so preview cards
  follow the physical monitor planes.
- Native Tauri/WebKit rendering uses the same visible preview content through a
  non-transformed, screen-space `Html` path. WebKitGTK reduced transformed Drei
  content to nearly zero-sized compositor layers even though its accessibility
  targets remained active, which made every at-rest monitor look blank.
- `resolveBoardroomSurfaceRenderStrategy()` is the shared runtime decision for
  upper monitors, lower desk surfaces, fleet previews, and the command core.
  Native rendering does not add a separate transparent hit target; the visible
  surface is also the accessible activation target.
- Fleet preview cards have bounded monitor and desk heights so live metrics stay
  inside their physical apertures in both runtimes.

Native acceptance is performed against the running `ARDA HUD` window with CUA,
not inferred from the browser build. The acceptance path must confirm visible
content on all five upper monitors and all five lower terminal zones, open one
surface through its visible card, and close it back to the boardroom.

For a repeatable development-only agent-claim walkthrough, launch with
`VITE_MONITOR_ACCEPTANCE=1 pnpm run tauri dev`. The resulting
`MonitorSurfaceNativeAcceptance` overlay invokes the real Tauri claim, payload,
create/focus, lease refresh, reload, authorization-rejection, release, and dismiss
paths while keeping each result operator-visible. It is absent unless that
development environment flag is explicitly enabled.

The current sparse, low-contrast World View presentation is intentional and is
not a boardroom monitor defect or a visual-refinement priority.
