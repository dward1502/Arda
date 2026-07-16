# ARDA HUD — Incremental Build Plan

Goal: grow `apps/arda-hud` from the current minimal scaffold into the visible
operator surface for the Arda network, **one feature/tool at a time**.
No big-bang router rewrites. No launcher expansion until HUD is useful.

## Current baseline

- `apps/arda-hud/src/App.tsx` probes `http://127.0.0.1:7171/v1/models`
- Layout is intentionally minimal: status + model list + refresh
- No routing, no node map, no autonomy controls, no tool palette yet

## Constraint

- Do not touch `fleet.toml`, charon router rebuilds, or HUD launcher expansion
- Each step must be independently visible and runnable on one machine first
- Network features are added only after the local HUD basics are solid

## Execution order

### Step 1 — HUD as local manwe status pane
- [ ] Confirm `manwe` gateway reachable at `:7171` from HUD
- [ ] Show `/v1/models` + `/status`-style health in the existing pane
- [ ] Add one configurable gateway URL so the HUD is not hardcoded to localhost
- [ ] Verify on workstation before any Tailscale/node features

### Step 2 — Node presence panel
- [ ] Add a simple node list panel in HUD sourced from a local JSON file
- [ ] Start with one file: `config/hud_nodes.json`
- [ ] Show node name, Tailscale name/IP, status, role
- [ ] Do not auto-discover; nodes are curated so the UI learns the substrate slowly

### Step 3 — Tool palette: one tool at a time
- [ ] Add a tool launcher UI section in HUD
- [ ] Step 3a: model probe only — refresh models, show provider/model/context
- [ ] Step 3b: inference smoke test — send one canned prompt, show latency/tokens
- [ ] Step 3c: node reachability — ping/health each curated node from HUD
- [ ] Step 3d: routing decision view — show which node/manwe route a request used

### Step 4 — Network wiring
- [ ] Once Step 3 is stable, enable Beelink Qwen lane in manwe routing
- [ ] Expose that lane in HUD node panel as an active inference target
- [ ] Then server lanes, then Pi edges
- [ ] Each addition is one node, one UI row, one manwe route

### Step 5 — Operator controls
- [ ] Add runtime toggles in HUD for active lane priority
- [ ] Add one approval-gated action first: restart a known service via SSH jump
- [ ] Expand controls only after the read pane is trusted

## Exit criteria per step
- Step 1: HUD shows manwe status/models without launcher changes
- Step 2: node panel curated from JSON, no broken auto-discovery
- Step 3: one tool/section visible and working at a time
- Step 4: one new inference lane visible in HUD before adding the next
- Step 5: controls are optional and approval-gated by default

## Out of scope for this plan
- Launcher expansion
- Fleet-wide autonomy
- Router rewrite / charon replacement
- HUD-as-native-Tauri proof until the React surface is solid
