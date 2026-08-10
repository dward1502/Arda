# Arda operator projection v1

`arda.operator-projection.v1` is the single read-only operator projection for
phone, CLI/API, HUD, voice, and optional outposts. Arda owns canonical IDs,
state, freshness, capability selection meaning, and approval authority.
Consumers may derive compact presentation signals, but they must not add state
transitions or reinterpret unavailable/stale/degraded state as success.

Canonical Rust model:
`crates/spine/governance/arda-core/src/operator_projection.rs`

HUD transport parser and typed mirror:
`apps/arda-hud/src/lib/operatorProjection.ts`

The shared fixture under `fixtures/` is parsed and semantically validated by
both Rust and TypeScript tests. It deliberately includes a degraded dependency
and an unavailable optional capability so omission or false completion is
visible in every projection consumer.

The canonical runtime handoff path is
`core/state/operator_projection.json`. The loopback harness exposes the same
validated payload through read-only `GET /v1/operator-projection`: a missing
handoff returns HTTP `404` with `state: unavailable`, an invalid handoff returns
HTTP `422` with `state: failed`, and mutation methods are not routed. CLI and
phone-connected Hermes sessions may consume that endpoint while the HUD reads
the canonical file directly. The API does not rewrite IDs, state, freshness, or
provenance. Consumers must not reconstruct a competing operator truth from
scene-local or channel-local state; canonical mutation remains behind separately
authorized operation endpoints.
