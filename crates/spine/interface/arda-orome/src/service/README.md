---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "unwired_service_source_map"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-26"
---

> 🜏 Soterion: ⟁ unwired_service_source_map | owner: HADES | status: active | reviewed: 2026-07-26

# arda-orome service source

This directory is not attached to `arda-orome`'s current `lib.rs` module graph. Its Rust files are
therefore not compiled or covered by the crate's passing Cargo gates.

The sibling `../service.rs` is the sole canonical service root and declares this directory's Rust
children. That root is itself still absent from `lib.rs`, so this tree remains unwired. Do not
describe this directory as a live runtime surface or expose it wholesale.

See `../../PLAN.md` for the required wire/migrate/retire decision and `INDEX.md` for the exact child
inventory.
