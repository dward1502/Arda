<!-- sigil: REPAIR -->
# Presence Contract

This document defines the hologram/presence contract for the ARDA scene
runtime.

It is the live presence authority referenced by:

- `../boardroom/BOARDROOM_CONTRACT.md`
- `CONTRACTS.md`

## Purpose

The `hologram_anchor` is the embodied presence of ARDA/Arandur in the
boardroom. It must not be treated as decorative lighting.

This contract defines how the hologram is composed, lit, animated, and
constrained so that the presence reads as an operating peer rather than a
glowing prop.

## Presence Surfaces

Presence is expressed through these surfaces, all bound to the
`hologram_anchor`:

- `hologram_base_plate` — projection-origin geometry
- `hologram_body` — `AvatarPresenceLayer` representation, currently the
  phase-driven `ParticleOrb`
- `hologram_halo` — optional orbital/emphasis ring and bounded support-agent
  markers
- `hologram_pulse_light` — local emissive point light
- `hologram_overlay_panel` — diegetic info overlay anchored to presence

No presence surface may exist outside this list. Additional presence
elements require updating this contract first.

## Composition Rules

- the presence occupies a bounded volume at the hologram anchor; it does not
  fill the room
- the projection axis points up from the base plate; drift is bounded
- presence geometry respects scene scale; it must read as a human-scale
  figure from the operator camera
- presence must remain legible from `boardroom_wide` and `boardroom_operator`
  camera states

## Material Rules

Presence uses a restricted material set:

- `boardroom_hologram_plate` for the base
- `shared_emissive_edge` for halo and projection edges
- a dedicated hologram shader for the body (see
  `shaders/SHADER_CONTRACT.md`)

Presence surfaces may not reuse structural material families such as
`boardroom_wall` or `boardroom_desk`.

## Lighting Rules

- presence is lit internally by `boardroom_hologram_pulse`
- presence does not use a standalone directional light
- presence may cast bounce light into the room via its own emissive falloff;
  it may not illuminate the entire room

## Animation Rules

The presence may animate through:

- idle breathing / sway
- halo rotation
- pulse intensity ramp on activation
- subtle displacement on state change
- phase-driven materialization and dematerialization
- bounded mood-derived density, turbulence, dissolve, and color-temperature
  changes when a valid persona projection exists

The presence may not:

- teleport between positions
- flash or strobe for emphasis
- expand to fill the camera frame
- invert color palette for attention

## State Bindings

`AgentPresenceState`, derived in `presenceState.ts`, is the lifecycle authority.
The live layer receives that typed state at the permanent
`boardroom.avatar.emitter` and derives rendering values through the pure
`presenceVisualState()` and `presenceSupportMarkers()` helpers.

The corresponding runtime concepts are:

- `presence.attention` — visibility / emphasis level
- `presence.mode` — idle / speaking / advising / alert
- `presence.accent` — optional color shift within bounded range
- `presence.anchor_target` — anchor currently held in attention

A component may not invent parallel lifecycle state or bypass those helpers.
Phase, scenario, urgency, primary-agent identity, and alert precedence remain
authoritative even when optional identity state is available.

## Persona Projection Influence

`AvatarPresenceLayer` may read the latest canonical
`core/state/identity/<actor>.json` projection through the existing generic read
boundary and validated `statefulPersona.ts` parser.

- positive valence may increase density, warm color temperature, and calm
  motion;
- negative valence may reduce density, cool color temperature, increase
  turbulence, and increase dissolve tendency;
- only non-stale traits meeting the bounded confidence threshold may add a
  subtle trait accent;
- alert state overrides persona-derived color and pulse behavior;
- missing, malformed, unsupported, stale, or actor-mismatched data produces
  neutral phase-only behavior without an error surface.

Persona projection is read-only visual input. The HUD must not create a second
identity store, projection writer, scheduler, avatar pipeline, or IPC channel.

## Particle Runtime Rules

- animation buffers and reusable vectors are preallocated;
- each frame starts from immutable base positions rather than previous-frame
  output, preventing accumulated drift;
- materialization and dismissal use deterministic bounded transition stepping;
- once idle dismissal reaches zero, particle frame mutation hard-pauses;
- reduced-motion and disabled-motion settings retain a stable representation;
- support-agent markers come only from `presenceSupportMarkers()`.

## Overlay Rules

The `hologram_overlay_panel` is the only allowed presence overlay.

It must:

- stay anchored to the hologram position
- remain subordinate to the presence silhouette
- respect `systems/CONTRACTS.md` for activation

It must not:

- detach and float as a free HUD card
- duplicate boardroom monitor content
- become a primary reading surface

## Restraint Rules

- presence is present; it does not perform
- presence does not react to every state change; it reacts to escalations,
  transitions, and direct operator engagement
- presence does not compete visually with the world gate or center console
- presence at idle should be quiet; the scene must remain usable without
  drawing operator attention to the hologram

## Companion Contracts

This contract depends on:

- `../boardroom/BOARDROOM_CONTRACT.md`
- `CONTRACTS.md`
- `../shaders/SHADER_CONTRACT.md`

## Exit Requirement

The presence contract is satisfied only when the hologram anchor reads as a
restrained, embodied operating peer with bounded animation and diegetic
overlay; lifecycle behavior traces to `AgentPresenceState`; optional persona
input fails neutral; alert precedence is preserved; and the dismissed particle
simulation reaches a hard-paused state.
