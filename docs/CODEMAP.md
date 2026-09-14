---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-08"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-08-08

# Arda Codemap

Low-token entrypoint for the canonical Arda repository. Updated 2026-08-08
from `cargo metadata --no-deps --format-version 1` and live path inspection.
It navigates the Arda 1.0 personal agent ecosystem; Workbench and optional
capabilities are consumers of one governed kernel, not separate product roots.

## Top Level

| Directory | Purpose |
|-----------|---------|
| `crates/` | Rust engine and spine packages |
| `apps/` | Canonical HUD and launcher applications |
| `config/` | Operator-managed TOML/YAML/JSON configuration and generated runtime env files |
| `core/` | Realm, state, edge, project data read/written by services |
| `data/` | Runtime outputs, receipts, ledgers, state snapshots |
| `docs/` | 📜 Soterion documentation: human-facing design, migration, integration, and operations notes |
| `human/` | Human notes, plans, summaries, and Obsidian-style knowledge surfaces |
| `outposts/` | Bounded outpost protocol, scout, and read-only RELIC bridge packages |
| `sdk/` | Project-adapter SDKs |
| `vendor/` | Explicitly vendored dependency sources governed by workspace policy |
| `scripts/` | Operator scripts, bootstrap flows, system utilities, systemd unit sources |
| `tests/` | Cross-crate integration tests |
| `meta/` | Marketing, licensing, registry-adjacent metadata |
| `spec/` | Protocol/format specifications |

Runtime/build/noisy directories commonly present locally: `.cache/`, `.tmp/`, `target-check/`, `logs/`, `tmp/`.

---

## Build

```bash
cargo build -p arda
cargo check --workspace --all-targets --all-features
```

Installed systemd units may execute previously built artifacts. A running unit
does not prove that the current source tree was rebuilt or deployed.

## System Status Pointers

- Canonical operator entry: `AGENTS.md`
- Root authority pointer: `ARDA_ROOT_PROTOCOL.md`
- Product doctrine: `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`
- Current baseline: `docs/releases/0.9/BASELINE.md`
- Completed improvement record: `docs/archive/2026-08-12-arda-0.9-baseline-and-improvement-plan.md`
- Current bounded status snapshot: `ARDA_SYSTEM_STATUS_REPORT.md`
- Task queue projection: `core/state/queue_summary.json`
- Task queue ledger: `core/projects/tasks/queue.jsonl` (append-only evidence; avoid broad reads)
- Runtime truth surfaces: `core/state/`, `data/`

---

## Primary Entry Points

- `Cargo.toml` — 18-member workspace; root package and binary are `arda`
- `src/main.rs` — canonical composition root
- `crates/engine/src/lib.rs` — durable run/project execution and service supervision
- `crates/spine/governance/arda-core/src/lib.rs` — cross-subsystem contracts
- `crates/spine/runtime/manwe/src/lib.rs` — optional model/provider routing
- `crates/spine/interface/arda-orome/src/lib.rs` — communication semantics
- `crates/spine/memory/arda-vaire/src/lib.rs` — canonical long-term memory
- `crates/spine/observability/arda-aule/src/lib.rs` — observability and operator CLI surfaces
- `apps/arda-hud/src/` and `apps/arda-hud/src-tauri/` — canonical native HUD
- `apps/arda-launcher/src-tauri/` — launcher package included in the workspace

---

## Rust Package Map

`cargo metadata` resolves these 18 packages. There is no `arda-council`
package; council behavior is provided by current governance, Oromë, and Aulë
surfaces rather than a fabricated crate boundary.

| Package | Path | Current role |
|---|---|---|
| `arda` | `.` | canonical composition-root binary |
| `arda-engine` | `crates/engine` | durable project/run execution and supervision |
| `arda-contract-registry` | `crates/spine/contract/arda-contract-registry` | contract registration and lookup |
| `arda-core` | `crates/spine/governance/arda-core` | shared contracts and governance gates |
| `arda-governance` | `crates/spine/governance/arda-governance` | advisory governance and evidence evaluation |
| `arda-orome` | `crates/spine/interface/arda-orome` | communications semantics and transports |
| `arda-aule` | `crates/spine/observability/arda-aule` | observability, projections, and operator CLI |
| `arda-vaire` | `crates/spine/memory/arda-vaire` | canonical memory authority |
| `arda-varda` | `crates/spine/executors/arda-varda` | governed evidence ingest/query service |
| `arda-economics` | `crates/spine/runtime/arda-economics` | JouleWork/resource accounting |
| `arda-mandos` | `crates/spine/runtime/arda-mandos` | reasoning/verdict runtime |
| `arda-rumil` | `crates/spine/runtime/arda-rumil` | project-audit coordination |
| `manwe` | `crates/spine/runtime/manwe` | optional model/provider routing |
| `arda-launcher` | `apps/arda-launcher/src-tauri` | native launcher |
| `arda-outpost-protocol` | `outposts/arda-outpost-protocol` | bounded outpost contracts |
| `arda-outpost-scout` | `outposts/arda-outpost-scout` | advisory outpost research/survey worker |
| `arda-relic-bridge` | `outposts/arda-relic-bridge` | read-only presence projection bridge |
| `arda-project-adapter-sdk` | `sdk/rust` | external-project adapter SDK |

---

## Device and UI Surfaces

- `apps/arda-hud/` — canonical Tauri HUD; final visual acceptance is native
  Tauri, not browser preview.
- `apps/arda-launcher/` — canonical desktop launcher.
- `outposts/arda-relic-bridge/` — read-only runtime-presence bridge.
- External CITADEL/Mirromere consumers are optional and are not workspace
  package or base-release requirements.

---

## Config and State Hotspots

| File | Purpose |
|------|---------|
| `config/manwe.providers.toml` | Manwë provider/model routing input |
| `config/fleet.toml` | Fleet node metadata and inference endpoints |
| `config/routing/model_route_matrix.toml` | Cross-provider route policy input |
| `config/governance/autonomy_operating_loop.toml` | Draft operating-loop contract; not live proof |
| `config/monitoring-setup/` | Canonical Beelink Grafana/Prometheus bundle; no local Grafana tree |
| `core/knowledge/realm/arda.toml` | System identity and laws |
| `core/knowledge/realm/agents.toml` | Agent roster and authority |
| `core/knowledge/realm/boot.toml` | Boot order and baseline inputs |
| `core/state/` | Machine-readable runtime truth snapshots |
| `core/state/arda_source_map.json` | ARDA source/projection map |
| `core/state/system_source_map.json` | System source/projection map |
| `core/state/queue_summary.json` | Compact latest-by-id task queue projection for agents and HUD |
| `core/projects/tasks/queue.jsonl` | Active and historical append-only task queue ledger; use for exact evidence or appends |
| `data/prometheus/` | Supervisor/autopilot outputs, preflight snapshots, maintenance receipts |
| `data/hades/` | HADES queue/action/organization automation state |
| `data/` | Runtime outputs and receipts; inspect only for a task that needs live evidence |

---

## Common Task Entry Paths

**Change provider routing / models**
→ `config/manwe.providers.toml`, `config/routing/model_route_matrix.toml`, `crates/spine/runtime/manwe/`

**Change routing logic / scoring**
→ `crates/spine/runtime/manwe/src/`

**Change Hermes delivery/classification**
→ `crates/engine/src/adapters/hermes.rs`, `crates/spine/interface/arda-orome/`

**Change CLI behavior**
→ `src/main.rs`, `crates/spine/observability/arda-aule/src/cli/`

**Change ARDA HUD**
→ `apps/arda-hud/` (native Tauri acceptance required)

**Change supervisor / agent lifecycle**
→ `src/main.rs`, `crates/engine/src/`, `services.toml`

**Change autonomous operating loop / control plane**
→ Product doctrine: `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`
→ Portable config: `config/governance/autonomy_operating_loop.toml`
→ Runtime composition: `crates/engine/`; observability: `crates/spine/observability/arda-aule/`

**Change monitoring configuration**
→ `config/monitoring-setup/prometheus-central.yml`,
`config/monitoring-setup/prometheus-rules/`, and
`config/monitoring-setup/grafana-dashboards/`; re-probe the deployment before
claiming any endpoint is live.

---

## Traversal Advice

- Start here → `AGENTS.md` → crate README (if present) → exact entry file.
- All code and config files should include sigil headers where the local convention requires them (e.g. `# sigil: ANKH` for scripts, `# sigil: SCROLL` for config/docs).
- Documentation and generated documentation indexes should use the Soterion/SCROLL marker: YAML `soterion.sigil: "SCROLL"`, glyph `📜`, code point `U+1F4DC`, role `documentation` or `directory_index`; `FILE_TREE.jsonl` records use `"sigil":"📜"` and `"domain":"documentation"`.
- Restrict search to `crates/`, `apps/`, `config/`, `scripts/`, or `docs/operations/` before widening.
- Avoid broad searches over `data/`, `human/`, `archive/`, and runtime output directories unless you need runtime or knowledge context.
- Prefer each package's `README.md`, `STATUS.md`, `BREAKDOWN.md`, and `INDEX.md`
  when present, then verify claims in its public module graph and tests.
