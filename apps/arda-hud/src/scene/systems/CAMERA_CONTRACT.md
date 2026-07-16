<!-- sigil: REPAIR -->
# Camera Contract

This document defines the minimum camera-state contract for ARDA scenes.

## Purpose

Camera behavior must be explicit.

No implementation agent should infer camera states ad hoc from visual taste
alone.

## Required Properties

Every camera state must define:

- `id`
- `scene`
- `purpose`
- `target_anchor` or `target_zone`
- `distance_profile`
- `motion_profile`
- `transition_in`
- `transition_out`

## Boardroom Camera States

- `boardroom_wide`
- `boardroom_operator`
- `boardroom_console_focus`
- `boardroom_monitor_focus`
- `boardroom_gate_focus`

## World Camera States

- `world_entry`
- `world_traverse`
- `world_district_focus`
- `world_terminal_focus`
- `world_return_focus`

## Camera Rules

- camera transitions must be intentional
- camera changes must reinforce scene meaning
- camera movement must preserve orientation when possible
- camera should not mimic page navigation

## Exit Requirement

The camera contract is satisfied only when every major interaction path maps to
an explicit camera state or camera transition rule.
