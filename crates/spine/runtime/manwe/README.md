# manwe

`manwe` is the Arda inference gateway crate. It owns provider catalog state, adaptive routing, governance/status surfaces, and the transport/proxy layer for routed LLM requests.

## Current capabilities

- Adaptive service spine (`CharonService`) with async provider state, event writer, route history/sessions, bandit/agent-quota/cache/http-client submodules.
- Adaptive routing adapter + policy/scoring/selection with lane fitness, fallback, and provider eligibility checks.
- Catalog reconciliation + runtime state mutation + state I/O for provider/model metadata.
- Axum HTTP transport with `/status`, `/providers/candidates`, provider capability views, metrics, OpenAI-compatible proxy routes, and streaming support.
- Optional integration with `arda-core`, `arda-governance`, `arda-economics`, and `arda-vaire`.

## Connected providers

Provider configuration is loaded from:
- `config/governance/charon.providers.toml`
- runtime state/bootstrapped defaults via `bootstrap_defaults`

OpenAI-compatible providers are supported through the proxy/routing stack. Reconcilable `/models` catalogs are probed for `openai_compat` drivers; others fall back to configured model catalogs.

## Current limitations / improvement areas

- Cache/session cloning: `RouteCandidateCache`, `AgentQuotaWindows`, `BanditStore` use internal mutexes; clone/snapshot behavior is basic.
- Lane fitness snapshot is currently a stub returning `None`.
- HTTP client cache is always lazily initialized from `Option`; no explicit prebuild/budget.
- Some capabilities/methods are placeholder stubs pending deeper implementation.
- Tests warn under `route_policy_tests` because test items live outside `#[cfg(test)]`.
- `cargo fix --allow-dirty` introduced metadata/cache oddities; prefer isolated repo-state verification.

## Status

This is a mid-repair snapshot. The crate builds with `--features adaptive` after targeted fixes, but still carries warnings and the async/edition lint noise noted above.
