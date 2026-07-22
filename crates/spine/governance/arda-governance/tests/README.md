---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-07-21

# tests

- Purpose: integration and compatibility verification for `arda-governance`
- Coverage: chain/profile contracts, alignment behavior, injected filesystem roots, public API wire shapes, and stable enum encodings

## Contents

See `INDEX.md` for deterministic child listing.

`fixtures/public_api_v1.json` is a versioned wire-shape contract. Update it only after an explicit compatibility review; removing or renaming a listed field is breaking.
