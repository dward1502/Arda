---
soterion:
  sigil: "FORGE"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 4 — Governance + Guardhouse Decision Chamber

> 🜏 Soterion: Governance now presents one selected decision record, its evidence and authority, and only the actions valid for that record. Missing Warden connections remain visibly unavailable.

## Scope

Phase 4 replaces the Governance workstation's stacked generic modules with a single focused decision chamber. It reuses the existing review-gate derivation and mutation callbacks rather than creating a new approval authority.

Implemented surfaces:

- focused `GovernanceGuardhouseWorkstation`;
- canonical `governance_guardhouse` composition;
- Governance lower-instrument decision-pressure signal;
- existing review, approval, defer, task-cancel, and task-retry authorities.

## Focused information architecture

The focused workstation now has four bounded regions:

1. a posture rail for policy readiness, autonomy posture, Guardhouse state, and source coverage;
2. an explicit Warden source rail;
3. a left selectable index for review records and governed active tasks;
4. a right selected-record detail with summary, scope, evidence, authority, timestamps, receipt binding, and progressive evidence detail.

The previous second `section_focus` module is no longer part of the canonical Governance composition. Source metadata is integrated into the decision chamber instead of appearing in a duplicate tab.

## Authority preservation

Approval and rejection continue through `submitReviewGateDecision`, which invokes the existing `record_human_augmentation_approval` path. The redesign does not write canonical task authority directly.

Deferral is deliberately non-mutating: it records operator UI state only and states that no approval record was appended. This preserves the existing append-only approval semantics instead of inventing a backend defer receipt.

Active canonical tasks remain selectable records. Their contextual controls retain the existing native commands:

- `cancel_approved_queue_task_action` for `in_progress`, `running`, or `claimed` tasks;
- `retry_approved_queue_task_action` for non-cancelled failed tasks.

Reference-only ATHENA records and terminally decided records expose no mutation controls.

## Source truth

The workstation classifies the audited Warden families independently:

| Family | Canonical path | Rendering contract |
|---|---|---|
| Guardhouse | `core/state/warden_guardhouse.json` | connected, snapshot, projected, stale, or unavailable from provenance |
| edge contract | `core/state/warden_edge_contract.json` | same |
| nightly doctrine | `core/state/warden_nightly_doctrine.json` | same |
| policy authority | `core/state/warden_policy_authority.json` | same |

No matching provenance record resolves to `unavailable`; absence never renders as zero or healthy. Empty review content distinguishes a connected source with no pending records from an unavailable Arandur/HADES/ATHENA source family.

## Lower instrument

The lower Governance instrument no longer derives pressure from total archived record volume. Its primary signal is decision pressure:

- actionable pending decisions;
- unresolved HADES lifecycle incidents.

Reference-only ATHENA records do not inflate pending decision pressure. The shared Phase 3 truth-state resolver remains the source-status authority for the aperture and instrument screen.

## Tests

Phase 4 adds contracts for:

- posture and Warden source-state rendering;
- left-list/right-detail selection;
- evidence and authority preceding action controls;
- connected-empty versus source-unavailable states;
- non-mutating defer behavior;
- active-task cancel authority preservation;
- canonical one-module Governance composition;
- incident/decision-pressure instrument behavior.

Verification on 2026-08-16:

- targeted Phase 4 tests: 4 files, 33 tests passed;
- standalone TypeScript project build: passed;
- full HUD Vitest suite: 136 files, 554 tests passed after final rerun;
- production frontend build: passed; 2,615 modules transformed;
- documentation local-link audit: 52 links checked, 0 broken;
- `git diff --check`: passed.

Evidence logs:

- [`evidence/phase4-vitest-20260816.log`](evidence/phase4-vitest-20260816.log)
- [`evidence/phase4-build-20260816.log`](evidence/phase4-build-20260816.log)

## Qualification

No fresh native screenshot or native pointer interaction is claimed. Desktop discovery returned no windows in this environment. Phase 4 visual/native acceptance is therefore limited to deterministic component tests, semantic controls, TypeScript, the production build, and existing workstation shell contracts.
