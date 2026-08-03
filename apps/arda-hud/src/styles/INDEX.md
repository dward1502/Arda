---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "directory_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-03"
---

> 🜏 Soterion: 📜 directory_index | owner: HADES | status: active | reviewed: 2026-08-03

# Index: apps/arda-hud/src/styles

The canonical stylesheet entrypoint is `apps/arda-hud/src/index.css`, imported
by `apps/arda-hud/src/main.tsx`. The complete standalone tree is available in
[`TREE.md`](TREE.md), with a machine-readable sibling in
[`TREE.json`](TREE.json).

## File tree

```text
styles/
├── INDEX.md
├── TREE.json
├── TREE.md
├── adapters/
│   └── fleet.css
├── components/
│   ├── cards.css
│   ├── controls.css
│   ├── data-display.css
│   ├── hermes-dashboard.css
│   ├── media-library.css
│   ├── modules.css
│   ├── research.css
│   └── service-surfaces.css
├── foundation/
│   ├── base.css
│   ├── keyframes.css
│   ├── themes.css
│   ├── tokens.css
│   └── utilities.css
├── layout/
│   ├── app-shell.css
│   ├── panels.css
│   └── workspace.css
├── scene/
│   ├── boardroom.css
│   ├── hud-instruments.css
│   ├── scene-stage.css
│   ├── terminal-surfaces.css
│   ├── workstations.css
│   └── world.css
└── tokens/
    └── nightcity.tokens.ts
```

## Directory roles

- `foundation/`: reset/base rules, CSS variables, themes, animation keyframes,
  and shared utility classes.
- `layout/`: application shell, panels, and workspace geometry.
- `components/`: reusable cards, controls, modules, data displays, research,
  media, Hermes, and service surfaces.
- `scene/`: boardroom/world scene overlays, HUD instruments, stage styling,
  terminals, and workstation styling.
- `adapters/`: source/provider-specific presentation overrides.
- `tokens/`: TypeScript design-token exports.

## Entrypoint import order

`src/index.css` imports:

```text
foundation/base.css
foundation/tokens.css
foundation/themes.css
foundation/keyframes.css
foundation/utilities.css
layout/app-shell.css
layout/panels.css
layout/workspace.css
components/cards.css
components/controls.css
components/modules.css
components/data-display.css
components/media-library.css
components/hermes-dashboard.css
components/service-surfaces.css
components/research.css
scene/scene-stage.css
scene/boardroom.css
scene/workstations.css
scene/world.css
scene/hud-instruments.css
scene/terminal-surfaces.css
adapters/fleet.css
```

## Current wiring notes

- `scene/terminal-surfaces.css` is imported but currently empty.
- `scene/workstations.css` is imported but currently empty.
- `tokens/nightcity.tokens.ts` currently has no importer under `src/`.
