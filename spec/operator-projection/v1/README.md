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
`core/state/operator_projection.json`. Absence means unavailable projection
data; the HUD does not derive a competing operator truth from its local scene
state. Mutation remains behind separately authorized operation endpoints.
