# Spine Tooling — Disposition Matrix

VERIFIED from `Cargo.toml` of each crate under `crates/spine/<layer>/annunimas-*`.
"Used-by" = crates that `path`-depend on it (will break if removed).

Disposition legend:
- KEEP-RESIDENT  — must stay resident in the `arda` daemon (gateway / comms only).
- LIBRARY        — convert daemon to an on-demand library + thin CLI; no resident daemon.
- MERGE          — fold into another crate / the `arda` daemon.
- DECOMMISSION   — remove after dependents rerouted.
- [MANUAL]       — you said you'll move/delete this by hand; reroute prereqs listed.

## The five you named

| Crate | Layer | Used-by | Disposition | On-demand target |
|-------|-------|-----------|--------------|------------------|
| annunimas-athena | executors | cli, hermes | [MANUAL] LIBRARY | `arda ingest` command + `varda` lib (REFACTOR_PLAN) |
| annunimas-hades | runtime | cli, hermes | [MANUAL] DECOMMISSION | drop; org-audit becomes `arda audit` on-demand job |
| annunimas-mnemosyne | memory | athena, charon, chronos, cli, hades, hermes, human, prometheus | [MANUAL] LIBRARY | `vaire` lib + on-demand store; substrate for all others |
| annunimas-cli | interface | — (root) | [MANUAL] LIBRARY | becomes the `arda` CLI surface itself |
| annunimas-hermes | interface | cli, prometheus | [MANUAL] MERGE | folds into `orome` comms bridge (KEEP-RESIDENT) |

## Full matrix (all 26 spine crates)

| Crate | Layer | Used-by (count) | Disposition | Notes |
|-------|-------|-----------------|------------|-------|
| annunimas-core | governance | 21 | KEEP-RESIDENT | shared types/ledger/task; foundational |
| annunimas-governance | governance | 15 | KEEP-RESIDENT | triad validation; used everywhere |
| annunimas-plutus | runtime | 12 | LIBRARY | JW economics → on-demand lib |
| annunimas-charon | runtime | 3 | MERGE→manwe | KEEP-RESIDENT gateway (port 7171) |
| annunimas-mnemosyne | memory | 8 | LIBRARY [MANUAL] | substrate; convert before dependents |
| annunimas-oracle | runtime | 4 | LIBRARY | reasoning/validation → on-demand |
| annunimas-hermes | interface | 2 | MERGE→orome [MANUAL] | comms bridge, resident |
| annunimas-warden | tooling | 3 | LIBRARY | monitoring → on-demand |
| annunimas-apollo | interface | 2 | LIBRARY | execution daemon → on-demand lib |
| annunimas-prometheus | observability | 2 | LIBRARY | orchestration → on-demand (ceo, cli use) |
| annunimas-hades | runtime | 2 | DECOMMISSION [MANUAL] | dead weight per REFACTOR_PLAN |
| annunimas-athena | executors | 2 | LIBRARY [MANUAL] | ingest → on-demand |
| annunimas-chronos | memory | 1 | LIBRARY | scheduling → on-demand |
| annunimas-mcp | interface | 1 (hermes) | MERGE→orome | MCP exposure follows hermes |
| annunimas-comm | interface | 1 (prometheus) | MERGE→orome/prometheus | A2H protocol |
| annunimas-fleet | interface | 1 (prometheus) | DECOMMISSION | fleet = later growth ring; not for one box |
| annunimas-onboarding | interface | 1 (cli) | LIBRARY | one-shot setup → on-demand |
| annunimas-ceo | executors | 1 (prometheus) | DECOMMISSION | explicit BC shim (doc: "kept to avoid breaking imports") |
| annunimas-council | governance | 1 (prometheus) | LIBRARY | blueprint/contract → on-demand |
| annunimas-signal-grid | observability | 0 | DECOMMISSION | "does not own live transport" — dead surface |
| annunimas-service-registry | executors | 0 | LIBRARY | foundational registry, thin it |
| annunimas-systemd | executors | 1 (prometheus) | LIBRARY | thin typed systemctl client |
| annunimas-tool-harness | tooling | 2 (core, forge-mind) | KEEP-RESIDENT | governed tool invocation contract |
| annunimas-forge-mind | tooling | 1 (cli) | LIBRARY | comfyui/iterate → on-demand |
| annunimas-human | executors | 1 (mnemosyne) | LIBRARY | human override surface → on-demand |
| annunimas-charon→manwe | runtime | (see charon) | KEEP-RESIDENT | gateway only resident daemon |

## Dependency reality check (why order matters)
- `mnemosyne` has the widest blast radius (8). It is converted to a library
  FIRST, so the 8 dependents can keep calling it as a library without a daemon.
- `hades` is only used by `cli` + `hermes`. Both are [MANUAL]. Once those two
  stop importing `hades`, the crate is deletable with zero remaining dependents.
- `athena` + `hermes` both import `hades` and `mnemosyne`. Reroute those two
  imports, and hades/mnemosyne lose their heaviest consumers.
- `charon` (→manwe) is the ONLY crate that must stay resident besides hermes→orome.
  Every other daemon can become a library without losing runtime capability.
