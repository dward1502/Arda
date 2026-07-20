# CHARON Plan Review

## Overview
CHARON is the Arda inference routing and provider health subsystem. It owns model/provider selection, route-class policy, local and edge-capable backend posture, cooldown/degradation tracking, and routing evidence for operator-facing autonomy decisions.

## Core Runtime Surfaces
The current CHARON contract is represented by these primary surfaces:

- `core/projects/Plans/CHARON.md` — quick reference and plan pointer
- `core/state/charon_router.json` — router projection and provider posture
- `config/charon.providers.toml` — dynamic provider/model configuration
- `data/charon/state.jsonl` — runtime route/provider state ledger
- `core/metrics/by_crate/charon/` — crate-level metrics output path
- `core/state/fleet_backbone.json` — fleet/node context for local and edge providers

## Current Contract
CHARON owns:

1. **Inference routing** across local, edge, and cloud/aggregator providers.
2. **Provider health and cooldown state** exported for operator and ARDA visibility.
3. **Route-class policy** including task capability, context window, streaming, structured-output, tool, latency, privacy, and execution-lane constraints.
4. **Config reload posture** through `arda-cli charon reload-config` and `config/charon.providers.toml`.
5. **Serialized runtime evidence** through JSON/JSONL state surfaces where malformed records are surfaced rather than silently hidden.
6. **Fleet-aware local routing** where edge/backbone providers depend on live Tailscale/fleet node health.

## Observed Runtime State
The latest inspected router projection is populated but degraded:

- Provider count: 23
- Degraded providers: 12
- Cooldown providers: 4
- Bootstrap recovery failed total: 6
- Online local/edge targets observed include `edge_core`, `edge_beelink_light`, and `edge_guardhouse`.
- Offline or unrecovered targets include `edge_backbone`, `edge_backbone_coder`, `edge_backbone_vision`, `edge_carnice`, and `edge_laptop`.
- Several recovery attempts are blocked by fleet/SSH posture, including Tailscale peer offline and strict host-key failures.

This means CHARON is not absent, but its live provider mesh is environment-dependent and currently degraded.

## Provider Configuration Notes
`config/charon.providers.toml` defines multiple provider classes:

- Direct cloud or aggregator providers such as OpenCode/OpenRouter-style lanes.
- Local/edge providers such as `edge_core`, `edge_beelink_light`, `edge_backbone`, `edge_backbone_coder`, and `edge_guardhouse`.
- Per-model capabilities including task classes, context windows, tool support, structured output, streaming, visible reasoning, aliases, quality band, access tier, and rate limits.

The file explicitly warns that `healthy = true/false` should not be set in provider config; live health belongs to CHARON runtime probes. Static participation should use `enabled = true/false`.

## Implementation Status

### Completed / Present
- Core CHARON crate exists at `crates/arda-charon`.
- Dynamic provider configuration exists at `config/charon.providers.toml`.
- Router projection exists at `core/state/charon_router.json`.
- Provider state/cooldown/degradation posture is exported for ARDA/operator visibility.
- Provider model capabilities are represented in config for routing decisions.
- Fleet bootstrap recovery evidence is embedded in the router projection.

### Degraded / Blocked
- The provider mesh is partially degraded because several fleet nodes are offline or not recoverable from current SSH/Tailscale posture.
- The quick reference still names a Prometheus `/metrics` endpoint as an open task and warns not to implement it in isolation before broader observability convention selection.
- Live reload/service claims require fresh service checks before release or operational status claims.

### Follow-up Work
1. **Provider mesh health repair**
   - Re-probe fleet node availability.
   - Repair Tailscale/offline-node blockers.
   - Fix SSH host-key trust failures where appropriate.
   - Re-run CHARON recovery and route-health checks.

2. **Metrics endpoint planning**
   - Align with workspace-wide metrics conventions before adding `/metrics` to CHARON.
   - Keep label cardinality bounded: likely labels include fleet/node/crate/route_class/provider_id/model.
   - Add request counters, failover counters, quota-burn counters, and route latency histograms only after the shared convention is selected.

3. **L3 routing hardening**
   - Continue L3 readiness work for route-aware compression/tool-heavy/code-edit turns.
   - Ensure weak local providers are not selected for tasks requiring high context, tool support, structured output, or privacy constraints they cannot satisfy.

4. **Operator documentation**
   - Keep this human plan synchronized with `core/projects/Plans/CHARON.md` and `core/state/charon_router.json`.
   - Treat runtime posture as evidence-based and timestamp-sensitive.

## Verification Commands
Useful focused checks:

```bash
cargo run -p arda-cli -- charon reload-config
cargo run -p arda-cli -- export queue-active
scripts/check_task_queue_append_only.sh
```

For live runtime validation, prefer fresh service and route checks before claiming provider availability:

```bash
systemctl --user status arda-charon.service
scripts/check_charon_health.sh
```

## Alignment with Arda Principles
- **Sovereign routing:** local and edge providers are first-class where healthy and capable.
- **Evidence-first operations:** router state, provider health, cooldowns, and recovery attempts are projected into auditable state files.
- **Safety gates:** route policy should respect capability, privacy, budget, context, and governance signals before dispatch.
- **Operator clarity:** degraded provider posture must be surfaced explicitly rather than hidden behind generic routing failures.

## Open Questions
1. Which shared metrics crate and label convention should become the Arda-wide observability standard?
2. Which provider classes should be allowed for compression-heavy and tool-heavy L3 work when high-context local lanes are offline?
3. Should fleet recovery failures from SSH host-key trust be represented as a separate operator action class from ordinary provider degradation?

## References
- Quick reference: `core/projects/Plans/CHARON.md`
- Router projection: `core/state/charon_router.json`
- Provider config: `config/charon.providers.toml`
- Charon crate: `crates/spine/runtime/manwe`
- L3 routing plan reference: `docs/plans/2026-06-08-l3-readiness-closure-plan.md`
- Compression credential gate: `docs/contracts/hermes-compression-credential-freshness-gate.md`
