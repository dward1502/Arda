---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Annunimas Codemap

Low-token entrypoint for navigating the repository. Updated 2026-05-26.

## Top Level

| Directory | Purpose |
|-----------|---------|
| `crates/` | Rust workspace and agent crates — 25 workspace members |
| `apps/` | UI and device-facing applications (ARDA HUD, CITADEL avatar) |
| `config/` | Operator-managed TOML/YAML/JSON configuration and generated runtime env files |
| `core/` | Realm, state, edge, project data read/written by services |
| `data/` | Runtime outputs, receipts, ledgers, state snapshots |
| `docs/` | 📜 Soterion documentation: human-facing design, migration, integration, and operations notes |
| `human/` | Human notes, plans, summaries, and Obsidian-style knowledge surfaces |
| `archive/` | Historical snapshots — usually not needed for current work |
| `archived_scripts/` | Historical scripts kept outside the active automation path |
| `audit/` | Audit reports and follow-up findings |
| `scripts/` | Operator scripts, bootstrap flows, system utilities, systemd unit sources |
| `tests/` | Cross-crate integration tests |
| `meta/` | Marketing, licensing, registry-adjacent metadata |
| `spec/` | Protocol/format specifications |

Runtime/build/noisy directories commonly present locally: `.cache/`, `.tmp/`, `target-check/`, `logs/`, `tmp/`.

---

## Build

```bash
source scripts/runtime_build_env.sh && cargo build -p annunimas-cli --release
```

> Binary goes to `~/.cache/annunimas-build/target/release/annunimas-cli` — this is what systemd services run, not `./target/`.

## System Status Pointers

- Canonical operator entry: `AGENTS.md`
- Root authority pointer: `ANNUNIMAS_ROOT_PROTOCOL.md`
- Current status snapshot: `ANNUNIMAS_SYSTEM_STATUS_REPORT.md`
- Task queue projection: `core/state/queue_summary.json`
- Task queue ledger: `core/projects/tasks/queue.jsonl` (append-only evidence; avoid broad reads)
- Runtime truth surfaces: `core/state/`, `data/`

---

## Primary Entry Points

- `Cargo.toml` — workspace definition, default member is `annunimas-cli`
- `crates/annunimas-cli/src/main.rs` — top-level CLI and subcommand wiring
- `crates/annunimas-charon/src/service.rs` — provider state, health, proxy logic
- `crates/annunimas-charon/src/service/route_policy.rs` — routing decisions, provider/model filtering
- `crates/annunimas-hermes/src/service.rs` — comms queueing, delivery, CHARON routing
- `crates/annunimas-prometheus/src/pipeline.rs` — orchestration pipeline
- `crates/annunimas-systemd/src/lib.rs` — typed `systemctl --user` client consumed by Prometheus/autopilot; supervision policy stays in `annunimas-prometheus`, executor stays as systemd + `scripts/agent_supervisor.sh`
- `crates/annunimas-hades/src/service/organization.rs` — HADES organization/audit automation surface
- `${HOME}/Eregion/arda-hud/src/` — ARDA HUD frontend modules
- `${HOME}/Eregion/arda-hud/src-tauri/` — ARDA HUD Tauri shell, excluded from the root Rust workspace

---

## Rust Crate Map

Workspace members in `Cargo.toml`:

| Crate | Role |
|-------|------|
| `annunimas-cli` | Operator entrypoint, daemon launcher, command surfaces |
| `annunimas-core` | Shared primitives: Task, Agent, Ledger, Router, contracts |
| `annunimas-ceo` | CEO/autopilot loop scaffolding |
| `annunimas-prometheus` | Orchestration pipeline, supervisor/autopilot policy |
| `annunimas-mnemosyne` | Memory continuity, recall, consolidation |
| `annunimas-charon` | LLM provider routing, health tracking, streaming proxy |
| `annunimas-athena` | Knowledge ingestion, triage, digest/query surfaces |
| `annunimas-hades` | Lifecycle, queue/sweep cleanup, organization automation |
| `annunimas-governance` | Triad, Resonance, Game-Theory primitives |
| `annunimas-warden` | Monitoring, alerts, guardhouse/security posture |
| `annunimas-comm` | Communications primitives shared by Hermes/edge bridges |
| `annunimas-mcp` | MCP server bridge surface |
| `annunimas-hermes` | Comms routing, outbound delivery, Discord/A2A/edge provider orchestration |
| `annunimas-oracle` | Reasoning and triad validation |
| `annunimas-plutus` | JouleWork accounting and economics |
| `annunimas-apollo` | Workflow execution surface |
| `annunimas-chronos` | Temporal workflow orchestration, predictive maintenance, continuous audit automation |
| `annunimas-tool-harness` | Shared tool execution harness |
| `annunimas-service-registry` | Service endpoint registry |
| `annunimas-council` | Multi-agent boardroom/council deliberation |
| `annunimas-forge-mind` | Code/build agent integration |
| `annunimas-signal-grid` | Cross-agent signal/event mesh |
| `annunimas-fleet` | Fleet/topology coordination and node telemetry |
| `annunimas-systemd` | Thin typed `systemctl --user` client |
| `annunimas-human` | Human knowledge/vault interface and Mnemosyne bridge |

Chronos is included above as a current workspace member.

---

## Device and UI Surfaces

- `/var/home/mythos/Eregion/arda-hud/` — Tauri HUD application surface; final validation is native Tauri, not host Vite-only preview
- `/var/home/mythos/Eregion/citadel-avatar/` — Pi5 kiosk/avatar display private consumer

ARDA HUD native validation path:

```bash
distrobox enter lothlorien -- bash -lc 'cd /var/home/mythos/Eregion/arda-hud && npm run tauri:dev:stable'
distrobox enter lothlorien -- bash -lc 'cd /var/home/mythos/Eregion/arda-hud && npm run tauri:build:stable'
distrobox enter lothlorien -- bash -lc 'cd /var/home/mythos/Annunimas && scripts/launch_arda_hud.sh'
```

---

## Config and State Hotspots

| File | Purpose |
|------|---------|
| `config/charon.providers.toml` | Provider pool, models, rate limits |
| `config/litellm.proxy.yaml` | LiteLLM proxy config (Anthropic bridge at :4000) |
| `config/fleet.toml` | Fleet node metadata and inference endpoints |
| `config/model_route_matrix.toml` | Cross-provider model routing matrix |
| `config/llm_model_routes.json` | Per-task model routing |
| `config/system_constitution.yaml` | System-wide constitutional rules |
| `config/topology_registry.yaml` | Node/topology registration |
| `config/warden.toml` | Warden monitoring config |
| `config/monitoring-setup/` | Canonical Beelink Grafana/Prometheus bundle; no local Grafana tree |
| `apps/arda-hud/arda_hud.settings.json` | ARDA HUD settings |
| `config/default.toml` | System defaults |
| `config/llm.toml` | LLM provider config |
| `config/runtime.generated.env` | Generated runtime environment snapshot |
| `core/realm/annunimas.toml` | System identity and laws |
| `core/realm/agents.toml` | Agent roster and authority |
| `core/realm/boot.toml` | Boot order, ARDA paths, JouleWork baseline |
| `core/state/` | Machine-readable runtime truth snapshots |
| `core/state/arda_source_map.json` | ARDA source/projection map |
| `core/state/system_source_map.json` | System source/projection map |
| `core/state/queue_summary.json` | Compact latest-by-id task queue projection for agents and HUD |
| `core/projects/tasks/queue.jsonl` | Active and historical append-only task queue ledger; use for exact evidence or appends |
| `data/prometheus/` | Supervisor/autopilot outputs, preflight snapshots, maintenance receipts |
| `data/hades/` | HADES queue/action/organization automation state |
| `data/charon/` | Router state, bandit/lane fitness, governance events |
| `data/mnemosyne/` | Episodic memory and chain state |
| `data/knowledge/athena/` | Athena knowledge index and source registry |

---

## Common Task Entry Paths

**Change provider routing / models**
→ `config/charon.providers.toml`, `config/model_route_matrix.toml`, `crates/annunimas-charon/src/service/route_policy.rs`
→ Reload live: `cargo run -p annunimas-cli -- charon reload-config` or POST the operator-local Charon reload endpoint when explicitly validating a private deployment

**Change routing logic / scoring**
→ `crates/annunimas-charon/src/service/route_policy.rs`
→ `crates/annunimas-charon/src/service.rs`

**Change Hermes delivery/classification**
→ `crates/annunimas-hermes/src/service.rs`, `src/router.rs`, `src/provider.rs`, `src/transport/`

**Change CLI behavior**
→ `crates/annunimas-cli/src/main.rs`, `src/commands/`, `src/export_surface/`

**Change CITADEL display**
→ `/var/home/mythos/Eregion/citadel-avatar/index.html`, `metatron-cube.js`, `run.sh`

**Change ARDA HUD**
→ `/var/home/mythos/Eregion/arda-hud/` (Tauri); settings: `apps/arda-hud/arda_hud.settings.json`

**Change supervisor / agent lifecycle**
→ `scripts/agent_supervisor.sh` (executor); `annunimas-prometheus` (policy + autopilot service-health); `annunimas-systemd` (typed query client)

**Change autonomous operating loop / control plane**
→ Canonical contract: `docs/contracts/autonomous-operating-loop-contract.md`
→ Portable config: `config/autonomy_operating_loop.toml`
→ Engine: `docs/operations/CEO_AUTOPILOT.md`, `crates/annunimas-prometheus/src/autopilot/`
→ Subordinate lanes: HADES lifecycle, ATHENA source ledgers, Council discussion, Oracle validation, Plutus JouleWork, Hermes confirmation

**Change HADES organization automation**
→ `crates/annunimas-hades/src/service/organization.rs`, `scripts/hades_organization_maintenance.sh`, `docs/operations/HADES_ORGANIZATION_AUTOMATION_PLAN.md`

**Change human knowledge ingestion**
→ `crates/annunimas-human/`, `human/`, `human_ingest_inventory.json`, `ingest_human_notes.py`

**Change Beelink monitoring / Grafana dashboards**
→ `config/monitoring-setup/prometheus-central.yml`, `config/monitoring-setup/prometheus-rules/`, `config/monitoring-setup/grafana-dashboards/`
→ Live services are on Beelink: Grafana `100.103.125.88:3000`, Prometheus `100.103.125.88:9090`; do not recreate local `config/grafana/` or `config/prometheus.yml`.

---

## Traversal Advice

- Start here → `AGENTS.md` → crate README (if present) → exact entry file.
- All code and config files should include sigil headers where the local convention requires them (e.g. `# sigil: ANKH` for scripts, `# sigil: SCROLL` for config/docs).
- Documentation and generated documentation indexes should use the Soterion/SCROLL marker: YAML `soterion.sigil: "SCROLL"`, glyph `📜`, code point `U+1F4DC`, role `documentation` or `directory_index`; `FILE_TREE.jsonl` records use `"sigil":"📜"` and `"domain":"documentation"`.
- Restrict search to `crates/`, `apps/`, `config/`, `scripts/`, or `docs/operations/` before widening.
- Avoid broad searches over `data/`, `human/`, `archive/`, and runtime output directories unless you need runtime or knowledge context.
- Each crate should have a `README.md` with deployment/ops context; `annunimas-human` is currently an exception, so inspect `src/lib.rs` directly.
- Crate-level index: `crates/INDEX.jsonl`, `crates/INDEX_TREE.md`.
