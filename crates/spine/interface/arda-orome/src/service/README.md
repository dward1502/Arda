---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "service_source_map"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-27"
---

# arda-orome service source

This directory is compiled when `arda-orome` enables `service-runtime`. The sibling `../service.rs` is the sole service root and declares every Rust child in this directory.

The feature preserves the resident-service migration surface and its tests without claiming that compatibility dispatch is live. `provider_compat.rs` bridges historical service calls to current provider orchestration: compatibility dispatch is deterministic/no-network, inbound polling is empty, and configured providers remain offline until health evidence exists. The separate `provider::HttpJsonTransport` is the concrete receipt-backed path for callers that explicitly configure and policy-gate live HTTP delivery.

Do not create `service/mod.rs`, bypass provider policy, or treat Discord identifiers as canonical task identity. See `../../README.md`, `../../BREAKDOWN.md`, and `INDEX.md`.
