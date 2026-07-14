---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: REPAIR
# AIPKG Quick Reference

Status: active (reviewed 2026-04-30; v0 core freeze underway, keep)
Owner: prometheus
Human plan: `human/plans/AIPKG.md`
Core runtime: `core/state/aipkg_contract.json`
Spec root: `spec/aipkg/v0.1/`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

`.aipkg` defines the sovereign package, preflight, receipt, and runtime contract for portable agent tooling.

## Current Contract

- open standard law is separate from marketplace and financial tooling
- zero-work preflight is mandatory
- Triad, Bacon-lite, JouleWork, Love Equation, and Soterion trace are explicit governance gates
- receipts and signed attestations are first-class runtime truth
- profiles are split into `wasm-wasi`, `oci-sandboxed`, and `local-sovereign`

## Primary Runtime Surfaces

- `core/state/aipkg_contract.json`
- `spec/aipkg/v0.1/manifest.example.json`
- `spec/aipkg/v0.1/execution-request.schema.json`
- `spec/aipkg/v0.1/receipt.schema.json`

## Readable Context

Use `human/plans/AIPKG.md` for the operator-facing plan narrative and graph node.
