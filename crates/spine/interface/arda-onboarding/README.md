---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "crate_documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-06-06"
---

> 🜏 Soterion: 📜 crate_documentation | owner: HADES | status: active | reviewed: 2026-06-06

# arda-onboarding

Purpose: Shared onboarding library for First Light environment profiling, prerequisite reporting, guided sessions, private config staging, and human-gated apply receipts.

## Contents

See `INDEX.md` for the deterministic child listing.

## Boundary

This crate may produce onboarding evidence and staged private configuration payloads, but mutation of operator configuration requires the explicit human-gate approval receipt path exposed through `arda-cli onboarding apply-config`.

## L3 Readiness

First Light onboarding includes the L3 readiness checklist from `docs/operations/l3-readiness-onboarding.md`. The checklist separates read-only projection, safe-local packet selection, bounded mutation readiness, and human-required classes.

The crate exposes `l3_readiness_onboarding_checklist()` and adds an `l3_readiness` guided-session step so new operators can verify bounded readiness without overstating broad autonomy.
