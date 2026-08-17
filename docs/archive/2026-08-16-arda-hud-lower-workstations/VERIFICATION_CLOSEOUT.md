---
soterion:
  sigil: "SCROLL"
  role: "phase_record"
  owner: "HERMES"
  status: "complete"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 whole-product verification closeout | owner: HERMES | status: complete | reviewed: 2026-08-16

# Phase 10 — Whole-Product Verification and Documentation Closeout

## Outcome

Phase 10 is complete for the current optimized X11 compatibility profile. The complete automated gate set passed, the optimized Tauri candidate was rebuilt after an acceptance-found composition defect, and the current release window was exercised natively at 1920 × 1080 with semantic controls, direct pointer input, keyboard Escape/focus restoration, screenshots, static-demand timing evidence, and a like-for-like resource profile.

The original default-launch result—an alive process with zero enumerable windows and a `0x0` scoped capture—remains historical evidence for that configuration. It was superseded for this plan by launching the same optimized candidate with the repository's explicit X11/WebKit compatibility profile, which exposed a rendered and controllable native window.

## Automated evidence

| Gate | Result | Evidence |
|---|---|---|
| HUD Vitest suite | pass — 142 files, 577 tests | [`evidence/phase10-vitest-final-20260816.log`](evidence/phase10-vitest-final-20260816.log) |
| HUD lint | pass — 149 existing warnings, 0 errors | [`evidence/phase10-lint-final-20260816.log`](evidence/phase10-lint-final-20260816.log) |
| Frontend production build | pass — 2,619 modules transformed | [`evidence/phase10-build-final-20260816.log`](evidence/phase10-build-final-20260816.log) |
| Boardroom asset budget | pass — 81 files, 30,245,745 / 37,748,736 bytes, 0 violations | [`evidence/phase10-boardroom-assets-final-20260816.log`](evidence/phase10-boardroom-assets-final-20260816.log) |
| Soterion Unicode guard | pass — 558 UTF-8 files, 11 glyph/code-point declarations | [`evidence/phase10-soterion-unicode-final-20260816.log`](evidence/phase10-soterion-unicode-final-20260816.log) |
| Optimized Tauri rebuild after acceptance fix | pass | [`evidence/phase10-native-rebuild-20260816.log`](evidence/phase10-native-rebuild-20260816.log) |

The Unicode gate initially exposed a stale package-script contract: `package.json` invoked `scripts/soterion_unicode_guard.sh`, but that script did not exist in the imported Arda history. Phase 10 restored the strict UTF-8, replacement-character, and Soterion glyph/code-point guard before recording the passing result.

## Acceptance-found defect and repair

Native Fleet traversal exposed that specialized views were selected from a resolved primary module ID rather than the workstation slot's semantic ownership. A Fleet slot could therefore fall through to generic composition.

The repair:

- adds `getFocusedWorkstationKind(...)` to the canonical composition authority;
- selects Fleet, Routing, and Continuity focused renderers from semantic slot ownership in `App.tsx`;
- adds regression coverage for all specialized classifications;
- passes the focused workstation suites, frontend production build, and optimized native rebuild.

The rebuilt Fleet, Routing, and Continuity screenshots demonstrate distinct canonical owners rather than the former generic fallback.

## Native launch and interaction evidence

The optimized candidate was launched with:

- `GDK_BACKEND=x11`;
- `WEBKIT_DISABLE_DMABUF_RENDERER=1`;
- `__NV_DISABLE_EXPLICIT_SYNC=1`.

The resulting `ARDA HUD` native window was directly observed at 1920 × 1080 with X11 window id `35651587`. Native evidence includes:

- semantic opening of Governance, Fleet, Routing, and Human/Business/Personal focused owners;
- direct XTEST activation of a physical Command Core/control-bank action and an observable `READY` state transition;
- keyboard Escape closing Governance and restoring the visible focus ring to its native opener;
- current direct screenshots for all focused owners plus Command Core/boardroom state;
- explicit separation between live native observations and state permutations proven by focused tests.

Full record: [`evidence/native/phase10-native-interaction-20260816.md`](evidence/native/phase10-native-interaction-20260816.md).

## Performance and motion evidence

The compatibility profile intentionally selects static demand rendering. A 15-second scheduled sample at 60 Hz produced 901 captures, zero late samples, and zero changed idle samples. An idle animation FPS is therefore not applicable to this profile and is not fabricated.

The adopted optimized native resource contract passed over 60.055 seconds:

| Metric | Result | Limit | Disposition |
|---|---:|---:|---|
| Aggregate CPU, one-core percent | 11.64% | 40.0% | pass |
| PSS peak | 408.04 MiB | 550.0 MiB | pass |
| PSS growth | 14.86 MiB | 16.0 MiB | pass |
| RSS peak | 574.84 MiB | 700.0 MiB observability ceiling | pass |
| Swap peak | 0 MiB | no growth expected | pass |

Evidence:

- [`evidence/native/phase10-native-frame-timing-20260816.log`](evidence/native/phase10-native-frame-timing-20260816.log)
- [`evidence/native/phase10-native-profile-20260816.json`](evidence/native/phase10-native-profile-20260816.json)

## Acceptance disposition

[`ACCEPTANCE_MATRIX.md`](ACCEPTANCE_MATRIX.md) records the final combined native and contract-tested disposition. Direct native proof establishes owner routing, current visual composition, Command Core/control-bank response, keyboard closure/focus restoration, and compatibility-profile performance. Focused tests remain the evidence for deliberately induced stale/missing/unavailable, hidden-polling, reduced-motion, blocked-action, receipt, and duplicate-launch permutations not mutated in the live operator state.

This closeout does not broaden public-release support or replace separate hardware-backed Wayland qualification. It closes the bounded lower-workstation convergence plan and allows the plan directory to move from active planning authority into the documentation archive.
