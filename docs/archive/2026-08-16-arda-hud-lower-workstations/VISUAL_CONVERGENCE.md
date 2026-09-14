---
soterion:
  sigil: "FORGE"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 8 — Shared Visual Convergence

> 🜏 Soterion: The lower workstations now share one physical, focus, truth, type, and motion grammar while retaining domain-specific instrument geometry.

## Scope

Phase 8 converges the implemented lower workstations visually without converting them into recolored copies. It adds no projection, command, or mutation authority.

The shared substrate defines:

- inset console material and vector-line weights;
- one visible keyboard focus outline;
- minimum focused-workstation and compact-instrument type sizes;
- live, snapshot, projected, stale, unavailable, and missing truth colors;
- symbolic and line-style truth cues so color is never the only state signal;
- bounded transition timings and a deterministic reduced-motion stop.

## Distinct domain compositions

The common substrate does not own layout:

| Surface | Retained composition |
|---|---|
| Governance + Guardhouse | decision queue and selected evidence/authority detail |
| Fleet + Backbone | node topology/rack line and selected-node scope |
| Routing + Communications | directed lane/provider flow and selected-route evidence |
| Human/Business/Personal | three continuity horizons and selected continuity record |
| Command Core | coherence scope plus separate tactile command and utility banks |

Nested regions are rendered as instrument divisions rather than repeated rounded cards. Existing domain accents remain bounded: Governance amber, Fleet cyan, Routing blue-cyan, and Continuity mint/gold.

## Design traceability

The production tokens implement the approved [Design References](DESIGN_REFERENCES.md) grammar rather than introducing a new theme:

- console materials, vector lines, focused type, source-corner placement, focus states, and motion budgets map directly to its shared-substrate rules;
- truth colors and line/glyph variants map to its explicit truth-state language;
- bounded timings and the reduced-motion stop map to its motion grammar;
- perimeter corner lines and domain-specific dominant geometry follow the cited R▲ Device UI and Andrew Sullivan translations;
- the retained task-shaped/asymmetric compositions follow the Star Citizen and Territory process translations.

## Source truth and focus

Fleet, Routing, Continuity, and Governance sources use the shared `arda-source-corner` treatment. Each source remains textually named and carries `data-truth-state`; glyph, underline style, and color jointly communicate state. Source markers stay in a compact evidence corner rather than becoming another persistent navigation rail.

Native buttons retain their existing semantics. Shared `:focus-visible` treatment applies to focused workstation controls and Command Core controls without altering callbacks or action authority.

## Contrast and reduced motion

Truth colors were mechanically checked against the shared deep console background (`#02070c`). The minimum contrast is **8.23:1** (`unavailable`), above WCAG AA and AAA normal-text thresholds. The full measured range is 8.23:1–14.25:1.

Under `prefers-reduced-motion: reduce`, focused workstation and Command Core transitions and animations collapse to a deterministic 0.01 ms duration, animation iteration is bounded to one, and smooth scrolling is disabled. Content and truth states remain present.

## Tests and verification

Phase 8 adds contracts for:

- complete shared-token definitions;
- shared material, line, and focused-type use across all four focused domains;
- distinct domain geometry remaining present;
- non-color truth cues and consistent source-corner placement;
- keyboard focus visibility;
- reduced-motion behavior;
- Command Core retaining tactile scope/control-bank geometry.

Verification on 2026-08-16:

- focused Phase 8 contracts: 18 passed;
- standalone TypeScript project build: passed;
- full HUD Vitest suite: 141 files, 573 tests passed;
- production frontend build: passed; 2,619 modules transformed;
- HUD lint: passed with pre-existing warnings only;
- Tauri Rust library tests: 69 passed, 0 failed, 2 Chromium-dependent tests ignored;
- HADES documentation audit: 77 local links checked, 0 broken links, 0 completion-language issues;
- native desktop enumeration: zero windows; no screenshot or boardroom-camera comparison claim made.

Evidence logs:

- [`evidence/phase8-vitest-20260816.log`](evidence/phase8-vitest-20260816.log)
- [`evidence/phase8-build-20260816.log`](evidence/phase8-build-20260816.log)
- [`evidence/phase8-links-20260816.md`](evidence/phase8-links-20260816.md)

## Qualification

The code, semantic component contracts, contrast calculation, TypeScript gate, and production renderer can be verified in this environment. Native boardroom-camera screenshot comparison remains unproven because no native HUD window is available. Phase 8 does not claim pixel-level native acceptance.
