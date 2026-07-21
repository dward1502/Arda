# Arda Substrate Build Plan

> Arda is not a box. It is a biological substrate: heterogeneous runtime
> surfaces, each with different capability, learning and searching as a network.
> The HUD is the visible shell. Everything else grows outward from there.

## Identity

- Arda is the continuation of Annunimas.
- Legacy `annunimas-*` names are read as `arda-*`/`manwe`.
- The goal is an organized network, not a single-machine setup.
- Surfaces must remain HUD-visible; hidden backends are out of scope.

## Node Philosophy

Treat each runtime surface as a different cell/organ in the substrate:

| Surface | Repo/manifest target | Substrate role |
|---|---|---|
| `arda-hud` | Operator shell | visible surface only |
| `arda-launcher` | Onboarding + bootstrap | operator entry / handshake |
| `arda-engine` | Repo-root supervised daemon | substrate lifecycle |
| `manwe` | Inference gateway/runtime | model/provider route layer |
| `spine/*` crates | Governance, memory, observability, contracts, executors | substrate capabilities |
| Fleet/runtime state | `core/state/*`, `data/*`, `config/routing/*` | substrate nerves |
| Core runtime data | `crates/spine/data/*` | canonical app state |

Notes:
- Heterogeneity is intentional: not every surface needs full capability.
- Visibility is the default; if a capability is invisible to the HUD, it does not count as complete.

## Build Order

### Phase 0 — Single visible shell
- `arda-hud` is the only operator surface.
- Everything else is invisible until exposed through the HUD.
- One runtime: local daemon only, launched from repo root.

### Phase 1 — Local gateway exposed in HUD
- HUD probes local `manwe` endpoint.
- Shows local models, status, health.
- No federated or fleet features yet.

### Phase 2 — One capability/section at a time in HUD
- Each capability appears as a visible section/panel.
- Surface read/write/runtime behavior in `core/state/*` before HUD wiring claims.
- Add one verified surface, then the next.

### Phase 3 — One node/surface at a time
- Add curated surfaces to the HUD.
- Each surface appears with its role, status, and active tools.
- No auto-discovery; surfaces are explicit substrate members the operator adds.

### Phase 4 — Inference wiring
- Once a surface is visible in HUD, wire its inference path through `manwe`.
- One route/queue at a time, visible as a route in the HUD.
- Verify with live state file updates and runtime evidence.

### Phase 5 — Learning/search layer
- Add tool/routing decisions visible in HUD.
- The substrate learns usage patterns and exposes them as substrate state.
- No separate backend invisible to the operator.

## Out of Scope
- Launcher expansion beyond bootstrap/onboarding.
- Fleet-wide autonomy before visibility.
- Router/config rewrites that hide decisions from the HUD.
- Reintroducing legacy `annunimas-*` crate names as new first-class structure.
- Equal-surface designs — controlled heterogeneity is the point.
