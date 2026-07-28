---
soterion:
  sigil: "ANKH"
  glyph: "🜃"
  role: "observability_home"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-28"
---

# arda-aule

Consolidated observability, governed coordination, telemetry, and operator-control surfaces for Arda.

## Public boundary

The default library surface is deliberately small:

- `contract`: the crate's governance and continuity baseline;
- `council`: supported council compatibility types;
- `governance_metrics`: Prometheus rendering for governance snapshots;
- `service`: readiness and observability briefs.

`arda-aule` owns observability projection and durable coordination records. It does not own provider/model selection (Manwe), task execution (core executors), source memory (Vaire), or the governance decisions it observes (`arda-governance`).

## Features

- Default / `--no-default-features`: the four stable library modules above.
- `telemetry`: OpenTelemetry/tracing event, configuration, and shutdown surfaces. This is the feature consumed directly by Manwe.
- `full-cli`: CEO/autonomy-profile and Prometheus/autopilot/service projections plus the `arda-cli` operator binary. It activates the concrete cross-crate contracts required by that closure.
- `http`: implies `full-cli` and adds HTTP transport, node metrics, and the `arda-cli metrics` exporter/snapshot command.

All 77 `src/**/*.rs` files are compiled by one of these feature closures. There are no generated includes, detached Rust trees, or file-vs-directory `mod.rs` collisions.

## Consumers and integration

Direct Cargo consumer:

- Manwe enables `arda-aule/telemetry` and uses `arda_aule::telemetry` in daemon startup and adaptive service events.

Runtime/operator consumers:

- `arda-cli` commands consume Aule's `full-cli` graph;
- Prometheus scrape jobs, dashboards, and alerting consume its exposition ABI;
- `arda-cli metrics` consumes bounded Vaire observability snapshots and existing core projections.

The existing `annunimas_*` series are retained as a compatibility ABI until scrape jobs, dashboards, and alert rules migrate together. New Mnemosyne series use `arda_*` names.

## Documentation

- `STATUS.md` — dated live verification and remaining boundaries.
- `BREAKDOWN.md` — exact source/feature classification and module map.
- `OWNERSHIP.md` — authority boundaries and required consumer checks.
- `INDEX.md` — exact crate-local navigation.

The historical foundation records remain archived and advisory. They are linked from `BREAKDOWN.md`; obsolete plans are not restored.
