---
soterion:
  sigil: "SCROLL"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 orphan retirement record | owner: HERMES | status: implemented | reviewed: 2026-08-16

# Phase 9 — Cleanup and Authority Retirement

## Scope

Phase 9 re-audited the three candidate artifacts from the lower-workstation cross-reference. Static imports, dynamic imports, tracked scripts and manifests, focused tests, application composition, and Git history were checked before disposition. Tauri backup artifacts remain outside this HUD convergence slice as required by the plan.

## Disposition

| Candidate | Disposition | Evidence |
|---|---|---|
| `apps/arda-hud/src/lib/providerRouting.ts` | retired | The tracked file contained only blank lines. No source, test, script, package manifest, generated manifest, or lazy import referenced it. The only non-historical reference was the stale HUD breakdown inventory. |
| `apps/arda-hud/src/components/arda/modules/fleet/FleetWorkstation.tsx` | retired | No external import or symbol consumer existed. Its `FLEET_FOCUSED` module contract was disconnected from the canonical composition registry, and its provider/lane/card content conflicted with Fleet's topology-only ownership. |
| `apps/arda-hud/src/scene/workstations/fleetWorkstationView.tsx` | retained and consolidated | `App.tsx` imports this component and selects it for `systems_health`; its focused component tests exercise topology, selection, missing, and loaded-empty behavior. Duplicate floating-window layout helpers were removed because `lib/bundleDerivation.ts` owns the live helpers used by `App.tsx`. |

## Surviving ownership

- `scene/workstations/fleetWorkstationView.tsx` is the canonical focused Fleet topology renderer.
- `scene/workstations/fleetWorkstationView.test.tsx` is its focused behavior contract.
- `App.tsx` owns composition selection and passes the AULË-derived `FleetViewModel` plus the existing refresh callback.
- `lib/bundleDerivation.ts` owns floating-workstation layout helpers.
- `components/arda/modules/fleet/focusedWorkstationModuleHelpers.ts` owns focused/default role classification only; it does not render Fleet content.
- Routing/provider details remain owned by the Routing workstation.

The surviving Fleet renderer carries an ownership comment at its export boundary. The Phase 9 retirement contract verifies both deleted paths remain absent, the canonical view remains imported and tested, and floating layout remains outside the Fleet renderer.

## Verification

The retirement contract was established red before deletion:

- initial contract: 2 failed, 1 passed because both retired files still existed;
- after retiring the empty routing placeholder: build passed while the remaining retirement assertions stayed red;
- after retiring the disconnected Fleet module: build passed while stale breakdown metadata stayed red;
- after metadata reconciliation and helper consolidation: focused Phase 9/Fleet tests passed, 2 files and 5 tests.

Final verification evidence from the Phase 9 closeout run:

- full HUD Vitest suite: 142 files and 576 tests passed;
- TypeScript project check: passed;
- production frontend build: passed with 2,619 modules transformed;
- lint: passed with pre-existing warnings only;
- Tauri Rust library tests: 69 passed, 0 failed, 2 ignored because Chromium is unavailable;
- HADES documentation audit: 82 local links checked, 0 broken, 0 completion-language issues;
- deleted-path static/dynamic reference sweep: no active code or tooling references;
- `git diff --check`: passed;
- native window enumeration: 0 windows, so native visual acceptance remains unavailable rather than passed.

## Qualification

This phase changes no backend authority, source projection, mutation path, physical slot assignment, or Tauri command. Native visual evidence is not inferred from automated tests. Tauri backup/source artifacts listed by the audit were intentionally not mixed into this commit.
