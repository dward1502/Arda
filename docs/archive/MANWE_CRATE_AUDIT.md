# manwe crate audit
Owner: hades | Date: 2026-07-19 | Status: evidence-backed review

## Good areas

- `main.rs` is small and readable: axum routing, explicit `run_http`, gRPC spawn path, clean proxy handler.
- `Config::embedded()` makes zero-config startup actually work.
- Provider resolution by `provider/model` prefix is a useful concrete contract.
- Default binary path compiles independently of adaptive code: `manwe` can run as a thin proxy without the adaptive tree.

## Problems

- `--adaptive / MANWE_ROUTING_MODE=adaptive` currently returns a hardcoded error from
  `AdaptiveRoutingAdapter::route_chat_completions`. In `main.rs` that error is silently
  ignored and falls back to static routing.
- `fleet_persistence.rs` is declared in `lib.rs` (`pub mod fleet_persistence;`) but there
  is no source file at that path, so this module is currently unresolved at compile.
- `config.rs` documents fleet loading, but `load_fleet_providers()` always returns embedded
  defaults; `FleetNode` fields exist but aren't wired.
- `list_models()` returns `created = chrono::Utc::now().timestamp()` per request, which
  violates the usual stable `/v1/models` contract.
- Default proxy errors are lossy: non-JSON and unreachable both collapse to `502` without
  upstream status/body for diagnostics.

## Concerns

- Adaptive code compiles but is not wired. This masks porting debt and generates
  ~337 library warnings.
- Feature/binary boundary is inconsistent: adaptive is compiled by default but remains
  intentionally non-functional.
- gRPC path duplicates listeners:
  - `main()` binds TCP via `TcpListener::bind(addr).await?`
  - `run_http()` binds TCP again via `TcpListener::bind(addr).await?`
  Under `--grpc` this can fail on bind; it also means two independent listeners
  with identical bind logic.
- No real tests exercising proxy behavior; runtime request path has no validation.
- Silent misconfiguration tolerance:
  `Config::load()` swallows parse/file errors and silently falls back to embedded Ollama
  defaults, which is convenient for dev and dangerous in ops.

## Current verified state

- `cargo check -p manwe`: PASS
- `cargo test -p manwe`: 0 unit, 0 doc tests found in this build
- `cargo check -p manwe --features adaptive`: FAILS
  - compile failures are in `route_policy.rs`, `service_events.rs`,
    `state_mutation.rs`, `adaptive_routing.rs`, `route_scoring.rs`,
    `route_candidate_cache.rs`, `bandit.rs`, `agent_quotas.rs`
- Default static proxy is functional at `/healthz`, `/v1/models`, `/v1/chat/completions`

## Proposed runtime contracts

- `gateway.rs:101-120` is the runtime contract for billing-session
  extraction and forbidden-step delivery.
- Preferred path is the env-gated provider-intelligence overlay instead of
  hard failures.

## Recommended next actions

1. Gate/featurize adaptive so it is not compiled by default.
2. Wire fleet loading or remove the dead stub surface.
3. Remove or test `gateway.rs`.
4. Add proxy-path tests.
5. Update provider-intelligence onboarding docs to document `ARDA_ENABLE_QUARANTINE`
   as the switch for stale-model quarantine behavior.
