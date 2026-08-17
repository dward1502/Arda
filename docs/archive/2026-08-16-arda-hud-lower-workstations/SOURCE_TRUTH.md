---
soterion:
  sigil: "LAMP"
  role: "shared_instrument_truth_substrate"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 3 — Shared Instrument Truth Substrate

> 🜏 Soterion: lower instruments distinguish source authority from operational status. Missing, unreadable, stale, derived, snapshot, and live data cannot silently render as equivalent nominal state.

## Scope

Phase 3 establishes a shared source-truth contract before the domain-specific workstation redesigns begin.

Implemented surfaces:

- compact lower Boardroom instruments;
- Boardroom aperture/agent-monitor instrument frames;
- source-family adapters used by Fleet, Queue, Knowledge, Routing, Governance, Human, and Daily Command;
- live monitor-session and claimed-monitor source records.

This phase does not add new backend families or promote a loaded source into authority merely because it is present in the bundle.

## Truth states

`HudInstrumentTruthState` is the shared display contract:

| State | Meaning | Non-color cue | Frame behavior |
|---|---|---:|---|
| `live` | a fresh live/runtime source | `● LIVE` | continuous |
| `snapshot` | a fresh snapshot, config, or manual source | `□ SNAPSHOT` | segmented |
| `projected` | a derived source, or a snapshot declaring upstream authorities | `◇ PROJECTED` | segmented |
| `stale` | a matched source outside its freshness window | `! STALE` | segmented |
| `unavailable` | a matched source is blocked, malformed, or unreadable | `× UNAVAILABLE` | segmented |
| `missing` | no source matching the family contract is available | `? MISSING` | segmented |

Every sourced instrument footer also displays the selected source label. Color remains supplementary; the marker, text label, source name, and line style carry the state without color perception.

## Mapping authority

`apps/arda-hud/src/scene/boardroom/boardroomHudSourceAdapters.ts` owns source-family matching and truth-state classification.

Classification order is fail-closed:

1. blocked or unknown → unavailable;
2. missing → missing;
3. stale → stale;
4. derived source/state or explicit `derivedFrom` authority → projected;
5. live source kind → live;
6. remaining fresh source kinds → snapshot.

When several matching records exist, the adapter preserves bounded IDs and paths, selects the newest matching record for the displayed source identity, and exposes the least trustworthy matched state. A fresh sibling cannot hide a blocked or stale authority.

When no record matches, the adapter returns the family’s canonical expected paths and a visible missing state. Unrelated loaded records remain unused; they do not become runtime authority.

## Operational status separation

Instrument operational states remain:

- `nominal`;
- `watch`;
- `external`;
- `offline`.

Source truth constrains those states:

- missing and unavailable force `offline`;
- stale forces `watch`;
- snapshot and projected map otherwise-nominal instruments to `external`;
- live preserves the derived operational status.

Domain pressure can therefore remain visible without misrepresenting how its data was obtained.

## Reduced-motion behavior

Truth state is independent of animation policy. Phase 3 removes the previous coupling in which a live monitor source was labeled `derived` whenever motion was disabled. Live session/claim provenance now remains live under reduced-motion rendering.

The truth cues and segmented frames are static Canvas operations and require no animation.

## Source paths

Primary implementation:

- `apps/arda-hud/src/lib/ardaProvenance.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudSourceAdapters.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.ts`
- `apps/arda-hud/src/scene/boardroom/BoardroomInstrumentScreen.tsx`
- `apps/arda-hud/src/scene/boardroom/BoardroomApertureSurface.tsx`
- `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- `apps/arda-hud/src/scene/boardroom/MonitorSessionWorkstation.tsx`

Tests:

- `apps/arda-hud/src/scene/boardroom/boardroomHudSourceAdapters.test.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.test.ts`

## Verification

Verified on 2026-08-16:

- TDD red state: 11 source-truth assertions failed before implementation;
- full HUD suite: 135 files and 546 tests passed;
- TypeScript and Vite production build passed;
- 2,614 modules transformed;
- missing, unavailable, stale, projected, snapshot, and live mappings have direct tests;
- loaded-but-unmatched source rejection has a direct test;
- each truth state has a deterministic non-color marker and label test;
- `git diff --check` passed before this record was written.

Evidence:

- [`evidence/phase3-vitest-20260816.log`](evidence/phase3-vitest-20260816.log)
- [`evidence/phase3-build-20260816.log`](evidence/phase3-build-20260816.log)

## Qualification

The build and contract tests verify wiring and deterministic truth presentation. A fresh native screenshot of the new Canvas labels was not obtained because GNOME screenshot access remains denied. This record therefore does not claim native pixel proof. Phase 10 still owns whole-product native visual and performance closeout.
