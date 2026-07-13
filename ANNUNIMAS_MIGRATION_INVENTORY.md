# Annunimas → Arda Migration Inventory

Source: `/var/home/mythos/Annunimas/crates/` (26 real crates; the "30" count
included 4 index files). Evidence gathered directly from each crate's
`Cargo.toml` deps, `src/lib.rs` doc comments, and `src/` file tree — **not
guessed**. This is a *proposed* keep/drop/merge/rename matrix. Decisions are
yours; the recommendations below are flagged as such.

Legend: ✅ keep · ❌ drop · 🔀 merge-into · 📐 blueprint (not live yet) ·
🔁 rename (strip mythic/esoteric verbage).

---

## Valar Rebrand (agent identities)

User directive: keep the Rust language, rename the agent identities to Tolkien
Valar. This supersedes the earlier `arda-*` functional agent names. The 10 named
agents map to crates as follows (recorded; collisions resolved below):

| Agent (old) | Valar (new) | Crate (old → new) | Domain |
|---|---|---|---|
| Arandur / CEO | Manwë | annunimas-core → **manwe** | sovereign orchestrator / spine |
| Athena / Knowledge | Varda (Elentári) | annunimas-athena → **varda** | light & knowledge / memory keeper |
| Oracle / Governance | Mandos (Námo) | annunimas-oracle → **mandos** | fates & records / Triad judge |
| Plutus / Finance | Aulë | annunimas-plutus → **aule** | crafts / JouleWork & creation |
| Hermes / Messenger | Oromë | annunimas-hermes → **orome** | hunter / routing & signals |
| Warden / Monitor | Tulkas | annunimas-warden → **tulkas** | champion / guardian & defender |
| Mnemosyne / Memory | Vairë | annunimas-mnemosyne → **vaire** | weaver / episodic memory |
| Chronos / Temporal | Irmo (Lórien) | annunimas-chronos → **lorien** | visions of time / scheduling |
| Hades / Cleanup | Nienna | annunimas-hades → **nienna** | mercy in endings / lifecycle |
| Apollo / Executor | (merged) | annunimas-apollo → merge into prometheus | — |

Collision resolution (proposed, confirm if you disagree):
- Aulë was assigned to both Plutus and Apollo. **Apollo merges into prometheus**
  (decided earlier), so Aulë → **aule** (Plutus/finance). No separate Aulë crate.
- Mandos was assigned to both Oracle and Hades. **Oracle → mandos** (governance/
  judge); **Hades → nienna** (mercy in endings). Clean split.

Notes / open:
- Infra/support crates NOT in the Valar table (onboarding, systemd, tool-gate,
  fleet, mcp, comm, charon, cli, prometheus) keep functional `arda-` names for
  now — confirm if you want them Valar-ified too.
- CEO = Manwë maps to the core spine (annunimas-core). Prometheus (executive
  orchestration) is unnamed in the table; propose it stays `arda-prometheus`
  unless you assign a Valar name.
- Config paths `~/.config/annunimas/` sanitize to `~/.config/arda/` (project-level,
  not agent-level) — correct regardless of Valar agent names.

---

## Tier 1 — Core spine (keep, rename, sanitize verbage)

| Crate | What it actually does (evidence) | Rec | New name |
|---|---|---|---|
| annunimas-core | agent, alerts, daemon, message, pipeline, router, tool, ledger, aipkg, background, config, contract/*, governance/*. The foundational spine. | ✅ keep | arda-core (exists) |
| annunimas-cli | Orchestration CLI: commands for apollo, athena, charon, forge, hades, hermes, loop, metrics, mnemosyne, oracle, plutus, prometheus, state… | ✅ keep | arda-cli |
| annunimas-charon | Provider routing: adaptive_routing, bandit, agent_quotas, route_scoring, route_selection, runtime_state, hermes_proxy_driver. LLM routing core. | ✅ keep | arda-charon |
| annunimas-athena | Knowledge ingestion: scholarly, crawl, github, deep, extraction, index, views, uncertainty_sampler, remediation, routing. | ✅ keep | arda-athena |
| annunimas-mnemosyne | Memory: store, retrieval, promotion, significance, transport http/ipc. | ✅ keep | arda-mnemosyne |
| annunimas-hermes | Inter-agent (A2A) + external (email/slack/discord MCP), slash, relay, router, intent, formatter. | ✅ keep | arda-hermes |
| annunimas-oracle | Truth-confidence scoring for the learning loop, reasoning, context, notify, pageindex. | ✅ keep | arda-oracle |
| annunimas-warden | Runtime monitoring/security: podman, crypto, alerts, schema_drift_detector, runaway_loop_detector, foreign. | ✅ keep | arda-warden |
| annunimas-prometheus | Executive orchestration/autonomy: pipeline (fleet_routing, preflight, local_execution), core_link (fleet, governance_runtime, hermes_command). | ✅ keep | arda-prometheus |
| annunimas-fleet | Device/fleet dispatch, providers, health. | ✅ keep | arda-fleet |
| annunimas-onboarding | Full first-run flow: provider, device, prerequisites, guided, console, io, readiness. | ✅ keep | arda-onboarding |
| annunimas-mcp | Exposes agents as MCP tools; browser, protocol, server, tools, external_sources. | ✅ keep | arda-mcp |
| annunimas-systemd | Thin `systemctl --user` typed client for service-health monitors. | ✅ keep | arda-systemd |

---

## Tier 2 — Useful but overlap/ambiguous (decide)

| Crate | Evidence | Rec | Note |
|---|---|---|---|
| annunimas-apollo | executor.rs, workflow.rs, rtk.rs, phi.rs, transport http/ipc, service.rs. Worker/executor node; overlaps prometheus/chronos. | 🔀 merge → prometheus | **DECIDED: merge into annunimas-prometheus** (`arda-prometheus`). Fold executor/workflow into the orchestration crate. |
| annunimas-chronos | Temporal: scheduler, predictions, time_series, audit automation, runtime. | ✅ keep | **DECIDED: keep** → `arda-scheduler`. Scheduling stays its own crate, separate from prometheus. |
| annunimas-hades | Lifecycle policy, organization, human_lifecycle, sweep, path_policy, sigils, support. Operator maintenance surface. | ✅ keep | **DECIDED: keep** → `arda-lifecycle`. Strip "HADES"/sigil verbage. |
| annunimas-human | "Human Knowledge Interface", lib.rs only (single module). | 🔀 merge into athena/comm | Small; fold into arda-athena or arda-comm. |
| annunimas-comm | A2H communication protocol, lib.rs only. | 🔀 merge into hermes | Fold into arda-hermes. |

---

## Tier 3 — Blueprint only (not live, decide whether to promote)

| Crate | Evidence | Rec | Note |
|---|---|---|---|
| annunimas-tool-harness | "defines contract and validation primitives for tool metadata, invocation envelopes, idempotency, governance posture. **not yet canonical**." | 📐→✅ promote | This IS the tool-gate core. Promote to real `arda-tool-gate`, strip "harness" verbage. |
| annunimas-service-registry | "foundational service registry… blueprint for registration, discovery, governance." | 📐 keep-if-needed / ❌ | Only if you want a registry crate; currently a blueprint. |
| annunimas-signal-grid | "Blueprint surface… **does not own live Hermes transport**." | ❌ drop | Pure contract stub; hermes already owns transport. |

---

## Tier 4 — Esoteric economy / mythic vocabulary (drop or strip)

These are exactly the "verbage to sanitize" targets. Mechanics may be useful;
names (Love Equation, Resonance, Triad, JouleWork, CRUSTIES, Solar, Council,
Sigils) are not.

| Crate | Evidence | Rec | Note |
|---|---|---|---|
| annunimas-plutus | joule_work.rs, love_equation.rs, economics.rs, meter.rs, ledger.rs. | 🔁 keep-generic, strip | **DECIDED: keep generic cost-metering only** → `arda-metering`. Extract meter/ledger economics; DELETE joule_work.rs, love_equation.rs, CRUSTIES economy, esoteric naming. |
| annunimas-governance | triad.rs, resonance.rs, love_equation.rs, joulework.rs, game_theory.rs, solar.rs, audio.rs, vision.rs, philosopher_profiles.rs (socrates…). | 🔁 keep-mechanism, strip names | Keep triad-validation + resonance-scoring as generic policy checks; **delete** love_equation/joulework/game_theory/solar/philosophers. Rename module → arda-policy. |
| annunimas-council | "Core blueprint for the Council agent… governance baseline all sovereign agents replicate." | ❌ drop | Mythic governance concept. Fold any generic baseline into arda-policy/governance. |
| annunimas-ceo | "Backward-compat shim. Canonical implementation moved to annunimas-prometheus." | ❌ drop | Shim, no longer needed. |

---

## Tier 5 — Niche / out of scope (drop unless wanted)

| Crate | Evidence | Rec |
|---|---|---|
| annunimas-forge-mind | Blender client/tasks, slicer (superslicer), forge generate/render/materialize, remote_workspace. | ❌ drop | **DECIDED: remove** — 3D assets out of scope. |

---

## Summary counts (proposed)
- ✅ Keep + rename (Tier 1): 13
- 🔀 Merge/drop (Tier 2): 5
- 📐 Blueprint promote/drop (Tier 3): 3
- ❌ Drop esoteric (Tier 4): 4
- ❌ Drop niche (Tier 5): 1
- Net after cherry-pick: ~13–18 crates (down from 26), all renamed to `arda-*`
  with mythic/esoteric names stripped.

## Decisions (locked)
1. apollo → **merge into prometheus** (`arda-prometheus`).
2. chronos → **keep** as `arda-scheduler`.
3. hades → **keep** as `arda-lifecycle` (strip sigil verbage).
4. plutus → **keep generic metering only** as `arda-metering` (delete esoteric economy).
5. forge-mind → **remove** (3D out of scope).

## Final crate set (after cherry-pick + rename)
- Core spine (13): arda-core, arda-cli, arda-charon, arda-athena, arda-mnemosyne,
  arda-hermes, arda-oracle, arda-warden, arda-prometheus (+apollo merged),
  arda-fleet, arda-onboarding, arda-mcp, arda-systemd.
- Tier2 resolves: arda-scheduler (chronos), arda-lifecycle (hades),
  arda-human + arda-comm folded into athena/hermes.
- Tier3: arda-tool-gate (tool-harness promoted), signal-grid dropped,
  service-registry deferred.
- Tier4: arda-metering (plutus generic), arda-policy (governance mechanism only),
  council + ceo dropped.
- Tier5: forge-mind removed.
- **Net: 18 crates** (down from 26), all in `arda-*` namespace, esoteric names stripped.

## Implementation notes
- Move is done crate-by-crate, manually, with rename + verbage strip.
- Each moved crate: rename dir `annunimas-X` → `arda-Y`, edit `Cargo.toml`
  `[package] name` and any `annunimas-*` path/dep references, strip mythic
  doc comments.
- Cross-crate references updated in the same batch the dependent crate moves.
