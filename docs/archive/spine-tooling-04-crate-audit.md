# Section 4 — Vendored Crate Audit (keep / shrink / delete)

VERIFIED 2026-07-13 against `crates/old-annunimas/*` (26 crates). Evidence:
`lib.rs` pub-surface + submodule tree, real LoC (src, non-test), and reverse
path-deps computed from the full `crates/` tree (NOT the matrix's layer labels).
The matrix in `*-01-disposition-matrix.md` used reference-architecture layer
paths; this file uses on-disk reality and corrects two of its claims.

Silmarillion target-name legend (from ./REFACTOR_PLAN, prefix corrected to arda-*):
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

## arda-core
[5527 LoC, 21 deps, path-deps 1, workspace=true]
- What it does: foundational types/contracts/ledger/tasks/messages/tools/governance gates; older daemon/loop orchestration stubs still live here but are planned to move to `arda-prometheus`.
- Evidence: current `src/` contains 23 pub modules, including core domain (`agent`, `ledger`, `contract`, `task`, `message`, `state`, `tool`, `router`, `pipeline`) plus orchestration stubs (`daemon`, `loop_engine`, `loop_alerts`, `loop_economy`, `learning`, `background`, `soterion_watcher`).
- Disposition: **KEEP / SHRINK → `arda-core`**. Keep only the type/contract/ledger core plus minimal runtime glue. Move daemon/loop orchestration/residency code into `arda-prometheus` when that crate’s architecture is finalized.
- Note: `arda-prometheus` already exists in the workspace, but its receiver API should be finalized before this crate re-homes orchestration behavior there.
- Blast radius: high. Finalize `arda-prometheus` ownership/receiver API before broadening consumers.
- Silmarillion: none (keep existing `arda-core` crate; `arda-prometheus` will absorb the orchestration pieces from this crate).

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
- Disposition: **SHRINK → `arda-mithril -demand lib). The economy
  loop is not needed resident for one box; keep lib + thin meter.
- Silmarillion: **arda-economics** (new).

### arda-service-registry  [188 LoC, 0 rev-deps] COMPLETED
- What it does: foundational service registry / contract / crate_identity.
- Evidence: 4 pub items (contract, registry, service, crate_identity). 0 users.
- Disposition: **KEEP (thin) → fold into `arda-core`**. Small; absorb rather
  than stand alone. Zero dependents = safe to move now.
- 
- folds into `arda-core`.

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

### arda-hermes  [14776 LoC, 2 rev-deps, path-deps 7]
- What it does: comms bridge — discord bot, mcp, relay, context_enrichment,
  router, slash commands, serenity_bot, provider.
- Evidence: 22 pub mods. The largest crate; only 2 rev-deps (prometheus, cli).
- Disposition: **MERGE → `orome`** (comms bridge, resident). Keep transport +
  relay + protocol + mcp; DROP the Discord/serenity-specific bot surface (not
  needed for local single-box). Largest shrink candidate.
- Silmarillion: **orome**.

### arda-mcp  [1339 LoC, 1 rev-dep (hermes)]
- What it does: MCP server/protocol/browser/external_sources/tools.
- Evidence: 5 pub mods. Only used by hermes.
- Disposition: **MERGE → `orome`** once hermes folds. MCP exposure rides the
  comms bridge. Drop as standalone.
- Silmarillion: folds into **orome**.

### arda-comm  [397 LoC, 1 rev-dep (prometheus)]
- What it does: Agent-to-Human message schema (A2HMessage, Priority, Channel,
  MessageQueue, governance metadata).
- Evidence: 16 pub items — a clean, small protocol crate.
- Disposition: **KEEP (thin) → fold into `orome`** as the A2H protocol types.
  Small and clean; absorb, don't stand alone.
- Silmarillion: folds into **orome**.

## Orchestration / agents / utility — delete after extraction

### arda-prometheus  [25955 LoC, 2 rev-deps, path-deps 11]
- What it does: CEO orchestration — autopilot, planner, pipeline, council,
  heartbeat, orders, thought, core_link, registry, router.
- Evidence: 13 pub mods. 26k LoC; depends on 11 crates. AULË target but the
  orchestration *brain* does not need to be resident for one box.
- Disposition: **SHRINK → `aulë`** (orchestration LIB, on-demand, last). Extract
  planner + pipeline + council + autopilot; delete the IPC/heartbeat daemon
  surface and the resident supervisor. Biggest extraction job — run LAST.
- Silmarillion: **aulë** (L5, last).

### arda-ceo  [421 LoC, 0 rev-deps]
- What it does: NOTHING — `pub use arda_prometheus::*;` shim (REPAIR sigil).
- Evidence: lib.rs 7 lines, confirmed re-export. Canonical impl in prometheus.
- Disposition: **DELETE**. Dead backward-compat shim. Matrix claimed "delete"
  — confirmed, with file evidence.
- Silmarillion: none (deleted).

### arda-athena  [10218 LoC, 2 rev-deps, path-deps 4]
- What it does: ingest agent — human, ingest, learning, transport; AthenaAgent.
- Evidence: 7 pub mods. VARDA ingest target.
- Disposition: **SHRINK → `varda`** (ingest lib, on-demand). Keep ingest +
  learning + human; drop the agent/transport daemon shell.
- Silmarillion: **varda** (L2, after manwe).

### arda-mandos  [migrated from annunimas-oracle, real lib]
- What it does: reasoning — context, reasoning, scoring, pageindex, notify,
  service, transport.
- Evidence: real implementation copied from `~/Annunimas/crates/annunimas-oracle`
  and retargeted to Arda crate names (`arda_core`, `arda_governance`,
  `arda_plutus`). Exposes `OracleEngine`, `OracleQuery`, `OracleService`,
  verdict types, pageindex, notifier.
- Disposition: **KEEP as `arda-mandos` lib**. Migration complete.
- Silmarillion: **mandos** (L3, after varda).

### arda-human  [folded → `arda-varda`]
- What it does: human knowledge/note vault — HumanNote, MemoryStore, scan_vault.
- Evidence: 8 pub items. 0 rev-deps in this workspace.
- Disposition: **FOLDED into `arda-varda`**. The human-knowledge surface now
  lives under `arda-varda`'s ingest/human module and is no longer a separate
  workspace member.
- Silmarillion: none (absorbed by **varda**).

### arda-mandos  [migrated from annunimas-oracle, real lib]
- What it does: reasoning — context, reasoning, scoring, pageindex, notify,
  service, transport.
- Evidence: real implementation copied from `~/Annunimas/crates/annunimas-oracle`
  and retargeted to Arda crate names. `arda-mandos` now builds as a real lib.
- Disposition: **KEEP as `arda-mandos` lib**. Migration complete.
- Silmarillion: **mandos** (L3, after varda).

### arda-apollo  [2179 LoC, 2 rev-deps, path-deps 4]
- What it does: workflow executor / RTK / phi / transport / service.
- Evidence: 6 pub mods. Used by prometheus + cli.
- Current state: `arda-apollo` crate already exposes `ApolloService` and
  `ApolloDaemon/ApolloDaemonConfig`; `cli` commands still use
  `apollo_call_or_local()` and the `start` branch still launches
  `ApolloDaemon`. `prometheus` autopilot already prefers the in-process
  `ApolloClient::InProcess` path when no daemon socket exists.
- Disposition: **KEEP as lib for now; tighten audit to lib-only**.
  The daemon/IPC retargeting is blocked until `cli` switches to direct
  `ApolloService::*` calls. Do not attempt transport removal until
  that consumer path is updated.
- Silmarillion: folds into **aulë**.

## Utility crates — delete after extraction (no Silmarillion target)

### arda-cli  [52235 LoC, 0 rev-deps]
- What it does: monolithic fleet CLI binary (src/main.rs, 110KB). Pulls every
  crate as a path-dep to expose one giant command surface.
- Evidence: bin crate (no lib), 52k LoC, 0 dependents. The "fleet" surface.
- Disposition: **DELETE**. The Silmarillion design drops the fleet CLI; the
  `arda` daemon (crates/engine + src/main.rs) replaces its useful entry points.
- Silmarillion: none (deleted).

### arda-chronos  [2173 LoC, 1 rev-dep (cli)]
- What it does: scheduling/clock agent — ChronosAgent::run/initialize.
- Evidence: 4 pub items. Only used by cli (fleet scheduler).
- Disposition: **DELETE** (fleet scheduler not needed resident for one box).
  Replaced by `arda` daemon's process supervision loop if any scheduling needed.
- Silmarillion: none (deleted).

### arda-apollo  [2179 LoC, 2 rev-deps, path-deps 4]
- What it does: workflow executor / RTK / phi / transport / service.
- Evidence: 6 pub mods. Used by prometheus + cli.
- Disposition: **DELETE / MERGE → `aulë`**. Workflow execution is orchestration;
  fold the workflow/executor core into aulë when aulë is built. Drop standalone.
- Silmarillion: folds into **aulë**.

### arda-hades  [8177 LoC, 2 rev-deps, path-deps 4]
- What it does: dead-letter / persistence agent — agent, error, service,
  transport, types.
- Evidence: 5 pub mods. Used by cli + hermes.
- Disposition: **DELETE** (confirmed 2026-07-13: user will remake later). Do not
  extract. Persistence/DLQ store is out of scope for now.
- Silmarillion: none (deleted; remade later)

### arda-warden  [1541 LoC, 3 rev-deps, path-deps 4]
- What it does: security monitor — alerts, crypto, foreign, monitor, podman,
  scoring. Also has a tiny src/main.rs (1116B).
- Evidence: 7 pub mods. Used by apollo, cli, hermes.
- Disposition: **DEFERRED**. Active dependency remains from `arda-aule`:
  `crates/spine/interface/arda-aule/src/executor.rs` imports `evaluate_execution_harness` directly.
  Must remove that consumer/lib path dependency before crate deletion.
- Silmarillion: none yet.

### arda-fleet  [1492 LoC, 1 rev-dep (prometheus)]
- What it does: multi-node fleet routing — FleetNode, capacity manager,
  select_best_node, local-node detection.
- Evidence: 23 pub items. ONLY used by prometheus.
- Disposition: **DELETE**. Explicitly out of scope: "drop the fleet". The
  single-box design has no fleet. (Matrix listed it keep-with-notes — REJECTED:
  it is pure fleet routing with one user.)
- Silmarillion: none (deleted).

### arda-forge-mind  [3195 LoC, 1 rev-dep (cli)]
- What it does: 3D asset forge — blender, slicer, forge, tools, workflow.
- Evidence: 7 pub mods. Only used by cli.
- Disposition: **DELETE**. Asset-forge tooling is out of scope for the local
  Silmarillion runtime. (Matrix listed keep-with-notes — REJECTED: single user
  is the fleet CLI, zero tie to the resident path.)
- Silmarillion: none (deleted).

### arda-onboarding  [port complete → launcher app]
- What it does: interactive console onboarding — device_scan, service_plan,
  private_config, guided session, 13 private modules.
- Evidence: real lib (1054B facade + 13 submodules). Only used by cli.
- Disposition: **PORTED → `arda` launcher app**. Onboarding is now in
  `apps/arda-launcher/src-tauri/src/onboarding/` and removed from the Arda
  workspace. It does NOT stay under `crates/spine/interface`.
- Silmarillion: none (moved to arda-launcher app).

### arda-signal-grid  [284 LoC, 0 rev-deps]  COMPLETED
- What it does: ANKH blueprint — defines a *projection surface* for signal
  routing, not live transport. contract + pipeline + service + crate_identity.
- Evidence: 284 LoC, 0 rev-deps. Blueprint surface only (self-described).
- Disposition: **DELETE**. Matrix listed it keep-with-notes; REJECTED — it is a
  284-LoC speculative blueprint with zero dependents and no live transport.
- Silmarillion: none (deleted).

### arda-systemd  [195 LoC, 1 rev-dep (prometheus)]
- What it does: typed `systemctl --user` client — Unit, UnitKind, parse_list_units.
- Evidence: 7 pub items. Clean, small, useful.
- Disposition: **KEEP (thin) → fold into  `arda-core`.
  The `arda` daemon already supervises processes; a systemctl client is handy for
  it. Small = absorb, don't stand alone.
- Arda: folds into **arda-core**.

### arda-council  [179 LoC, 1 rev-dep (prometheus)]
- What it does: council contract/service + crate_identity.
- Evidence: 4 pub items. Used by prometheus.
- Disposition: **MERGE → `aulë`**. Council logic is orchestration; absorb into
  aulë when built. Drop standalone.
- Silmarillion: folds into **arda-aule**. Content merged into
  `crates/spine/observability/arda-aule`; `arda-council` standalone crate removed.

### arda-tool-harness  [373 LoC, 2 rev-deps (core, forge-mind)]
- What it does: tool contract/service/types + crate_identity.
- Evidence: 4 pub items. Used by core + forge-mind.
- Disposition: **KEEP (thin) → fold into `arda-core`** as the tool-contract
  types. Small; forge-mind is deleted so only core uses it. Absorb.
- Silmarillion: folds into **arda-core**.

## Summary table

| crate | LoC | rev-deps | disposition | Silmarillion |
|-------|----:|---------:|-------------|--------------|
| arda-core | 5527 | 18 | SHRINK→lib | arda-core |
| arda-governance | 3873 | 14 | SHRINK→lib | arda-governance |
| arda-vaire | 3143 | 8 | SHRINK→lib | vaire |
|| arda-economics | live | 11 | demand + economy/JW lib | owns plutus boundary; replaces `arda-plutus` |
|| arda-service-registry | 188 | 0 | fold | → arda-core |
|| arda-charon | 5047 | 3 | DELETE (cascade) | — (live `crates/spine/runtime/manwe`) |
|| arda-hermes | 14776 | 2 | MERGE | orome |
|| arda-mcp | 1339 | 1 | MERGE | → orome ||
|| arda-comm | 397 | 1 | fold | → orome ||
|| arda-prometheus | 25955 | 2 | SHRINK (last) | aulë (L5) ||
|| arda-council | 179 | 1 | MERGE | → arda-aule ||
|| arda-tool-harness | 373 | 2 | fold | → arda-core ||
|| arda-athena | 10218 | 2 | SHRINK | varda (L2) ||
|| arda-mandos | 1493 | 3 | KEEP-on-demand | mandos (L3) ||
|| arda-human | 240 | 0 | fold | → varda ||
|| arda-cli | 52235 | 0 | **DELETE** (fleet) | — ||
|| arda-chronos | 2173 | 1 | **DELETE** | — ||
|| arda-apollo | 2179 | 2 | KEEP-lib-for-now / tighten to lib | apollo demand entry; merge to `aulë` later when consumer path matures ||
|| arda-hades | 8177 | 2 | **DELETE** (remake later) | — ||
|| arda-warden | 1541 | 3 | DEFERRED / blocked | requires `arda-aule` consumer removal before delete ||
|| arda-fleet | 1492 | 1 | **DELETE** (fleet) | — ||
|| arda-forge-mind | 3195 | 1 | **DELETE** | — ||
|| arda-onboarding | 3244 | 1 | → launcher app | (arda-launcher) ||
|| arda-signal-grid | 284 | 0 | **DELETE** (blueprint) | — ||
|| arda-systemd | 195 | 1 | fold | → arda/arda-core ||

## Corrections to disposition-matrix (file evidence)
1. annunimas-ceo: matrix said "delete" ✓ but gave no evidence — CONFIRMED it is a
   7-line `pub use annunimas_prometheus::*` shim (REPAIR sigil). Safe delete.
2. annunimas-signal-grid: matrix said "keep (with notes)" — REJECTED. On-disk it is
   a 284-LoC ANKH blueprint surface with 0 rev-deps and no live transport. DELETE.
3. annunimas-fleet / annunimas-forge-mind: matrix said "keep (with notes)" — REJECTED
   on scope ("drop the fleet"). fleet has 1 user (prometheus) and is pure multi-node
   routing; forge-mind has 1 user (cli) and is 3D asset tooling. Both DELETE.


- [x] arda-hermes — merged into `arda-orome`
- [x] arda-apollo — merge into `aulë`
- [x] arda-athena — rename/move to `arda-varda`
- [x] arda-mandos — keep as lib `arda-mandos`
- [x] arda-chronos — delete
- [x] arda-hades — delete / remake later
- [x] arda-warden — delete
- [x] arda-fleet — delete
- [x] arda-forge-mind — delete
- [x] arda-onboarding — port to launcher app
- [x] arda-cli — deleted
- [x] arda-ceo — delete
- [x] arda-prometheus — shrink into `aulë` (last)
- [ ] Substrate libs → library-first refactor
- [ ] Directory renames to `arda-*` across crates

## Execution order (per refactor plan)
- L1 varda (ex-athena) — ingest lib  [was manwe; charon is DELETE, live
  `crates/manwe` is already the gateway and needs no extraction]
- L2 mandos (ex-oracle) — reasoning lib
- L4 vaire (ex-mnemosyne) — memory lib
- L5 aulë (ex-prometheus+ceo) — orchestration lib, LAST
PLUS substrate (arda-core / governance / economics) converted earliest because
18/14/11 crates depend on them. Thin folds (service-registry, comm, human, systemd,
council, tool-harness) absorbed into core/launcher as their target is built.
