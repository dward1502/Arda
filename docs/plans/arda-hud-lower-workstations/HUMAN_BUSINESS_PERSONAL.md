---
soterion:
  sigil: "FORGE"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 7 — Human/Business/Personal Continuity Workstation

> 🜏 Soterion: Human, Business, and Personal now converge in one privacy-aware continuity surface with explicit snapshot and missing-reference semantics.

## Scope

Phase 7 replaces the stacked Human Realm, Business, and Personal Growth modules with one continuity-focused owner module. It consumes the existing Human, Business, and Personal projections and existing derivations; it does not add a producer or mutation authority.

Implemented surfaces:

- canonical `human_business_personal` composition rendered by `ContinuityFocusedWorkstationView`;
- three selectable and distinguishable horizons: Human, Business, and Personal;
- current-focus rail and selected-record detail rather than rendering every sensitive field at once;
- explicit planned/opportunity and realized-value metrics;
- client/project reference reconciliation against the loaded live inventory;
- concise lower continuity instrument driven by horizon counts and missing-reference pressure.

## Focused information architecture

The workstation has four bounded regions:

1. privacy-aware posture and current-focus rail;
2. three horizon selectors with attention counts;
3. selected horizon collection and one selected-record detail panel;
4. independent source-truth references.

Human retains relationship, commitment, and note context only where the Human projection supplies it. Business owns opportunity, client, engagement, project, and value evidence. Personal now participates through the previously omitted Personal projection. Dense collections use native button selection with deterministic fallback when the selected record disappears.

## Source truth

| Family | Canonical source | Rendering contract |
|---|---|---|
| Human continuity | `core/state/human_context.json` | snapshot source; absent collections remain empty rather than fabricated |
| Business continuity | `core/state/business_runtime.json` plus existing company-ops derivation | snapshot/projected source; opportunities remain separate from realized value |
| Personal continuity | `core/state/personal_runtime.json` | snapshot source, now included in the assigned focused workstation |
| Reference inventory | live loaded `data/business/clients/*.json` and `data/projects/*/project.json` inventories | referenced paths must reconcile to the live inventory or render `missing` |

A stale Business snapshot can retain historical records, but an absent referenced client/project path cannot become evidence of completed or live work. The loader annotates reconciled records and the focused view exposes missing references visibly.

## Privacy boundary

The lower right-wrap instrument exposes only aggregate Human, Business, Personal, and missing-reference pressure. Names, notes, health context, relationship detail, and personal priorities stay inside the operator-opened focused workstation. No personal mutation callbacks were added.

## Tests and verification

Phase 7 adds contracts for:

- reconciliation of projected client/project references against live file inventories;
- missing-reference rendering and status escalation;
- separate planned/opportunity and realized-value metrics;
- Human/Business/Personal horizon presence and selection fallback;
- privacy-respecting lower instrument aggregation;
- canonical single-owner continuity composition and role registration.

Verification on 2026-08-16:

- focused Phase 7 tests: 41 passed;
- standalone TypeScript project build: passed;
- full HUD Vitest suite: 140 files, 568 tests passed;
- production frontend build: passed; 2,619 modules transformed;
- HUD lint: passed with pre-existing warnings only;
- Tauri Rust library tests: 69 passed, 0 failed, 2 Chromium-dependent tests ignored;
- native desktop enumeration: zero windows; no screenshot or pointer-interaction claim made.

Evidence logs:

- [`evidence/phase7-vitest-20260816.log`](evidence/phase7-vitest-20260816.log)
- [`evidence/phase7-build-20260816.log`](evidence/phase7-build-20260816.log)
- [`evidence/phase7-links-20260816.md`](evidence/phase7-links-20260816.md)

## Qualification

No fresh native screenshot, native pointer interaction, or newly launched Tauri binary is claimed. Visual acceptance is limited to deterministic semantic component tests and the production build in this environment. The existing projections remain snapshots rather than live personal or business authorities.
