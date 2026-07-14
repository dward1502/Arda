# Section 4 — Vendored Crate Audit (keep / shrink / delete)

VERIFIED 2026-07-13 against `crates/old-annunimas/*` (26 crates). Evidence:
`lib.rs` pub-surface + submodule tree, real LoC (src, non-test), and reverse
path-deps computed from the full `crates/` tree (NOT the matrix's layer labels).
The matrix in `*-01-disposition-matrix.md` used reference-architecture layer
paths; this file uses on-disk reality and corrects two of its claims.

Silmarillion target-name legend (from REFACTOR_PLAN, prefix corrected to arda-*):
  arda-varda = ingest (ex-athena)   arda-mandos = reasoning (ex-oracle)
  arda-vaire = memory (ex-mnemosyne) arda-orome = comms (ex-hermes)
  arda-aule = orchestration (ex-prometheus+ceo, last)
  NOTE: `crates/manwe` is the LIVE gateway binary (127.0.0.1:7171) — it is
  NOT produced by this audit; charon is DELETE (superseded by it).
  arda-core / arda-governance / arda-economics = substrate libs

CONSTRAINTS (user, 2026-07-13):
- Prefix is `arda-*` NOT `silmaril-*`.
- NO monolithic files: proper Rust module decomposition, one concern per file.
- annunimas-hades: DELETE (will be remade later) — not extracted.
- annunimas-governance: KEEP-ALL, do NOT trim (love_dynamics/solar/vision stay;
  user will evolve governance later). Port as-is.

## Substrate — convert to library FIRST (widest blast radius)

### annunimas-core  [5527 LoC, 18 rev-deps, path-deps 1]
- What it does: shared types/ledger/task/contract/daemon/governance/llm/router/
  loop_engine/soterion. Foundational substrate.
- Evidence: 23 pub mods (agent, ledger, contract, governance, llm, router,
  loop_engine, soterion, task, message, pipeline, state, tool, ...).
- Disposition: **KEEP / SHRINK → `arda-core`**. Keep the type/contract/ledger
  core; trim daemon/loop orchestration that belongs to aulë. Highest blast
  radius — convert before anything depends on it as a lib.
- Silmarillion: none (new core crate `arda-core`).

### annunimas-governance  [3873 LoC, 14 rev-deps, path-deps 1]
- What it does: triad validation, game_theory, love_equation, resonance,
  readiness, bacon_lite, solar, vision, philosopher_profiles.
- Evidence: 13 pub mods. Used by athena, mnemosyne, prometheus, charon, hades,
  oracle, plutus.
- Disposition: **KEEP / SHRINK → `arda-governance`**. Port as-is (do NOT trim:
  love_dynamics/solar/vision stay — user will evolve governance later).
- arda-governance: none (port as-is, full)

### annunimas-mnemosyne  [3143 LoC, 8 rev-deps, path-deps 3]
- What it does: memory store, consolidation, significance scoring, Obsidian
  sync, recall. Facade over `service::MnemosyneService`.
- Evidence: lib.rs 365B facade, but real impl in service/significance/transport.
  MnemosyneService, ConsolidationReport, RecallRecentEntry are real pub items.
- Disposition: **KEEP / SHRINK → `vaire`** (memory lib + on-demand store).
  Substrate for 8 crates — convert early, before dependents change.
- Silmarillion: **vaire**.

### annunimas-plutus  [2255 LoC, 11 rev-deps, path-deps 2]
- What it does: JW economics, joule_work, ledger, love_equation, meter.
- Evidence: 8 pub mods (economics, joule_work, ledger, love_equation, meter).
- Disposition: **SHRINK → `arda-economics`** (on-demand lib). The economy
  loop is not needed resident for one box; keep lib + thin meter.
- Silmarillion: **arda-economics** (new).

### annunimas-service-registry  [188 LoC, 0 rev-deps]
- What it does: foundational service registry / contract / crate_identity.
- Evidence: 4 pub items (contract, registry, service, crate_identity). 0 users.
- Disposition: **KEEP (thin) → fold into `arda-core`**. Small; absorb rather
  than stand alone. Zero dependents = safe to move now.
- Silmarillion: folds into `arda-core`.

## Resident / gateway / comms — the only crate(s) that stay resident

### annunimas-charon  [5047 LoC, 3 rev-deps (engine, hermes, prometheus, cli)]
- What it does: inference router — adaptive_routing, bandit, route_scoring,
  route_selection, route_sessions, proxy, health_probe, hermes_proxy_driver.
- Evidence: 22 pub mods. The old heavyweight router; superseded by the
  EXISTING LIVE gateway `crates/manwe` (binary, `127.0.0.1:7171`,
  `/v1/chat/completions`, supervised by `arda` daemon). The arda-native tree
  only `pub use annunimas_charon as charon;` in engine/lib.rs:10 — no live
  consumer. Dependents are hermes (→orome absorb), prometheus (DELETE),
  cli (DELETE) — i.e. charon dies with its dependents.
- Disposition: **DELETE (cascade)**. DO NOT extract to `crates/manwe`
  (that crate already exists and is the live gateway). Remove the engine
  re-export line once hermes/prometheus/cli are gone; then drop from
  workspace + delete the vendored dir.
- Silmarillion: none (replaced by live `crates/manwe`).

### annunimas-hermes  [14776 LoC, 2 rev-deps, path-deps 7]
- What it does: comms bridge — discord bot, mcp, relay, context_enrichment,
  router, slash commands, serenity_bot, provider.
- Evidence: 22 pub mods. The largest crate; only 2 rev-deps (prometheus, cli).
- Disposition: **MERGE → `orome`** (comms bridge, resident). Keep transport +
  relay + protocol + mcp; DROP the Discord/serenity-specific bot surface (not
  needed for local single-box). Largest shrink candidate.
- Silmarillion: **orome**.

### annunimas-mcp  [1339 LoC, 1 rev-dep (hermes)]
- What it does: MCP server/protocol/browser/external_sources/tools.
- Evidence: 5 pub mods. Only used by hermes.
- Disposition: **MERGE → `orome`** once hermes folds. MCP exposure rides the
  comms bridge. Drop as standalone.
- Silmarillion: folds into **orome**.

### annunimas-comm  [397 LoC, 1 rev-dep (prometheus)]
- What it does: Agent-to-Human message schema (A2HMessage, Priority, Channel,
  MessageQueue, governance metadata).
- Evidence: 16 pub items — a clean, small protocol crate.
- Disposition: **KEEP (thin) → fold into `orome`** as the A2H protocol types.
  Small and clean; absorb, don't stand alone.
- Silmarillion: folds into **orome**.

## Orchestration / agents / utility — delete after extraction

### annunimas-prometheus  [25955 LoC, 2 rev-deps, path-deps 11]
- What it does: CEO orchestration — autopilot, planner, pipeline, council,
  heartbeat, orders, thought, core_link, registry, router.
- Evidence: 13 pub mods. 26k LoC; depends on 11 crates. AULË target but the
  orchestration *brain* does not need to be resident for one box.
- Disposition: **SHRINK → `aulë`** (orchestration LIB, on-demand, last). Extract
  planner + pipeline + council + autopilot; delete the IPC/heartbeat daemon
  surface and the resident supervisor. Biggest extraction job — run LAST.
- Silmarillion: **aulë** (L5, last).

### annunimas-ceo  [421 LoC, 0 rev-deps]
- What it does: NOTHING — `pub use annunimas_prometheus::*;` shim (REPAIR sigil).
- Evidence: lib.rs 7 lines, confirmed re-export. Canonical impl in prometheus.
- Disposition: **DELETE**. Dead backward-compat shim. Matrix claimed "delete"
  — confirmed, with file evidence.
- Silmarillion: none (deleted).

### annunimas-athena  [10218 LoC, 2 rev-deps, path-deps 4]
- What it does: ingest agent — human, ingest, learning, transport; AthenaAgent.
- Evidence: 7 pub mods. VARDA ingest target.
- Disposition: **SHRINK → `varda`** (ingest lib, on-demand). Keep ingest +
  learning + human; drop the agent/transport daemon shell.
- Silmarillion: **varda** (L2, after manwe).

### annunimas-oracle  [1493 LoC, 3 rev-deps, path-deps 3]
- What it does: reasoning — context, reasoning, scoring, pageindex, notify.
- Evidence: 7 pub mods. MANDOS target.
- Disposition: **KEEP-on-demand → `mandos`** (reasoning lib). Part of the per-task
  reasoning chain; pull when a task needs inference.
- Silmarillion: **mandos** (L3, after varda).

### annunimas-human  [240 LoC, 0 rev-deps]
- What it does: human knowledge/note vault — HumanNote, MemoryStore, scan_vault.
- Evidence: 8 pub items. 0 rev-deps.
- Disposition: **KEEP (thin) → fold into `varda`** ingest path or `arda-core`.
  0 dependents = safe to move now. Move to varda's human-ingest module.
- Silmarillion: folds into **varda**.

## Utility crates — delete after extraction (no Silmarillion target)

### annunimas-cli  [52235 LoC, 0 rev-deps]
- What it does: monolithic fleet CLI binary (src/main.rs, 110KB). Pulls every
  crate as a path-dep to expose one giant command surface.
- Evidence: bin crate (no lib), 52k LoC, 0 dependents. The "fleet" surface.
- Disposition: **DELETE**. The Silmarillion design drops the fleet CLI; the
  `arda` daemon (crates/engine + src/main.rs) replaces its useful entry points.
- Silmarillion: none (deleted).

### annunimas-chronos  [2173 LoC, 1 rev-dep (cli)]
- What it does: scheduling/clock agent — ChronosAgent::run/initialize.
- Evidence: 4 pub items. Only used by cli (fleet scheduler).
- Disposition: **DELETE** (fleet scheduler not needed resident for one box).
  Replaced by `arda` daemon's process supervision loop if any scheduling needed.
- Silmarillion: none (deleted).

### annunimas-apollo  [2179 LoC, 2 rev-deps, path-deps 4]
- What it does: workflow executor / RTK / phi / transport / service.
- Evidence: 6 pub mods. Used by prometheus + cli.
- Disposition: **DELETE / MERGE → `aulë`**. Workflow execution is orchestration;
  fold the workflow/executor core into aulë when aulë is built. Drop standalone.
- Silmarillion: folds into **aulë**.

### annunimas-hades  [8177 LoC, 2 rev-deps, path-deps 4]
- What it does: dead-letter / persistence agent — agent, error, service,
  transport, types.
- Evidence: 5 pub mods. Used by cli + hermes.
- Disposition: **DELETE** (confirmed 2026-07-13: user will remake later). Do not
  extract. Persistence/DLQ store is out of scope for now.
- Silmarillion: none (deleted; remade later)

### annunimas-warden  [1541 LoC, 3 rev-deps, path-deps 4]
- What it does: security monitor — alerts, crypto, foreign, monitor, podman,
  scoring. Also has a tiny src/main.rs (1116B).
- Evidence: 7 pub mods. Used by apollo, cli, hermes.
- Disposition: **DELETE**. Podman/monitoring surface is fleet security, not
  needed resident. Crypto helpers → `arda-core` if reused.
- Silmarillion: crypto helpers → `arda-core` or delete.

### annunimas-fleet  [1492 LoC, 1 rev-dep (prometheus)]
- What it does: multi-node fleet routing — FleetNode, capacity manager,
  select_best_node, local-node detection.
- Evidence: 23 pub items. ONLY used by prometheus.
- Disposition: **DELETE**. Explicitly out of scope: "drop the fleet". The
  single-box design has no fleet. (Matrix listed it keep-with-notes — REJECTED:
  it is pure fleet routing with one user.)
- Silmarillion: none (deleted).

### annunimas-forge-mind  [3195 LoC, 1 rev-dep (cli)]
- What it does: 3D asset forge — blender, slicer, forge, tools, workflow.
- Evidence: 7 pub mods. Only used by cli.
- Disposition: **DELETE**. Asset-forge tooling is out of scope for the local
  Silmarillion runtime. (Matrix listed keep-with-notes — REJECTED: single user
  is the fleet CLI, zero tie to the resident path.)
- Silmarillion: none (deleted).

### annunimas-onboarding  [3244 LoC, 1 rev-dep (cli)]
- What it does: interactive console onboarding — device_scan, service_plan,
  private_config, guided session, 13 private modules.
- Evidence: real lib (1054B facade + 13 submodules). Only used by cli.
- Disposition: **KEEP (thin) → port to `arda` launcher app**. Onboarding is a
  one-time setup flow the launcher owns, not a resident crate. Move to the
  launcher app's Rust backend (apps/arda-launcher/src-tauri/src/), not Silmarillion.
  Does NOT stay under old-annunimas.
- Silmarillion: moves to arda-launcher app (not a Silmarillion crate).

### annunimas-signal-grid  [284 LoC, 0 rev-deps]
- What it does: ANKH blueprint — defines a *projection surface* for signal
  routing, not live transport. contract + pipeline + service + crate_identity.
- Evidence: 284 LoC, 0 rev-deps. Blueprint surface only (self-described).
- Disposition: **DELETE**. Matrix listed it keep-with-notes; REJECTED — it is a
  284-LoC speculative blueprint with zero dependents and no live transport.
- Silmarillion: none (deleted).

### annunimas-systemd  [195 LoC, 1 rev-dep (prometheus)]
- What it does: typed `systemctl --user` client — Unit, UnitKind, parse_list_units.
- Evidence: 7 pub items. Clean, small, useful.
- Disposition: **KEEP (thin) → fold into `arda` daemon** or `arda-core`.
  The `arda` daemon already supervises processes; a systemctl client is handy for
  it. Small = absorb, don't stand alone.
- Silmarillion: folds into **arda daemon / arda-core**.

### annunimas-council  [179 LoC, 1 rev-dep (prometheus)]
- What it does: council contract/service + crate_identity.
- Evidence: 4 pub items. Used by prometheus.
- Disposition: **MERGE → `aulë`**. Council logic is orchestration; absorb into
  aulë when built. Drop standalone.
- Silmarillion: folds into **aulë**.

### annunimas-tool-harness  [373 LoC, 2 rev-deps (core, forge-mind)]
- What it does: tool contract/service/types + crate_identity.
- Evidence: 4 pub items. Used by core + forge-mind.
- Disposition: **KEEP (thin) → fold into `arda-core`** as the tool-contract
  types. Small; forge-mind is deleted so only core uses it. Absorb.
- Silmarillion: folds into **arda-core**.

## Summary table

| crate | LoC | rev-deps | disposition | Silmarillion |
|-------|----:|---------:|-------------|--------------|
| annunimas-core | 5527 | 18 | SHRINK→lib | arda-core |
| annunimas-governance | 3873 | 14 | SHRINK→lib | arda-governance |
| annunimas-mnemosyne | 3143 | 8 | SHRINK→lib | vaire |
| annunimas-plutus | 2255 | 11 | SHRINK→lib | arda-economics |
| annunimas-service-registry | 188 | 0 | fold | → arda-core |
| annunimas-charon | 5047 | 3 | DELETE (cascade) | — (live `crates/manwe`) |
| annunimas-hermes | 14776 | 2 | MERGE | orome |
| annunimas-mcp | 1339 | 1 | MERGE | → orome |
| annunimas-comm | 397 | 1 | fold | → orome |
| annunimas-prometheus | 25955 | 2 | SHRINK (last) | aulë (L5) |
| annunimas-ceo | 421 | 0 | **DELETE** (shim) | — |
| annunimas-athena | 10218 | 2 | SHRINK | varda (L2) |
| annunimas-oracle | 1493 | 3 | KEEP-on-demand | mandos (L3) |
| annunimas-human | 240 | 0 | fold | → varda |
| annunimas-cli | 52235 | 0 | **DELETE** (fleet) | — |
| annunimas-chronos | 2173 | 1 | **DELETE** | — |
| annunimas-apollo | 2179 | 2 | MERGE | → aulë |
| annunimas-hades | 8177 | 2 | **DELETE** (remake later) | — |
| annunimas-warden | 1541 | 3 | **DELETE** | — |
| annunimas-fleet | 1492 | 1 | **DELETE** (fleet) | — |
| annunimas-forge-mind | 3195 | 1 | **DELETE** | — |
| annunimas-onboarding | 3244 | 1 | → launcher app | (arda-launcher) |
| annunimas-signal-grid | 284 | 0 | **DELETE** (blueprint) | — |
| annunimas-systemd | 195 | 1 | fold | → arda/arda-core |
| annunimas-council | 179 | 1 | MERGE | → aulë |
| annunimas-tool-harness | 373 | 2 | fold | → arda-core |

## Corrections to disposition-matrix (file evidence)
1. annunimas-ceo: matrix said "delete" ✓ but gave no evidence — CONFIRMED it is a
   7-line `pub use annunimas_prometheus::*` shim (REPAIR sigil). Safe delete.
2. annunimas-signal-grid: matrix said "keep (with notes)" — REJECTED. On-disk it is
   a 284-LoC ANKH blueprint surface with 0 rev-deps and no live transport. DELETE.
3. annunimas-fleet / annunimas-forge-mind: matrix said "keep (with notes)" — REJECTED
   on scope ("drop the fleet"). fleet has 1 user (prometheus) and is pure multi-node
   routing; forge-mind has 1 user (cli) and is 3D asset tooling. Both DELETE.

## Execution order (per refactor plan)
- L1 varda (ex-athena) — ingest lib  [was manwe; charon is DELETE, live
  `crates/manwe` is already the gateway and needs no extraction]
- L2 mandos (ex-oracle) — reasoning lib
- L4 vaire (ex-mnemosyne) — memory lib
- L5 aulë (ex-prometheus+ceo) — orchestration lib, LAST
PLUS substrate (arda-core / governance / economics) converted earliest because
18/14/11 crates depend on them. Thin folds (service-registry, comm, human, systemd,
council, tool-harness) absorbed into core/launcher as their target is built.
