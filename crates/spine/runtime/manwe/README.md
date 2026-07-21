# manwe

`manwe` is the Arda inference gateway crate. It owns provider catalog state, adaptive routing, governance/status surfaces, and the transport/proxy layer for routed LLM requests.

## Current capabilities

- Static service spine with async provider state, event writer, route history/sessions, bandit/agent-quota/cache/http-client submodules.
- Static routing adapter + policy/scoring/selection with lane fitness, fallback, and provider eligibility checks.
- Catalog reconciliation + runtime state mutation + state I/O for provider/model metadata.
- Axum HTTP transport with `/status`, `/providers/candidates`, provider capability views, metrics, OpenAI-compatible proxy routes, and streaming support.
- Optional integration with `arda-core`, `arda-governance`, `arda-economics`, and `arda-vaire`.

## Connected providers

Provider configuration is loaded from:
- `manwe.toml`; falls back to embedded local Ollama defaults if absent/malformed.
- active fleet nodes from `config/fleet.toml` if present.

OpenAI-compatible providers are supported through the proxy/routing stack.

## Current limitations / improvement areas

- Cache/session cloning: `RouteCandidateCache`, `AgentQuotaWindows`, `BanditStore` use internal mutexes; clone/snapshot behavior is basic.
- Lane fitness snapshot is currently a stub returning `None`.
- HTTP client cache is always lazily initialized from `Option`; no explicit prebuild/budget.
- Some capabilities/methods are placeholder stubs pending deeper implementation.
- Tests warn under `route_policy_tests` because test items live outside `#[cfg(test)]`.

## Status

This is a mid-repair snapshot. The default crate builds cleanly, but adaptive compilation currently fails with 278 errors. Future work should restore adaptive behind bounded feature layers.
