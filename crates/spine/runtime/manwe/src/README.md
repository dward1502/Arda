---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-04"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-08-04

# Manwe source

Purpose: source overview for `crates/spine/runtime/manwe/src`.

The binary shell in `main.rs` always starts the governed service and HTTP
transport under [`adaptive/`](adaptive/). `config.rs`, `error.rs`,
`routing_adapter.rs`, and `types.rs` are the stable library boundary. The
parallel static provider/receipt/resource-limit modules and standalone gRPC
process were retired during single-runtime convergence.

See [`INDEX.md`](INDEX.md) for the validated direct-child listing.
