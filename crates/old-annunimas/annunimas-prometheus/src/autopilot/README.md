---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-05-21

# autopilot

Purpose: HADES-generated directory overview for `crates/annunimas-prometheus/src/autopilot`.

## Contents

See `INDEX.md` for deterministic child listing.

## Governance surfaces

- `oracle_gate.rs` runs high-stakes objective plans through ORACLE triad gates and converts the resulting quorum evidence into deterministic Triad Philosopher metadata.
- `governance_policy.rs` classifies operational action classes and blocks delegation when human authorization, HADES review, read-only benchmark evidence, ORACLE quorum, or a non-proceed Triad Philosopher verdict is required.
- `runner.rs` consumes these decisions before task delegation, so non-proceed philosopher verdicts become operational holds instead of advisory-only metadata.
