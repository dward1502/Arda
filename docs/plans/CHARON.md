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
the canonical operator plan for that gateway surface.

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
- `crates/spine/runtime/manwe/src/grpc.rs` — feature-gated gRPC server surface
- `crates/spine/runtime/manwe/src/adaptive/transport/http.rs` — governed
  adaptive HTTP surface
- `data/mnemosyne/obsidian_index.jsonl`

## Current Contract

`manwe`/`CHARON` owns:

1. Inference routing across local, edge, and cloud/aggregator providers.
2. Provider health and cooldown state exported for operator and ARDA visibility.
3. Route-class policy including task capability, context window, streaming,
   structured-output, tool, latency, privacy, and execution-lane constraints.
4. Local inference surface preference for adaptive `execution`/`background`
   lanes via `ARDA_LOCAL_INFERENCE_SURFACE=` `mesh|llamacpp|hybrid`; unknown
   values fall back to `hybrid`.
5. Static/local OpenAI-compatible HTTP gateway behavior with config reload
   through `manwe.toml` and optional fleet hydration.
5. Serialized/observable runtime evidence where malformed records are surfaced
   rather than silently hidden.
6. Fleet-aware local routing where edge/backbone providers may depend on live
   Tailscale/fleet node health.

## Observed Runtime State

The default and adaptive Manwe surfaces compile and test independently:

- Default static gateway targets `127.0.0.1:7171`.
- `/healthz`, `/v1/models`, `/v1/capabilities`, and
  `/v1/chat/completions` are exposed.
- `src/types.rs` is the canonical domain type surface for both modes.
- `--features adaptive` activates fleet health/model probing, deterministic
  health/capability/context/task-fit selection, physical resource-group
  serialization, streaming proxy transport, and route receipts.
- gRPC is independently gated behind `--features grpc`; requesting an omitted
  runtime feature fails explicitly instead of silently degrading.
- Fleet state refreshes from `config/fleet.toml` every 60 seconds. A node must
  answer its probe and expose its configured model before becoming routable.
- Non-streaming routed requests record latency, token counts, generation
  throughput, finish reason, answer/reasoning posture, and optional exact-match
  quality in `data/manwe/route_receipts.jsonl`.

Verified temporary-port evidence on 2026-07-21/22:

- seven configured providers; four healthy/model-confirmed
- adaptive reasoning route selected `edge_backbone_bonsai27`
- HTTP 200, exact `MANWE_OK`, 22.94 generation tokens/second, quality 1.0
- simultaneous same-host requests exposed `active=1`, `queued=1`, `limit=1`
  and both completed sequentially with HTTP 200

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
- Fleet-backed discovery and dispatch share one live catalog.
- Adaptive hard filters enforce observed health, model presence, modality, and
  requested context before task-fit scoring.
- Resource groups are derived from physical fleet hostname and default to one
  active request per host, with bounded queueing.
- Route receipts provide real throughput and basic outcome-quality evidence.
- Default, adaptive, and gRPC dependencies are separated by feature boundaries.
- Adaptive subtree compiles and its test suite is active.
- gRPC module files exist in the crate; server wiring is not active on the
  default path.

### Degraded / Blocked

- The full governed adaptive service is active only when `--adaptive` is
  requested and the crate is built with `adaptive`; static mode intentionally
  retains its separate lightweight provider catalog.
- Streaming requests hold resource-group capacity through stream lifetime, but
  their receipts stop at dispatch/header evidence; final streaming token and
  quality accounting remains follow-up work.
- Resource-group concurrency is currently a global default rather than a
  per-host value declared in `config/fleet.toml`.
- Quality scoring is intentionally basic: completion/finish posture plus an
  optional exact-answer expectation. It is not yet a benchmark or judge model.

### Follow-up Work

1. Close the remaining adaptive runtime gaps.
   - Implement or retire the lane-fitness snapshot hook.
   - Decide between true SSE pass-through and an explicitly buffered contract.
2. Extend resource policy.
   - Add per-host concurrency limits to fleet configuration.
   - Prefer alternate eligible resource groups before queueing when task fit is
     equivalent.
3. Extend receipt quality.
   - Finalize token/quality receipts when streaming responses complete.
   - Add bounded task-class benchmark receipts without placing judge work on
     constrained edge nodes.
4. Provider mesh health repair.
   - Repair or intentionally retire the three currently unreachable backbone
     services and the inactive Carnice lane.
5. Metrics/observability alignment.
   - Align with workspace-wide metrics conventions before adding `/metrics`
     to `manwe`.
   - Keep label cardinality bounded:
     fleet/node/crate/route_class/provider_id/model.
   - Add request counters, failover counters, quota-burn counters, and
     route latency histograms only after the shared convention is selected.
6. Operator documentation.
   - Keep this human plan synchronized with `crates/spine/runtime/manwe`,
     `src/grpc.rs`, and `src/adaptive/transport/http.rs` for router/provider
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
- Archive docs: `docs/archive/charon-manwe-migration.md`,
  `docs/archive/MANWE_FLEET_LOOKUP_PLAN.md`
