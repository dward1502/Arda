---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "inference_gateway"
  owner: "MANWE"
  status: "active"
  last_reviewed: "2026-07-21"
crate: manwe
gateway: manwe
status: active
last_reviewed: "2026-07-21"
---

> Manwe: 📜 inference gateway | owner: manwe | status: active | reviewed: 2026-07-21

# Manwe Plan Narrative

## Name / Identity

`CHARON` is now implemented in `crates/spine/runtime/manwe`. This document is
the canonical operator plan for that gateway surface. Historic narration is
preserved at `docs/plans/original-human-plan-narration/CHARON.md`.

## Overview

`manwe` is the Arda inference routing and provider health subsystem. It owns
model/provider selection, route-class policy, local and edge-capable backend
posture, cooldown/degradation tracking, and routing evidence for
operator-facing autonomy decisions. This narrative merges the detailed CHARON
operator narrative with the current live `manwe` crate surface so old operational
detail is retained, not lost.

## Core Runtime Surfaces

The live `manwe`/ex-CHARON contract is represented by these primary surfaces:

- `crates/spine/runtime/manwe` — canonical live gateway crate
- `manwe.toml` — default local provider catalog; falls back to embedded local
  Ollama defaults if absent/malformed
- `crates/spine/runtime/manwe/src/grpc_types.rs` — live gRPC router/state surface
- `data/mnemosyne/obsidian_index.jsonl`
- `docs/plans/CHARON.md` — this operator-facing plan narrative
- `docs/plans/original-human-plan-narration/CHARON.md` — historic narration

## Current Contract

`manwe`/`CHARON` owns:

1. Inference routing across local, edge, and cloud/aggregator providers.
2. Provider health and cooldown state exported for operator and ARDA visibility.
3. Route-class policy including task capability, context window, streaming,
   structured-output, tool, latency, privacy, and execution-lane constraints.
4. Static/local OpenAI-compatible HTTP gateway behavior with config reload
   through `manwe.toml` and optional fleet hydration.
5. Serialized/observable runtime evidence where malformed records are surfaced
   rather than silently hidden.
6. Fleet-aware local routing where edge/backbone providers may depend on live
   Tailscale/fleet node health.

## Observed Runtime State

The current `manwe` default static surface is active; adaptive-mode rebuild is
deferred:

- Default static gateway is live on `127.0.0.1:7171`.
- `/healthz`, `/v1/models`, and `/v1/chat/completions` are exposed.
- `/v1/chat/completions` resolves provider by prefix or `default_provider`
  and forwards upstream with optional bearer auth.
- Adaptive subtree exists under `src/adaptive/`, but `--features adaptive`
  currently fails at baseline rebuild with unresolved
  `arda_economics` / `arda_governance` / `arda_vaire` references and
  `pub(super)` visibility/method access failures across sibling modules.
- Some adaptive behaviors are in placeholder/stub form:
  lane fitness snapshots, route selection, adaptive routing adapter error
  path, and route policy tests.

## Implementation Status

### Completed / Present

- `manwe` gateway crate exists at `crates/spine/runtime/manwe`.
- Core static surface builds cleanly and exposes an OpenAI-compatible HTTP
  gateway on `127.0.0.1:7171`.
- Static provider catalog loaded from local `manwe.toml`; falls back to
  embedded local Ollama defaults if missing/malformed.
- `/v1/models` and `/healthz` are exposed for operator/HUD consumption.
- `/v1/chat/completions` resolves provider by prefix or `default_provider`
  and forwards upstream with optional bearer auth.
- Fleet bootstrap path exists in integration/documentation paths, but active
  repair status should be verified before relying on fleet hydration.
- Adaptive subtree exists under `src/adaptive/` as deferred/feature-gated work.
- gRPC module files exist in the crate; server wiring is not active on the
  default path.

### Degraded / Blocked

- `--features adaptive` fails at baseline rebuild with unresolved
  `arda_economics`, `arda_governance`, `arda_vaire` references and
  `pub(super)` visibility/method access failures across sibling modules.
- Some adaptive behaviors are in placeholder/stub form:
  lane fitness snapshots, route selection, adaptive routing adapter error
  path, and route policy tests.
- Fleet/service bootstrap loading is deferred in active repair status;
  do not treat health probes or recovery scripts as fully trusted until
  adaptive baseline is restored or the static path is explicitly validated.

### Follow-up Work

1. Restore adaptive baseline behind bounded feature layers.
   - Revisit `src/adaptive/service/*` visibility/type failures together.
   - Restore a known-good subset before broader rebuild.
2. Provider mesh health repair.
   - Re-probe local provider availability and embedded-Ollama fallback.
   - Repair any fleet-node health gaps before routing through fleet config.
3. Metrics/observability alignment.
   - Align with workspace-wide metrics conventions before adding `/metrics`
     to `manwe`.
   - Keep label cardinality bounded:
     fleet/node/crate/route_class/provider_id/model.
   - Add request counters, failover counters, quota-burn counters, and
     route latency histograms only after the shared convention is selected.
4. Operator documentation.
   - Keep this human plan synchronized with `crates/spine/runtime/manwe`
     and `crates/spine/runtime/manwe/src/grpc_types.rs` for router/provider
     state evidence.
   - Treat runtime posture as evidence-based and timestamp-sensitive.

## Verification Commands

Use these focused checks after local surface changes:

```bash
cargo check -p manwe
cargo test -p manwe
cargo check -p manwe --features adaptive
```

For live runtime validation, prefer fresh service/route checks before claiming provider availability:

```bash
curl -s http://127.0.0.1:7171/healthz
curl -s http://127.0.0.1:7171/v1/models
curl -s http://127.0.0.1:7171/v1/chat/completions ...
```

## Alignment with Arda Principles

- Local inference sovereignty keeps review-flagged reasoning on-station first
  where healthy and capable.
- Evidence-first operations: state, health, and route posture are exported for
  operator visibility.
- Safety gates: route policy should enforce capability, privacy, budget, and
  governance checks before dispatch.
- Operator clarity: degraded provider posture must be surfaced explicitly
  rather than hidden behind generic routing failures.

## Open Questions

1. Which shared metrics crate and label convention should become the Arda-wide
   observability standard for inference surfaces?
2. Which provider classes should be allowed for tool-heavy or context-heavy
   work when high-context local lanes are offline?
3. Should fleet recovery failures from Tailscale/SSH posture be represented as
   a separate operator action class from ordinary provider degradation?

## References

- Crate: `crates/spine/runtime/manwe`
- Live crate docs: `crates/spine/runtime/manwe/README.md`,
  `crates/spine/runtime/manwe/BREAKDOWN.md`
- Original narration archive: `docs/plans/original-human-plan-narration/CHARON.md`
- Archive docs: `docs/archive/charon-manwe-migration.md`,
  `docs/archive/MANWE_FLEET_LOOKUP_PLAN.md`
