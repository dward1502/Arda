---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "dashboard_bundle"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 dashboard_bundle | owner: HADES | status: active | reviewed: 2026-05-21

# Beelink Grafana Dashboards

Canonical dashboard JSON files for the remote Grafana instance at `http://100.103.125.88:3000`.

- `annunimas-fleet-overview.json` — Annunimas autonomy, queue, backlog, and provider-budget overview.
- `charon-routing.json` — live Charon scrape health plus provider failure-budget metrics from the Annunimas exporter.
- `cluster-health.json` — live scrape target, node exporter, and llama.cpp fleet metrics.

Do not keep duplicate local dashboard copies under `config/grafana/`.
