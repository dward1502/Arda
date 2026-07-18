# Architecture

`manwe` has two surfaces:

## Stable root

- `src/main.rs` / `src/lib.rs` — crate entry and public re-exports
- `src/config.rs` — config/model routing
- `src/gateway.rs` — endpoint/catalog types
- `src/provider.rs` — provider catalog bootstrap
- `src/transport.rs` — transport interfaces
- `src/route.rs` — routing/authority trait stubs
- `src/support.rs` — bridge shims
- `src/charon_remote.rs` — legacy charon bridge models

This root is the frozen local contract: one local OpenAI-compatible endpoint.

## Adaptive subtree

- `src/adaptive/service/types.rs` — `CharonService` spine
- `src/adaptive/service/*` — capabilities, http clients, quotas, bandit, route selection/scoring/caching, state mutation/io, status, observability, bootstrap
- `src/adaptive/transport/http.rs` and `ipc.rs` — Axum + IPC routes

The adaptive subtree is feature-gated with `--features adaptive` and is where active routing/governance work happens.

## Runtime

- Binds `127.0.0.1:7171`
- Health/model/chat endpoints
- Process supervision is owned by `arda-engine`; manwe itself does not daemonize.

## Consumers

- `arda-engine`: supervises manwe and proxies `/v1/models`
- `arda-hud`: reads `/v1/models` and status
- `arda-launcher`: assumes manwe on `:7171`
