---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: HADES
  status: active
  last_reviewed: 2026-07-22
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-22

# arda-vaire

Continuous memory and identity persistence service for Mnemosyne.

## Verified surface

- Encode: `encode(InformantEvent)` -> Optional `RecallRecentEntry`
- Recall: `recall_recent(hours, crate_filter)`, `recall_relevant(query, hours, crate_filter, scope, limit)`
- Maintenance: `consolidate(hours)`, `stats()`, `status()`
- Identity: `identity_state()`
- Human bridge: `sync_obsidian(vault_path, max_files)`
- Transport: IPC + optional HTTP/SSE daemon
- Governance path: significance-gated store membership and contract dual-write via `with_contract_memory_root`

## Verified evidence

Build/test proofpoint: cargo check -p arda-vaire + cargo test -p arda-vaire 28/28 passing, plus 4 additional integration tests in tests/.

## Live status

See STATUS.md for current health signals, open risks, and ownership.

## Work queue

See CHECKLIST.md for authorship, ownership, and implementation tracking.
