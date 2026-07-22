# manwe — Status

Crate: `crates/spine/runtime/manwe`
Status: live / actively supervised runtime component

## Verified compile/test/runtime snapshot — 2026-07-21/22

The documented checks pass from the workspace root:

| Command | Result |
|---|---|
| `cargo check -p manwe` | pass |
| `cargo test -p manwe` | pass: 10 tests |
| `cargo check -p manwe --features adaptive` | pass; no Manwe warnings |
| `cargo test -p manwe --features adaptive` | pass: 157 adaptive library tests + 10 gateway tests |
| `cargo fmt -p manwe -- --check` | pass |

The default/adaptive feature boundary is restored. `src/types.rs` is the one
canonical Manwe domain type surface, and adaptive modules re-export those types
instead of defining incompatible duplicates. gRPC is independently gated behind
the `grpc` feature.

Live validation on temporary port `127.0.0.1:17171` found seven configured fleet
providers and four healthy/model-confirmed providers. `/healthz`, `/v1/models`,
`/v1/capabilities`, and `/v1/chat/completions` all returned successfully. An
adaptive reasoning request selected `edge_backbone_bonsai27`, returned HTTP 200 and
the exact expected `MANWE_OK` answer, and recorded 22.94 generation tokens/second,
7.648 seconds total latency, and quality score 1.0 in
`data/manwe/route_receipts.jsonl`.

Resource-group serialization was also exercised live against two simultaneous
requests to logical lanes on `annunimas-server`: capabilities reported
`active=1`, `queued=1`, `limit=1`; both requests returned HTTP 200 sequentially.
The temporary `:17171` process was stopped after verification.

## What it does

`manwe` is the local OpenAI-compatible inference gateway. It listens on `127.0.0.1:7171`
and serves `/v1/chat/completions` + `/v1/models`. Its default build preserves explicit
model/provider routing. The `adaptive` feature adds fleet health/model probes,
deterministic capability/context/task-fit selection, bounded resource-group
concurrency, streaming proxying, and route quality/throughput receipts.

Core current surface:
- binary runtime: `src/main.rs`
- config/model routing: `src/config.rs`
- gateway/endpoint catalog types: `src/gateway.rs`
- provider catalog bootstrap: `src/provider.rs`
- charon bridge models: `src/charon_remote.rs`
- transport interfaces: `src/transport.rs`
- routing/authority trait stubs: `src/route.rs`
- crate public bridge shims: `src/support.rs`

Binary listens on:
- `127.0.0.1:7171`
- endpoints:
  - `GET /healthz`
  - `GET /v1/models`
  - `POST /v1/chat/completions`

Default model reference shape: `provider/model`.

## Why it exists

It replaces the older hosted multi-process `annunimas-charon` runtime with a single
local gateway root. Per the workspace refactor plan, `manwe` is the static local root
contract: one local OpenAI-compatible endpoint all local callers should target, providing
a stable boundary even as routing/auth subsystems are ported incrementally.

## Who uses it

- `arda-engine`: supervises the manwe process, uses the crate types for spawn-time wiring.
- `arda-hud`: operator dashboard reads `/v1/models` from manwe.
- `arda-launcher`: relies on engine + downstream-side routes that assume manwe at `:7171`.
- workspace registry/contracts: service registry classifies `manwe` as the gateway.

Relevant verified references:
- `crates/engine/Cargo.toml`: depends on `manwe`
- `crates/engine/src/manwe.rs`: re-exports `manwe`
- `crates/engine/src/harness.rs`: proxies `/v1/models` to manwe
- `apps/arda-hud/README.md`: surfaces live manwe gateway state
- `services.toml`: registers `name = "manwe"` as gateway service
- `docs/REFACTOR_PLAN.md` §2: documents manwe as the frozen local root

## Reception / known gaps

- adaptive routing currently uses deterministic hard filtering plus task-fit scoring;
  bandit learning is not allowed to override health, context, modality, or resource gates
- governance and quota-mesh policy exist in the adaptive subtree but are not yet the
  authority used by the stable HTTP dispatch path
- non-streaming responses produce complete throughput/quality receipts; streaming
  responses currently produce dispatch/header receipts without final token-quality data
- physical resource groups default to concurrency 1 and queue for 30 seconds; these are
  configurable with `ARDA_MANWE_RESOURCE_GROUP_CONCURRENCY` and
  `ARDA_MANWE_RESOURCE_GROUP_QUEUE_TIMEOUT_SECONDS`
- provider health is refreshed every 60 seconds; enrollment alone never implies health

## Risks to monitor

- if no providers are configured, inference will 503
- upstream credential/bind mismatches are runtime-only; no compile-time validation
- future auth/governance injection points are still stubs; treaty over gate may change response headers later
