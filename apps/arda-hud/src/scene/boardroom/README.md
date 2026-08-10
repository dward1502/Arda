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

- Five upper-monitor zones are scene anchors; all five expose canonical,
  independently claimable slots (`monitor_1..monitor_5`). Desk surfaces remain
  operator-owned command instruments.
- `useBoardroomSlotAssignments.ts` persists monitor claims and resolves live claim,
  static assignment, then workspace fallback precedence.
- `App.tsx` synchronizes native `monitor-claim-changed` and
  `monitor-surface-payload` events with boardroom state.
- `monitorSurfaceRuntime.ts` validates and bounds live payloads before they reach
  `BoardroomViewport.tsx`; scene zones resolve through their stable
  `assignmentSlotId` before entering the monitor contract.
- Render precedence is typed monitor session, then active agent claim, then the
  slot's unique ambient identity. Idle ambient monitors are intentionally
  non-interactive and have no static departmental assignment.
- Clicking an occupied typed monitor resolves the exact session registry record
  and opens/focuses its `surfaceSessionId` workstation. It must never fall back
  to a generic source-zone panel while that session exists.
- Monitor activation and dismissal use the Tauri surface bridge; Rust ownership in
  `commands/monitor_surface/` enforces one active owner per canonical slot,
  bounded leases, owner authorization, revision isolation, and exact
  payload-binding authorization.
- Missing, expired, or malformed live state renders explicit fallback/`NO DATA`
  content rather than synthetic telemetry.

## Native monitor rendering

- Production boardroom pixels render through CanvasTexture-backed WebGL planes
  fitted to the authored monitor and desk apertures. Perspective-transformed
  DOM cards and screen-space HTML overlays are not the production path.
- `BoardroomApertureSurface` hosts both boardroom and focused-workstation content
  through the same typed renderer registry.
- Upper ambient animation is bounded and honors deterministic/reduced-motion
  profiles without replacing valid content.
- Lower desk apertures use unique radar, waveform, lattice, reactor, and organic
  signal languages; detailed text belongs in deliberately opened workstations.

Native acceptance is performed against the running `ARDA HUD` window with CUA,
not inferred from the browser build. The acceptance path must confirm visible
content on all five upper monitors and all five lower apertures, prove that an
idle upper monitor does nothing, then open an occupied monitor and verify the
focused window preserves the same session and content.

For a repeatable development-only agent-claim walkthrough, launch with
`VITE_MONITOR_ACCEPTANCE=1 pnpm run tauri dev`. The resulting
`MonitorSurfaceNativeAcceptance` overlay invokes the real Tauri claim, payload,
create/focus, lease refresh, reload, authorization-rejection, release, and dismiss
paths while keeping each result operator-visible. It is absent unless that
development environment flag is explicitly enabled.

The current sparse, low-contrast World View presentation is intentional and is
not a boardroom monitor defect or a visual-refinement priority.
