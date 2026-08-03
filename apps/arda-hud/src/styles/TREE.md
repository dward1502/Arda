<!-- sigil: SCROLL -->
# File Tree: `apps/arda-hud/src/styles`

Generated from the live filesystem on 2026-08-03. The canonical CSS entrypoint is `apps/arda-hud/src/index.css`, imported by `src/main.tsx`.

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

## Entrypoint wiring

`src/index.css` imports the stylesheets in this order:

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

## Current structural facts

- `scene/terminal-surfaces.css` is present and imported but currently empty.
- `scene/workstations.css` is present and imported but currently empty.
- `tokens/nightcity.tokens.ts` exports TypeScript tokens but has no current importer under `src/`.
- Machine-readable sibling: `apps/arda-hud/src/styles/TREE.json`.
