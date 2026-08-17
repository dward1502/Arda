---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "delegation_map"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 delegation_map | owner: PROMETHEUS | status: active | reviewed: 2026-08-17

# Ambient Agent Workstream and Branch Map

> **For Hermes:** Delegate only packets whose dependency gate is satisfied. Give every worker the allowed/forbidden paths, acceptance command, and required evidence from this document.

## 1. Integration model

Use one program integration branch and short-lived phase branches. The current planning branch is `plan/ambient-agent-program`; implementation branches should be created only after this plan package is reviewed and merged into the chosen integration base.

Recommended topology:

```text
program/ambient-agent
├── feature/ambient-01-launcher-runtime
├── feature/ambient-02-hermes-continuity
├── feature/ambient-03-hud-mirromere
├── feature/ambient-04-presence-identity
├── feature/ambient-05-relic
├── feature/ambient-06-outpost-actuation
└── research/ambient-product-validation
```

Do not create all branches and let them drift indefinitely. Create a branch when its prerequisites are green; rebase or merge the current integration branch before implementation; close it after focused review.

## 2. Shared-file ownership

| Path | Primary owner | Parallel-edit rule |
|---|---|---|
| `docs/plans/ambient-agent/` | program coordinator | Phase workers update only their own phase doc after verification |
| `docs/plans/ARDA_PRODUCT_PLAN_SUITE.md` | program coordinator | No phase branch edits without explicit reconciliation packet |
| `README.md` | program coordinator | No concurrent edits |
| `Cargo.toml`, `Cargo.lock` | integration coordinator | Dependency edits serialized |
| `src/main.rs`, `services.toml` | Phase 1 | Phase 2+ consume interfaces; no parallel edits |
| `apps/arda-launcher/**` | Phase 1 | Exclusive until Phase 1 merge |
| `config/systemd/**`, lifecycle install scripts | Phase 1 | Phase 5 may add RELIC units only after Phase 1 merges |
| `crates/engine/src/adapters/hermes.rs` and Hermes bridge/plugin paths | Phase 2 | Exclusive; do not mix launcher lifecycle changes |
| `crates/spine/memory/arda-vaire/**` | Phase 2 | Phase 4 consumes claims through public contracts, no direct edits |
| `apps/arda-hud/**` | Phase 3 | Keep on current visual lineage; other phases consume contracts |
| Mirromere surface schema location chosen by Phase 3 | Phase 3 | Freeze schema before Phase 4 implementation |
| `outposts/arda-outpost-protocol/**` | protocol packets in Phases 4/6 | Serialize schema commits; Phase 5 reuses runtime presence |
| `outposts/arda-relic-bridge/**` | Phase 5 | Exclusive |
| External CITADEL renderer | Phase 5 | Separate repository/branch; provenance boundary remains explicit |
| Real printer/robot adapters | Phase 6 | One device adapter per branch after simulation contract merges |
| `docs/research/` ambient-agent market/user syntheses | Track 7 | No participant-identifying or private commercial data in Git |
| `core/state/queue_active.json`, `core/state/queue_summary.json` | runtime projection producer | Never stage as part of these phase commits |

## 3. Independent packets

Packets in the same row may run in parallel after their prerequisite is satisfied:

| Gate | Parallel packets |
|---|---|
| Program docs merged | P1-A lifecycle schema; P1-B desktop packaging audit; P2-A upstream Hermes extension audit; P3-A HUD surface inventory |
| P1 lifecycle schema frozen | P1-C systemd target; P1-D launcher status UI; P1-E HUD launch/recovery tests |
| P2 handoff schema frozen | P2-C Hermes event bridge; P2-D Vairë continuity records; P3-B Mirromere surface schema |
| P3 surface schema frozen | P3-C HUD aperture renderer; P3-D second-monitor native renderer; P4-A presence schema |
| P4 claim schema frozen | P4-B token provider; P4-C local vision provider; P4-D privacy/consent UI |
| Existing runtime-presence verified | P5-A bridge hardening; P5-B renderer mapping; P5-C physical recovery/soak |
| P6 manifest/intent schemas frozen | One simulated adapter; policy engine tests; one real-device adapter per device |
| Any technical slice proven | Track 7 competitive research; problem interviews; consented demo study; bounded offer test |

## 4. Required task-packet header

Every delegated implementation request must include:

```text
Goal:
Dependency gate:
Allowed paths:
Forbidden paths:
Existing authorities to reuse:
Contracts consumed/produced:
RED test and expected failure:
Implementation boundary:
Verification commands:
Native/physical acceptance:
Docs/evidence to update after proof:
Commit message:
```

A worker may not broaden allowed paths because a nearby cleanup looks useful. If an unowned shared file must change, stop and return a minimal integration request.

## 5. Review ladder

Each packet receives two reviews:

1. **Specification review**
   - implements only the assigned contract;
   - uses canonical authorities;
   - satisfies explicit error/stale/privacy behavior;
   - includes required tests and evidence.
2. **Code-quality review**
   - no secret exposure;
   - no parallel authority or fixture-as-live behavior;
   - cancellation/restart safe;
   - bounded resource use;
   - project style and strict gates pass.

Only then may the integration coordinator merge the packet.

## 6. Branch-specific closeout commands

All branches:

```bash
git status --short
git diff --check
git diff --cached --check
git diff --cached --stat
```

Rust packets add targeted `cargo fmt`, `cargo test -p <package>`, `cargo clippy -p <package> --all-targets --all-features -- -D warnings`, and `cargo check -p <direct-consumer>`.

HUD/launcher packets add from the owning app directory:

```bash
pnpm test
pnpm run lint
pnpm run build
```

Native claims require `pnpm run tauri build` or the repository's stable package command plus an observed native launch. Physical claims require direct display/device evidence and recovery testing.

## 7. Integration order

1. Merge Phase 1 lifecycle contract and deterministic startup.
2. Merge Phase 1 desktop icon, launcher, HUD handoff, and restart proof.
3. Merge Phase 2 Hermes event/session bridge and Vairë continuity projection.
4. Freeze `arda.surface-handoff.v1`.
5. Merge Phase 3 Mirromere surface schema, then its HUD and native consumers.
6. Merge Phase 4 presence claims and local providers.
7. Revalidate Phase 5 existing RELIC transport against the now-running lifecycle substrate; merge renderer/recovery improvements.
8. Freeze Phase 6 outpost action contracts and prove simulation before any real actuator.
9. Integrate one real device at a time.

## 8. Conflict prevention

- Do not run parallel workers against `Cargo.lock`, `services.toml`, `src/main.rs`, root README, or the product-plan index.
- Do not have Phase 3 and Phase 5 independently invent scene or source-truth semantics.
- Do not have Phase 2 and Phase 4 independently invent identity/session models.
- Do not let a device adapter add approval logic; it consumes governance disposition.
- Do not let UI code mint lifecycle, session, memory, presence, or execution truth.
- Do not merge generated queue projections or runtime state with source commits.
- Do not push branches unless explicitly requested; local verified commits are the default.

## 9. Program coordinator checklist

Before dispatch:

- confirm branch and working tree;
- verify prerequisites from live source/runtime;
- assign exclusive paths;
- include exact tests and contract version;
- identify required reviewer;
- record dependency on integration branch commit.

Before merge:

- read the diff, not only the worker summary;
- rerun focused gates on the integration host;
- verify no unrelated runtime projection changes are staged;
- verify source-truth and authority boundaries;
- update the phase plan only after evidence exists;
- create the next branch from the updated integration base.
