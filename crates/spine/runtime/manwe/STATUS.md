# manwe — Status

Crate: `crates/spine/runtime/manwe`
Status: live / actively supervised runtime component

## Verified compile/test snapshot — 2026-07-21 16:27 PDT

The three commands documented in `docs/plans/CHARON.md` were run from the
workspace root. None currently pass:

| Command | Result | Primary evidence |
|---|---|---|
| `cargo check -p manwe` | failed | 35 errors, 2 warnings: E0432 ×9 and E0282 ×26 |
| `cargo test -p manwe` | failed during compilation | 305 errors, 2 warnings; 255 diagnostics originate in `route_policy_tests.rs` |
| `cargo check -p manwe --features adaptive` | failed | same 35 errors and 2 warnings as the default check |

The first blocking defect is a partial `CharonRequestEnvelope` →
`ManweRequestEnvelope` rename. `src/adaptive/types.rs` defines only
`ManweRequestEnvelope`, while nine adaptive service imports/re-exports still request
`CharonRequestEnvelope`. The 26 E0282 inference errors are downstream fallout from
that missing request type, concentrated in `route_policy.rs`,
`route_selection.rs`, `route_sessions.rs`, and `bandit.rs`.

The default and adaptive checks are currently identical because `src/lib.rs`
unconditionally declares `pub mod adaptive`; the `adaptive = []` feature does not
isolate the adaptive subtree. The stable gateway therefore no longer has the clean
default-build boundary described below.

Test compilation exposes a second layer after the same root failure: stale CHARON
symbols in `route_policy_tests.rs`, duplicate and incompatible root/adaptive
`ModelState` types, and tests that still expect `CharonService::new` to return a
`Result`. These should be repaired only after restoring one canonical adaptive type
surface and the default feature boundary.

## What it does

`manwe` is the local OpenAI-compatible inference gateway. It listens on `127.0.0.1:7171`
and serves `/v1/chat/completions` + `/v1/models`. Requests are forwarded to upstream
providers from a static TOML-backed provider catalog; the gateway is intentionally thin
and does not perform adaptive routing, quota meshing, or request transformation beyond
stripping the local `provider/model` prefix before forwarding.

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

- current integration: files define a `CharonRemote` and authority trait shims, but these surface runtime `NotImplemented` gaps now; error translation is best-effort
- adaptive routing: explicitly deferred; there is no quota mesh or runtime rerouting in the stable root
- provider catalog is static; governance/governed transport selection is not complete
- placeholder counters exist in `ProviderCatalog` for migration staging, but real governance sources are not wired yet
- error path conventions are inconsistent in small places (e.g. `local_placeholder` vs `local` usage, misnamed log line in one branch)

## Risks to monitor

- if no providers are configured, inference will 503
- upstream credential/bind mismatches are runtime-only; no compile-time validation
- future auth/governance injection points are still stubs; treaty over gate may change response headers later
