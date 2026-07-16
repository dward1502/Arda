<!-- sigil: REPAIR -->
# Interaction Contract

This document defines the interaction model for the ARDA scene runtime.

## Purpose

Interactions must be scene-native and predictable.

The runtime must not rely on hidden dashboard-era assumptions such as “click a
card and switch a page.”

## Interaction Types

The runtime supports:

- `hover_reveal`
- `focus`
- `activate`
- `transition`
- `spawn_workstation`
- `open_native_window`
- `return_to_scene`

## Activation Rules

Every activatable anchor must define:

- `anchor_id`
- `interaction_type`
- `result_state`
- `camera_effect`
- `overlay_effect`

## Allowed Results

An activation may:

- focus an anchor
- open a workstation
- transition to another scene
- reveal tactical overlay content

An activation may not:

- silently replace the user’s entire context
- trigger ambiguous multi-step behavior without a visible cue

## Priority Rules

When multiple interactions compete:

- scene transition wins over passive overlay
- workstation spawn wins over cosmetic animation
- explicit operator action wins over ambient automation

## Exit Requirement

The interaction contract is satisfied only when scene behavior can be built from
explicit anchor-driven events instead of ad hoc UI callbacks.
