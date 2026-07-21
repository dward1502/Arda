<!-- sigil: REPAIR -->
# Workstation Contract

This document defines the workstation runtime contract.

## Purpose

Workstations are focused operational surfaces opened from scene anchors.

They are the detail layer for deeper interaction, but they must remain part of
the scene-first model.

## Presentation Modes

Workstations may appear in two modes:

1. `in_scene`
2. `native_window`

These modes must share the same content contract.

## Required Workstation Model

Each workstation instance must define:

- `id`
- `origin_scene`
- `origin_anchor`
- `title`
- `subtitle`
- `module_set`
- `presentation_mode`
- `focus_state`
- `placement_state`

## In-Scene Rules

In-scene workstations must support:

- spawn
- focus
- move
- resize
- close
- return focus to parent scene

They must behave like spatial surfaces, not generic browser panels.

## Native Window Rules

Native windows must support:

- explicit spawn
- explicit focus
- explicit close
- synchronization with parent scene state

Native windows are optional per workstation type, not mandatory for all.

## Content Rules

Workstations may host domain modules, but:

- modules do not define workstation architecture
- workstation container behavior is owned by the scene runtime
- scene slots assign workstations/components by stable slot ID rather than by
  hard-coding domain names into visual scene placement

## Transition Rules

Opening a workstation must:

- preserve spatial origin
- preserve the triggering anchor relationship
- avoid disorienting jumps

Closing a workstation must:

- restore scene continuity
- preserve prior camera context when possible

## Exit Requirement

The workstation contract is satisfied only when an implementation can open the
same workstation content either as an in-scene surface or a native Tauri window
without redefining its meaning.

## Companion Contracts

- `SLOT_COMPONENT_CONTRACT.md`
