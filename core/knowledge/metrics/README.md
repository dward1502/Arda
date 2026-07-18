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

# sigil: SCROLL
# Core Metrics Reference Surface

Canonical JSON metrics output snapshots for ARDA_HUB and downstream integrations.

## Structure
- `by_crate/<crate>/latest.json` - latest normalized snapshot for each crate
- `history/<timestamp>/...` - immutable timestamped captures
- `manifest.json` - snapshot pointers and generated metadata
- `audit_latest.json` - latest integrity/coverage/storage-pressure audit

## Refresh
Use:
```bash
scripts/export_core_metrics.sh
```

This folder is a reference surface and can be consumed by ARDA_HUB visual layers, dashboards, or background analytics jobs.

## Audit Signals
Each export now writes `history/<timestamp>/audit_summary.json` and refreshes
`audit_latest.json` with:
- JSON validation counts
- snapshot file SHA-256 checksums
- export coverage ratio
- storage pressure + oversized-log detection
- queue and governance efficiency summaries
