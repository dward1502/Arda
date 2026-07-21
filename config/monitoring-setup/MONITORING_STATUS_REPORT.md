---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "monitoring_status"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 monitoring_status | owner: HADES | status: active | reviewed: 2026-05-21

# Annunimas Monitoring Status Report

## Executive summary

Status: live Beelink monitoring is functional. Grafana dashboards are imported, Beelink Prometheus is running the repository canonical config, alert rules are loaded, and Charon scraping is healthy.

Grafana and Prometheus are centralized on Beelink. Local duplicate Grafana/Prometheus files were removed from this repository so operators do not confuse local artifacts with the live stack.

Live endpoints verified at `2026-05-21T06:57:48Z`:

- Grafana: `http://100.103.125.88:3000` reachable (`/api/health` returns 200; version 13.0.1)
- Prometheus: `http://100.103.125.88:9090` reachable and healthy
- Charon metrics: `http://100.78.138.113:5110/metrics` reachable after rebinding the Charon HTTP service to `0.0.0.0:5110`

Grafana provisioning completed through the authenticated HTTP API. Imported dashboards:

- `annunimas-fleet-overview` — Annunimas Fleet Overview
- `annunimas-charon-live` — Annunimas Charon Routing - Beelink Live
- `annunimas-cluster-live` — Annunimas Cluster Health - Beelink Live

## Repository cleanup completed

Canonical bundle retained:

- `config/monitoring-setup/prometheus-central.yml`
- `config/monitoring-setup/prometheus-rules/health-alerts.yml`
- `config/monitoring-setup/grafana-dashboards/annunimas-fleet-overview.json`
- `config/monitoring-setup/grafana-dashboards/charon-routing.json`
- `config/monitoring-setup/grafana-dashboards/cluster-health.json`
- `config/monitoring-setup/README.md`
- `config/monitoring-setup/INDEX.md`
- `config/monitoring-setup/centralized-monitoring-config.md`
- `config/monitoring-setup/MONITORING_STATUS_REPORT.md`

Removed to eliminate confusion:

- `config/grafana/`
- `config/prometheus.yml`
- stale one-off monitoring notes that referenced local Grafana/Prometheus or obsolete scrape jobs

## Live data verified in Beelink Prometheus

Prometheus currently returns live series for:

- `up{job="charon"}` — Charon target is healthy (`1`)
- `annunimas_queue_depth` — queue series present
- `annunimas_charon_failure_budget_remaining` — provider-budget series present
- `llamacpp:requests_processing` — inference lane series present
- `node_memory_MemAvailable_bytes{job="node"}` — node exporter series present

All 23 unique PromQL expressions in the three canonical dashboards parsed successfully against Beelink Prometheus on 2026-05-21 after dashboard import and Prometheus redeploy.

Alert rules loaded from `config/monitoring-setup/prometheus-rules/health-alerts.yml`:

- `annunimas-fleet-capacity` — 3 rules
- `annunimas-inference-health` — 2 rules
- `annunimas-scrape-health` — 2 rules

## Current scrape-target state

Healthy live targets include:

- `annunimas-orchestrator` at `100.78.138.113:9101`
- `charon` at `100.78.138.113:5110` (`up{job="charon"}` is `1` after the service bind-address fix)
- `llama-server` lanes at `100.102.250.115:8080`, `100.103.125.88:1234`, `100.110.85.37:1234`
- `node` exporters for Beelink and several fleet nodes
- `prometheus` self-monitoring on Beelink

Charon endpoint fix:

- Root cause: `annunimas-charon.service` launched through `scripts/start_charon_http.sh` with `--http-addr 127.0.0.1:5110`, so Beelink Prometheus could not scrape the Tailscale address.
- Fix applied: `scripts/start_charon_http.sh` now uses `${ANNUNIMAS_CHARON_HTTP_ADDR:-0.0.0.0:5110}`.
- Runtime action: `systemctl --user restart annunimas-charon.service`.
- Verification: `ss -ltnp` shows `0.0.0.0:5110`, local `/metrics` works, Tailscale `/metrics` works, and Beelink Prometheus reports `up{job="charon",node="annunimas-core"} == 1`.

The Charon dashboard surfaces scrape health through `up{job="charon",node="annunimas-core"}` and uses live Charon/provider metrics now available from Beelink Prometheus.

## Prometheus deployment

Beelink Prometheus drift was resolved by deploying `config/monitoring-setup/prometheus-central.yml` to `/home/citadel/.config/prometheus/prometheus.yml` and `config/monitoring-setup/prometheus-rules/health-alerts.yml` to `/home/citadel/.config/prometheus/prometheus-rules/health-alerts.yml`.

The Prometheus container was recreated with the same `prometheus-data` volume plus a persistent rules bind mount:

- `/home/citadel/.config/prometheus/prometheus.yml` -> `/etc/prometheus/prometheus.yml`
- `/home/citadel/.config/prometheus/prometheus-rules` -> `/etc/prometheus/prometheus-rules`
- `prometheus-data` -> `/prometheus`

Before deployment, the new config was validated with `promtool check config` inside the Prometheus container image. A backup of the previous Beelink config was written under `/home/citadel/.config/prometheus/backups/`.

## Required external steps

None for this pass. Re-run dashboard PromQL validation after future dashboard or scrape-config changes.
