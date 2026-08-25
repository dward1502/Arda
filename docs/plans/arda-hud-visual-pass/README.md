---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "plan"
  status: "active"
  owner: "visual-pass"
  last_reviewed: "2026-08-22"
---

# Arda HUD Visual Pass — Assessment & Plan

Scope: `apps/arda-hud` UI/UX structure. This pass is **visual/UX only** — it does
not touch authority boundaries, derivation logic, or acceptance paths.

## Current state (evidence)

### Strengths
- Tokenized design system already exists: `src/styles/foundation/tokens.css`
  defines color, spacing (1–8), radius, HUD kit, machine-grammar substrate
  (`--arda-*`), minimum text sizes, focus outlines.
- Organized stylesheet tree (`foundation/`, `components/`, `layout/`,
  `adapters/`, `scene/`) with INDEX/TREE docs.
- A visual convergence contract test already exists
  (`src/styles/phase8VisualConvergenceContract.test.ts`) — the pass should
  extend, not fight, this contract.
- Module components are individually tested; accessibility tests exist for
  WorkbenchModule.

### Structural risks found
1. **App.tsx is 2,775 lines** and imports ~30+ modules directly. It is the
   de-facto layout authority. Any visual restructure means editing a monolith.
2. **Two competing visual languages coexist**: the rounded glassy token set
   (`--radius-*` up to 1.75rem, soft surfaces) vs. the sharp cyberpunk HUD kit
   (`--hud-radius: 2px`, cut corners, glow). Unclear which applies where.
3. **Mixed styling mechanisms**: tokens.css, per-module CSS
   (e.g. `LearningLoopSurface.module.css`), component CSS files, and
   `nightcity.tokens.ts` — four sources of truth for visual decisions.
4. **Accessibility is uneven**: only 6 `aria-`/`role=` occurrences in
   WorkbenchModule; other modules untested.
5. **Text-density risk**: `--arda-text-min-instrument: 0.5rem` (8px) is below
   readable thresholds; instrument screens are intentionally near-textless, but
   any text at that size needs a deliberate rule, not a default.

## Operator decisions (2026-08-22)
1. **No monolith development** — App.tsx (2,775 lines) must be decomposed into
   shell components; new work goes in focused components, never appended.
2. **Sharp cyberpunk HUD kit is primary** — `--hud-*` / `--arda-*` machine
   grammar wins; rounded glassy tokens are legacy.
3. **`src/styles/` is the styling home** — `nightcity.tokens.ts` is old and to
   be retired; component-scoped CSS that drifted out of `styles/` migrates back
   (or is justified as true component-scoped modules).
4. **Accessibility deferred** — functionality first; a11y pass comes later.
5. **Units: rem/em, not px** — dynamic sizing; where a px value is required
   (hairlines, glows), define it once as a token at standard value.

## Visual evidence (live scene, 2026-08-22)
Observed from running HUD (World View, display-only):
- Upper row: 4–5 distinct idle monitor identities (waveform stripes, network
  constellation, polar/radar grid, rain-chart) — good variety, consistent
  dark frames.
- Lower desk: 4 instrument screens (gold diamond lattice, hex gauge, red
  circular-sweep, green orbit) — near-textless as designed; working well.
- Central hologram: wireframe chalice + geodesic orb over purple-lit pedestal;
  pastel color swatch grid floats beside it — the swatch grid's flat pastel
  rounded squares **clash** with the sharp neon wireframe language (legacy
  rounded vocabulary visible in-scene).
- Console materials and cityscape backdrop are coherent; no visible px/rem
  issues at this layer (scene is canvas, not DOM).

## Constraints (from memory + skills)
- HUD World View is display-only; sparse low contrast there is intentional.
- Lower desk screens are WebGL apertures, not DOM cards — out of scope.
- Reuse native acceptance/authority paths; browser preview is passive.
- No synthetic acceptance: visual changes verified against the running app.

## Proposed folder structure for this pass

```
docs/plans/arda-hud-visual-pass/
├── README.md            (this file — assessment + plan)
├── 01-design-language-unification.md
├── 02-app-shell-structure.md
├── 03-module-visual-audit.md
├── 04-accessibility-and-readability.md
└── WORKSTREAMS.md
```

## Workstreams

### WS1 — Design-language unification (01)
Decide: one primary visual language. Recommendation: the sharp HUD/machine
grammar (`--hud-*`, `--arda-*`) as the identity, with the soft token set
demoted to legacy/fallback. Inventory every component using `--radius-lg+`,
soft surfaces, or non-token colors; converge or document exceptions.

### WS2 — App shell structure (02)
Extract App.tsx layout regions into explicit shell components
(header/rail/dock/workstation host) so visual structure is inspectable without
reading 2,775 lines. No behavior change; tests must stay green
(`pnpm run tauri dev`, vitest suite).

### WS3 — Module visual audit (03)
Per-module pass over `src/components/arda/modules/` (~8.5k LOC): spacing
consistency, header hierarchy, empty/loading/failure states. Prioritize
core-usefulness surfaces first (capture, next action, Personal Operations,
review gate) per governance.

### WS4 — Units & styling consolidation (04) *(replaces a11y for now)*
- Convert px values to rem/em across DOM styles; single tokens for the
  unavoidable fixed values (1px hairlines, glow radii).
- Retire `tokens/nightcity.tokens.ts`; migrate consumers to `foundation/tokens.css`.
- Inventory CSS that drifted into component files; migrate back to
  `styles/components/` or justify as true CSS modules.
- Accessibility explicitly deferred.

## Order & acceptance
1. WS1 first (tokens decide everything downstream).
2. WS2 (structure) before WS3 (per-module polish).
3. WS4 runs alongside WS3 per module.
Acceptance = running HUD inspected live (Tauri dev), screenshots reviewed,
vitest + phase8 contract green. No doc-only completion.

## Opinion (explicitly requested)

The HUD's biggest visual problem is not aesthetics — it's **authority
ambiguity**: two design languages and four styling mechanisms mean every new
surface makes an implicit choice. Unify tokens first (WS1); it's the
highest-leverage, lowest-risk move. Second, App.tsx as layout monolith makes
any visual iteration expensive — the shell extraction pays for itself within
this pass. Third, resist adding new visual vocabulary: this pass should
*reduce* the number of visual decisions a component can make, matching the
"shared substrate, not shared layout" comment already in tokens.css.
