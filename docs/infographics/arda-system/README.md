---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "visual_documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-13"
---

> 🜏 Soterion: 📜 visual_documentation | owner: HADES | status: active | reviewed: 2026-08-13

# Arda System Infographic Pack

Source-backed, standalone HTML/SVG visual documentation for Arda. Open
[`index.html`](index.html) in a browser to navigate the pack.

## Plates

1. [`01-architecture-atlas.html`](01-architecture-atlas.html) — system layers,
   authority ownership, workers, and removable adapters.
2. [`02-operating-loop.html`](02-operating-loop.html) — governed execution,
   action classes, receipts, recovery, and proactive policy.
3. [`03-capability-map.html`](03-capability-map.html) — current 0.9 capabilities
   versus partial and future product-doctrine capabilities.
4. [`04-addition-horizons.html`](04-addition-horizons.html) — bounded 0.9
   improvements, 1.0 foundations, optional additions, and future public-release
   qualification.

## Truth boundary

- Current implementation and maturity claims come from
  [`../../releases/0.9/BASELINE.md`](../../releases/0.9/BASELINE.md).
- Product direction and potential additions come from
  [`../../architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`](../../architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md).
- The completed finite 0.9 improvement record is
  [`../../archive/2026-08-12-arda-0.9-baseline-and-improvement-plan.md`](../../archive/2026-08-12-arda-0.9-baseline-and-improvement-plan.md);
  further 0.9 implementation is defect-driven.
- Future doctrine and optional capabilities are visually distinguished from
  implemented, workflow-proven, or release-supported surfaces.

## Format

Each plate is a self-contained responsive HTML file with inline CSS and SVG or
HTML geometry. No JavaScript, build step, web font, or external asset is
required. They can be viewed offline or printed to PDF from a browser.

Linux preview:

```bash
xdg-open docs/infographics/arda-system/index.html
```
