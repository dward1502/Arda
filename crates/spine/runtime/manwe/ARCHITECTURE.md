# Architecture

`manwe` has two surfaces:

## Architecture

- `src/main.rs` / `src/lib.rs` — crate entry and public re-exports.
  Split decision: keep merged lib + binary surface for now because `arda-engine`
  and `arda-aule` already consume public `manwe` types from this crate; adding a
  separate gateway shell crate would duplicate shared types and currently has no
  consumer-ready replacement path. Revisit only when a separate deployment shell
  is required.
- `src/config.rs` — config/model routing.
- `src/gateway.rs` — endpoint/catalog types.
- `src/provider.rs` — provider catalog bootstrap.
- `src/transport.rs` — transport interfaces.
- `src/route.rs` — routing/authority trait stubs.
- `src/support.rs` — bridge shims if present.
- `src/charon_remote.rs` — legacy charon bridge models.
- `src/grpc.rs` — tonic `HealthModelService` + `RouteGovernanceService`; **inactive
  unless `--features grpc --grpc` is passed**.

## Adaptive subtree

- `src/adaptive/service/types.rs` — `ManweService` spine
- `src/adaptive/service/*` — capabilities, http clients, quotas, bandit, route selection/scoring/caching, state mutation/io, status, observability, bootstrap
- `src/adaptive/transport/http.rs` and `ipc.rs` — Axum + IPC routes
- `src/grpc.rs` — optional tonic gRPC server, gated behind `--features grpc`

The adaptive subtree is feature-gated with `--features adaptive` and is where active routing/governance work happens. The stable HTTP dispatch path intentionally preserves static/default-forward behavior and does not route through `AdaptiveRoutingAdapter`; the adapter remains gated until a real scoring pipeline is implemented. gRPC is independently gated and available for Rust-telemetry integrations when explicitly enabled.

## Runtime

- Binds `127.0.0.1:7171`
- Health/model/chat endpoints
- Process supervision is owned by `arda-engine`; manwe itself does not daemonize.

## Consumers

- `arda-engine`: supervises manwe and proxies `/v1/models`
- `arda-hud`: reads `/v1/models` and status
- `arda-launcher`: assumes manwe on `:7171`
