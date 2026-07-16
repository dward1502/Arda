---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "monitoring_configuration"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 monitoring_configuration | owner: HADES | status: active | reviewed: 2026-05-21

# Annunimas Monitoring Setup

Canonical monitoring is remote-first and runs on Beelink, not on this workstation.

- Grafana: http://100.103.125.88:3000
- Prometheus: http://100.103.125.88:9090
- Canonical Prometheus config artifact: `config/monitoring-setup/prometheus-central.yml`
- Canonical alert rules: `config/monitoring-setup/prometheus-rules/health-alerts.yml`
- Canonical Grafana dashboards: `config/monitoring-setup/grafana-dashboards/`

Do not start or maintain local Grafana/Prometheus on the Annunimas workstation. This directory is the repository copy of the Beelink monitoring configuration and dashboard bundle.

## Live-data status verified 2026-05-21

Prometheus on Beelink is reachable and currently exposes live series for:

- `annunimas-orchestrator` from `100.78.138.113:9101`
- `llama-server` from the backbone, Beelink standalone, and Warden lanes
- `node` exporter metrics for Beelink and other fleet nodes
- Prometheus self-monitoring on Beelink

Known external issue: the `charon` scrape target exists in Beelink Prometheus, but `100.78.138.113:5110/metrics` is currently connection-refused. The dashboards now expose that failure through `up{job="charon"}` instead of pretending stale Charon provider metrics exist.
