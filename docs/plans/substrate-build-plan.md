# Arda Substrate Build Plan

> Arda is not a box. It is a biological substrate: heterogeneous nodes,
> each with different capability, learning and searching as a network.
> The HUD is the visible shell. Everything else grows outward from there.

## Identity

- Arda is the continuation of Annunimas
- `~/Annunimas` is the reference architecture
- The goal is an organized network, not a single-machine setup
- Nodes are not identical; they learn, specialize, and connect

## Node Philosophy

Treat each machine as a different cell/organ in the substrate:

| Node | Current role | Substrate role |
|---|---|---|
| Workstation (3080) | daily driver, mesh_root | central coordinator |
| Server PC | LFM2.5 / Gemma4 / Vision lanes | backbone compute |
| Beelink SER9 Pro | Qwen3.5-4B worker | sovereign worker |
| Pi5 #1 (warden) | guardhouse | edge guard |
| Pi5 #2 (citadel) | avatar/display | product surface |
| Laptop | voice/STT, intermittent | mobile/organic peripheral |

## Build Order

### Phase 0 — Single visible shell
- `apps/arda-hud` is the only operator surface
- Everything else is invisible until exposed through the HUD
- One machine: workstation only

### Phase 1 — Local manwe exposed in HUD
- HUD probes local manwe gateway
- Shows local models, status, health
- No network features yet

### Phase 2 — One tool/section at a time in HUD
- Each capability appears as a visible section/panel
- Tools are NOT hidden in code; they are expressed in the HUD
- Add one section, verify it works, then add the next

### Phase 3 — One node at a time
- Add curated nodes to the HUD
- Each node appears with its role, status, and active tools
- No auto-discovery; nodes are members of the substrate that the operator adds

### Phase 4 — Inference wiring
- Once a node is visible in HUD, wire its inference through manwe
- One lane at a time, visible as a route in the HUD
- Server lanes first, then edge, then mobile

### Phase 5 — Learning/search layer
- Add tool/routing decisions visible in HUD
- The substrate learns usage patterns and exposes them as substrate state
- Not a separate backend invisible to the operator

## Out of Scope
- Launcher expansion beyond HUD
- Fleet-wide autonomy before visibility
- Router rewrites that hide decisions from the HUD
- Equal-node designs — heterogeneity is the point
