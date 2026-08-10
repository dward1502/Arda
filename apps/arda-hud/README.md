# arda-hud

Local-first Tauri operator environment for Arda. The HUD combines the live
boardroom, typed workstation surfaces, Manwë provider/runtime observation,
governed actions, and the state-driven Arandur presence.

## Stack

- Tauri 2 (Rust shell)
- React 19 + TypeScript
- Tailwind CSS v4 (via `@tailwindcss/vite`)
- Vite 8 dev/build

Mirrors the `arda-launcher` clean-root layout: `src-tauri/` holds the native
shell, `src/` holds the React surface.

## What it does

- renders the R3F boardroom as the primary operating scene;
- treats the five upper monitors as independent agent-owned canvases: idle
  monitors render quiet ambient identities, while occupied monitors render the
  authoritative typed session and open that exact session in a workstation;
- renders the five lower apertures as distinct, nearly textless command
  instruments derived from live state rather than miniature web dashboards;
- projects runtime, research, governance, settings, and review surfaces into
  typed workstations;
- observes Manwë through the configured OpenAI-compatible model, health, and
  status contracts rather than treating a dated port as permanent authority;
- projects routing/provider readiness under the current Manwë + Aulë authority;
  legacy Hermes dashboard/CLI launchers remain compatibility surfaces pending
  explicit retirement and are not upper-monitor ownership authority; and
- renders Arandur as a progressive particle presence driven by canonical
  presence events and an optional Vairë persona projection.

Idle upper monitors are deliberately non-interactive. Legacy static assignments
for Warp, Routing Providers, Knowledge + Memory, and Queue + Plans are not
defaults. Only a valid typed monitor session or live agent claim may activate a
monitor workstation, preventing an unassigned screen from falling through to
the generic Sovereign World panel.

## Arandur presence and persona projection

`AgentPresenceState` remains the visual lifecycle authority. The permanent
`boardroom.avatar.emitter` stage mounts `AvatarPresenceLayer`, which owns the
phase-driven `ParticleOrb` and bounded support-agent markers.

- active phases materialize the orb; idle/resolved phases dematerialize it;
- frame mutation hard-pauses once dismissal reaches zero;
- particle buffers are preallocated and rebuilt from immutable base positions
  to avoid per-frame allocation and accumulated drift;
- alert state retains precedence for pink color and urgent pulse behavior.

The layer may read the latest canonical
`core/state/identity/<actor>.json` projection through the existing generic read
boundary. Valid mood state can influence density, turbulence, dissolve bias,
and color temperature. Only current high-confidence trait evidence may add a
subtle accent. Missing, malformed, unsupported, or stale identity data falls
back to neutral phase-driven behavior. This does not create a second persona
store, avatar pipeline, scheduler, or IPC channel.

The source contracts are documented in
[`src/scene/systems/PRESENCE_CONTRACT.md`](src/scene/systems/PRESENCE_CONTRACT.md).

## Local dev

```bash
pnpm install
pnpm tauri dev      # launches the Tauri window (needs a display)
# or just the web surface:
pnpm dev            # vite dev server on :1421
```

## Build

```bash
pnpm install
pnpm tauri build    # produces the platform bundle under src-tauri/target
```

## Relationship to the rest of Arda

- `arda` (root daemon) supervises Manwë and owns runtime lifecycle.
- `arda-launcher` is the atmospheric onboarding/title screen.
- `arda-hud` is the spatial operator environment for the gateway those two
  produce.

The HUD remains a projection and control surface. Canonical tasks, memory,
identity, policy, and runtime authority stay in their owning Arda subsystems.
