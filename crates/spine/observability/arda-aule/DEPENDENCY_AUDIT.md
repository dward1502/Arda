# arda-aule Dependency Audit — Supported Graph

Status: closed on 2026-07-25.

## Supported dependency graph

The default library keeps the small governance dependency graph. `full-cli` activates the Arda
core, memory, runtime, economics, and routing dependencies actually imported by the attached
CEO/Prometheus modules.

| Package | Role | State |
|---|---|---|
| `arda-governance` | governance snapshots, readiness, and Bacon-Lite readers | active |

`cargo check -p arda-aule`, the `full-cli` graph, and all features/all targets pass.

## Resolved migration blockers

- The autopilot uses `core_executor_bridge` to acknowledge canonical queue handoff as pending; the
  active core loop/executor owns completion and the removed `arda_apollo` dependency is not restored.
- Provider/fleet routing belongs to Manwe. Aule emits durable execution intents identifying Manwe
  as routing authority and does not create a dependency cycle by importing Manwe internals.
- The overlapping legacy Prometheus pipeline was retired after CEO execution, governance,
  projections, execution-intent, and Manwe ownership paths were verified.

## Resolved residue

The previously listed runtime names and paths (`ANNUNIMAS_ROOT`,
`/tmp/annunimas-target`, `annunimas_totality`, `annunimas-cli`,
`annunimas_prometheus`, and the old CEO service unit) have zero matches in source.

## Decision

Do not restore stale dependencies merely to preserve historical module names. Aule owns
observability, CEO governance/autopilot, and intent production; Manwe owns provider routing.
