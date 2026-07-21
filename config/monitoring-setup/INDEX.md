---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "monitoring_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 monitoring_index | owner: HADES | status: active | reviewed: 2026-05-21

# Monitoring Setup Index

Canonical Beelink monitoring bundle:

- `README.md` — operator summary and live-data status
- `centralized-monitoring-config.md` — detailed architecture and operations guide
- `prometheus-central.yml` — Prometheus configuration deployed on Beelink
- `prometheus-rules/health-alerts.yml` — Prometheus alert rules loaded by Beelink Prometheus
- `grafana-dashboards/annunimas-fleet-overview.json` — Annunimas autonomy/queue dashboard
- `grafana-dashboards/charon-routing.json` — live Charon scrape/failure-budget dashboard
- `grafana-dashboards/cluster-health.json` — live fleet/node/llama dashboard
- `MONITORING_STATUS_REPORT.md` — status report and remaining external steps

Removed confusing local paths:

- `config/grafana/` — duplicate local Grafana dashboard tree
- `config/prometheus.yml` — duplicate local Prometheus config with stale paths
