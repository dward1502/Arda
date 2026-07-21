---
soterion:
  core: true
  agentic: true
  type: resource
  project: [annunimas, monitoring]
  priority: high
  tags: [monitoring, beelink, prometheus, grafana, live-data]
  language: markdown
  format: yaml-frontmatter
  version: 2.0.0
  created: 2026-05-03T22:15:00Z
  last_edited: 2026-05-21T06:39:31Z
  source: /var/home/mythos/Annunimas/config/monitoring-setup/centralized-monitoring-config.md
  validation:
    yaml_syntax: valid
    frontmatter_complete: true
    links_resolved: true
---

> 🜏 Soterion: 📜 monitoring_configuration | owner: HADES | status: active | reviewed: 2026-05-21

# Centralized Monitoring Configuration - Annunimas

## Canonical rule

Grafana and Prometheus are centralized on Beelink. The Annunimas workstation should not run or maintain a local Grafana/Prometheus stack.

- Grafana UI: `http://100.103.125.88:3000`
- Prometheus API/UI: `http://100.103.125.88:9090`
- Repository configuration bundle: `config/monitoring-setup/`

## Repository layout

```text
config/monitoring-setup/
├── README.md
├── INDEX.md
├── centralized-monitoring-config.md
├── prometheus-central.yml
├── grafana-dashboards/
│   ├── annunimas-fleet-overview.json
│   ├── charon-routing.json
│   └── cluster-health.json
└── prometheus-rules/
    └── health-alerts.yml
```

Removed to avoid confusion:

- `config/grafana/` — duplicate local dashboard tree
- `config/prometheus.yml` — duplicate local Prometheus config with stale local paths
- duplicate dashboard copies under multiple local trees

## Prometheus deployment artifact

Use `config/monitoring-setup/prometheus-central.yml` as the repository source for the Beelink Prometheus scrape config. It intentionally uses Beelink-local self targets (`localhost:9090`) only for services that run on Beelink itself.

Current scrape jobs:

| Job | Purpose | Expected live labels |
| --- | --- | --- |
| `annunimas-orchestrator` | Annunimas exporter/autonomy/queue state | `node="annunimas-core"` |
| `charon` | Charon scrape health from the core host | `node="annunimas-core"`, `crate="charon"` |
| `llama-server` | llama.cpp inference lanes | `node`, `lane`, `model` |
| `node` | fleet node_exporter metrics | `node`, `fleet_role` |
| `prometheus` | Beelink Prometheus self-monitoring | `node="beelink"` |

## Grafana dashboard bundle

Dashboards live only in `config/monitoring-setup/grafana-dashboards/`.

- `annunimas-fleet-overview.json` queries the live Annunimas exporter series, including `annunimas_queue_depth`, `annunimas_autonomy_mode`, and `annunimas_charon_failure_budget_remaining`.
- `charon-routing.json` uses live Beelink Prometheus series that currently exist: `up{job="charon"}`, scrape metrics, and Annunimas exporter provider failure budgets.
- `cluster-health.json` uses live Beelink Prometheus series from `up`, `node_*`, and `llamacpp:*` metrics.

## Verified Beelink state on 2026-05-21

- Grafana responds on `100.103.125.88:3000`.
- Prometheus responds on `100.103.125.88:9090`.
- Prometheus has live series for `annunimas-orchestrator`, `llama-server`, `node`, and `prometheus`.
- Prometheus has a configured `charon` target, but the target is down because `100.78.138.113:5110/metrics` refuses connections.
- Grafana API management requires credentials/token; unauthenticated `/api/*` calls return 401.

## Remaining external Beelink steps

1. Import or provision the three JSON dashboards in Beelink Grafana from `config/monitoring-setup/grafana-dashboards/`.
2. Apply `config/monitoring-setup/prometheus-central.yml` on Beelink Prometheus if the live target list drifts from the repository artifact.
3. Restart or reconfigure `annunimas-charon` on `annunimas-core` so `100.78.138.113:5110/metrics` is reachable.
4. Provide a Grafana API token if this repository agent should push dashboards/datasources directly.
