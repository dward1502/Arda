<!-- sigil: REPAIR -->
# Scene Systems Contracts

Merged authority for system-level scene rules.

## Source authority

- `src/scene/systems/SCENE_RUNTIME_CONTRACT.md`
- `src/scene/systems/PRESENCE_CONTRACT.md`
- `src/scene/systems/MATERIAL_CONTRACT.md`
- `src/scene/systems/LIGHTING_CONTRACT.md`
- `src/scene/systems/ATMOSPHERE_CONTRACT.md`
- `src/scene/systems/INTERACTION_CONTRACT.md`
- `src/scene/systems/CAMERA_CONTRACT.md`

Last merged: 2026-06-23

## Scene Runtime Contract

### Objective

The scene runtime must treat every surface as a runtime-driven projection,
not as ad hoc JSX. A surface should:
  - load from a declared zone and optional anchor
  - receive context through scene state
  - render placeholder geometry when the target is unknown
  - allow hidden/paused surfaces without leaking event listeners

### Runtime contract

A scene surface must:
  - declare its source zone and anchor
  - accept visible/hidden state
  - accept focus/followCam state
  - emit lifecycle methods: onMount/onUnmount/onFocus
  - never assume global DOM ownership

### Surface lifecycle

1. mount -> initialize renderer/meshes/models if needed
2. visible -> start updates, effects, input subscriptions
3. hidden -> stop updates, detach listeners, pause shader uniforms
4. unmount -> release disposable assets and event bindings

### Scene runtime state

- visible zones
- focused surface
- hovered anchor
- edit/debug mode flag
- selected district or object

## Presence Contract

### Objective

ARDA presence must feel embodied and physically grounded, not like a floating
head-up display sprite. The avatar is guidance and atmosphere, not gameplay or
decoration filler.

### Binding

Presence must originate from the scene anchor named `boardroom.avatar.emitter`
or an equivalent world anchor when outside the boardroom.

### Presence rules

Presence is:
  - guidance-first
  - status-aware
  - physically tethered to an emitter anchor
  - restrained when idle or when communication is passive

Presence is not:
  - a navigation cursor
  - a particle effects dump
  - a persistent audio spam layer
  - a replacement for explicit state surfaces

### Emissive behavior

Presence emissive intensity must map to state:
  - idle: low steady emissive
  - speaking/acting: brighter emissive pulse or scanline movement
  - error or blocked: red/amber shift, not silent failure

### Render order

Presence should render after opaque scene geometry and before transparent
overlays so it reads as part of the room, not a sticker on top.

### Cleanup

Presence animations and audio playback must stop when:
  - focus leaves the boardroom
  - the app is minimized or backgrounded
  - edit/debug mode hides the operator UI chrome that presence supports

## Material Contract

### Purpose

ARDA materials are surface definitions, not texture-only assignments. A
material must define its channel behavior, performance expectations, and fallback
path so runtime loading stays deterministic.

### Supported material families

- emissive screen glass for monitor/desk displays
- metallic trim for hardware frames and desk surfaces
- dielectric plastic for panels and non-metallic housing
- matte dark paint for large structural surfaces and wall panels
- hologram for projected presence and programmatic effects

### Material behavior

Each material family must:
  - define color, roughness, metalness, and normal/emissive channel use
  - specify if it supports emissive unlit pulses
  - specify if it supports runtime tint changes
  - expose a safe fallback when channels are missing or oversized

### Runtime management

Materials must:
  - share instances across repeated meshes when possible
  - dispose temp material instances created for widgets
  - avoid per-frame material churn
  - warn when textures exceed the asset performance budget

## Lighting Contract

### Purpose

Lighting in ARDA should read as physically coherent interior lighting plus
intentional cyberpunk accents. It is not one single HDRI plus haze.

### Lighting rules

Lighting must:
  - preserve the environment outside the room when seen through windows
  - keep human-readable surfaces above black when possible
  - use emissive accents for screens and active controls
  - keep dynamic lights low-frequency and low-count relative to GPU budget

### Lighting behavior

Emissive surfaces may:
  - pulse
  - flash on state change
  - serve as low-cost local illumination sinks

Dynamic lights must:
  - stay limited to key accent lights and scene state highlights
  - fade quickly rather than strobe
  - not compete with the environment map for primary visibility

### Performance and memory expectations

Limit dynamic lights to the minimum set needed for readability. Prefer emissive
mesh signals over constant-point animation.

## Atmosphere Contract

### Purpose

Atmosphere draws the world, the room boundaries, and the time-of-day mood
without painting over gameplay or data surfaces.

### Atmosphere rules

- fog must not hide district labels or readability-critical surfaces
- sky/environment may be heavy, so it is cached and shared across boardroom
  and world scenes
- bloom and post-processing must remain stylistic, not identity-destroying
- sensor-style color grading should keep cyberpunk accents readable

### Environment maps

- use an HDRI or LDR environment that matches the scene world outside the
  boardroom windows
- keep the environment loading path async and fail-safe
- in native Tauri/WebKit, treat missing HDR as a no-env fallback, not a launch
  blocker

### Sky

The sky should match the environment when viewed outside the window or through
the world gate. Simplified sky alternatives should preserve exposure and color
temperature.

### Reflection rules

Reflectivity should be scene-appropriate:
  - floor and monitor glass: low roughness where appropriate
  - heavy chrome or mirror reflection: surface-specific, not default

### Camera response

Camera tone mapping and exposure must account for emissive monitors and active
scene lights. If the renderer supports exposure control, prefer lightweight
exposure curves over harsh auto-exposure jumps.

## Interaction Contract

### Purpose

Interaction must feel predictable and grounded in the scene, not free-form 3D
navigation for the desktop UX.

### Rules

- hover may reveal affordance text, cursor change, or subtle emissive emphasis
- activation must be explicit and deterministic
- anchor activation should open the annotated target, not nearby geometry
- no anchor should teleport the operator into unrelated context or reset
  state without consent
- edit/debug drags must be observable, reversible, and local by default

## Camera Contract

### Purpose

Camera behavior must support the seated operator experience without becoming a
free fly camera when not intended.

### Rules

- the primary camera is fixed-distance seated, not first-person free roam
- lateral rotation is intentional, pan is disabled in normal mode
- zoom is disabled unless the user is explicitly in an inspect mode
- edit/debug camera adjustments happen through explicit UI controls and
  should not leak into operator presentation mode

### Camera states

Current supported camera states:
  - `boardroom_wide`
  - `boardroom_operator`
  - `boardroom_console_focus`
  - `boardroom_monitor_focus`
  - `boardroom_gate_focus`
