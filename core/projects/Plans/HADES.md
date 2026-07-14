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

# 🪙
# HADES Quick Reference

Status: complete (reviewed 2026-04-30; v0.1 baseline; coin-marked for cleanup/archive targeting)
Owner: hades
Human plan: `human/plans/HADES.md`
Crate: `crates/annunimas-hades`
Core runtime: `core/state/hades_lifecycle.json`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

HADES owns cleanup, sweep, orphan handling, repair signaling, and lifecycle maintenance for system artifacts and ledgers.

## Current Contract

- sweep engine and queue surfaces are live
- WARDEN informant handoff is active
- malformed log and queue record counts are surfaced
- JSONL lifecycle writes are serialized

## Primary Runtime Surfaces

- `data/hades/hades_log.jsonl`
- `data/hades/warden_queue.jsonl`
- `data/hades/athena_handoff_queue.jsonl`
- `core/state/warden_guardhouse.json`

## Readable Context

Use `human/plans/HADES.md` for the operator-facing plan narrative and graph node.
