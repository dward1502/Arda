# Spine Tooling — Execution Batches

Ordered so no batch leaves the workspace unbuildable. Every batch ends with a
BUILD/VERIFY gate: `cargo build -p <touched>` must pass before the next.

Principle: convert the SUBSTRATE (mnemosyne) first, then the resident pair
(manwe/charon, orome/hermes), then peel daemons into libraries, then
decommission the dead weight (hades, ceo, fleet, signal-grid).

=====================================================================
BATCH 0 — Prereqs (no code change, do once)
=====================================================================
- [x] `mkdir -p docs/plans` (done).
- [x] Confirm `cargo build` is green baseline: `cargo build --workspace`.
- [x] Snapshot `Cargo.lock` (git stash / tag) so we can diff later.
GATE: baseline build passes.

=====================================================================
BATCH 1 — mnemosyne → library (substrate)
=====================================================================
[MANUAL] you move this; steps for when you do:
- [ ] In `crates/old-annunimas/annunimas-mnemosyne/Cargo.toml`: add lib
      target if missing; keep `MnemosyneService` public.
- [ ] For each of the 8 consumers (athena, charon, chronos, cli, hades,
      hermes, human, prometheus): replace
      `annunimas_mnemosyne::transport::ipc::send_command` with direct
      `annunimas_mnemosyne::MnemosyneService::*` lib calls.
- [ ] Remove the mnemosyne IPC daemon wiring from the `arda` supervisor.
GATE: `cargo build -p annunimas-mnemosyne` AND every dependent still builds.

=====================================================================
BATCH 2 — Resident pair: charon→manwe + hermes→orome
=====================================================================
- [ ] charon: slim to gateway (port 7171), drop unused routing sprawl.
      KEEP resident in `arda` daemon.
- [ ] hermes [MANUAL]: fold into `orome` comms bridge (resident). Drop
      `hades` import; switch `athena`/`mnemosyne` to lib calls; carry `mcp`.
- [ ] `cli` + `prometheus`: keep targeting them via `arda` daemon IPC (or
      lib if prometheus is already library in BATCH 3).
GATE: `cargo build -p annunimas-charon -p annunimas-hermes` green;
      `arda` daemon starts and exposes 7171 + comms.

=====================================================================
BATCH 3 — Peel daemons to libraries (parallel-safe per crate)
=====================================================================
Each: add/confirm lib target, expose service as public API, replace the
consumer's `send_command` with lib call, remove that crate's daemon from
supervisor. Crank these as you touch them (REFACTOR_PLAN rename rule):
- [ ] annunimas-apollo  → lib (cli, prometheus use)
- [ ] annunimas-plutus  → lib (12 consumers)
- [ ] annunimas-oracle  → lib (4 consumers)
- [ ] annunimas-warden  → lib (3 consumers)
- [ ] annunimas-prometheus → lib (ceo, cli use)  ← do before orome wiring
- [ ] annunimas-chronos → lib
- [ ] annunimas-athena [MANUAL] → lib + `arda ingest`
- [ ] annunimas-forge-mind, council, systemd, service-registry, human,
      onboarding, comm, tool-harness → lib / thin
GATE: `cargo build --workspace` green; no `send_command` to a now-library
      crate remains in `cli`/`hermes`.

=====================================================================
BATCH 4 — Decommission dead weight
=====================================================================
Only after their dependents are rerouted (verified by `cargo build`):
- [ ] annunimas-hades [MANUAL]: delete after `cli`+`hermes` drop it;
      replace with `arda audit` on-demand job (fix stale watch scope).
- [ ] annunimas-ceo: BC shim, zero real logic → delete (prometheus rerouted).
- [ ] annunimas-fleet: fleet = later growth ring, not for one box → delete.
- [ ] annunimas-signal-grid: "does not own live transport" → delete.
GATE: `cargo build --workspace`; `search_files` for the deleted crate names
      in Cargo.toml returns nothing.

=====================================================================
BATCH 5 — Cli consolidation + cleanup
=====================================================================
- [ ] `cli` [MANUAL]: all subcommands now call libs in-process; prune the
      IPC bootstrap code (`cli_bootstrap.rs`, `ipc_bridge.rs` dead branches).
- [ ] Nested-duplicate cleanup: `annunimas-mnemosyne/crates/annunimas-mnemosyne`
      and `annunimas-warden/crates/annunimas-warden` — collapse the inner
      crate into the outer (the dup the REFACTOR_PLAN flags).
- [ ] Update `docs/REFACTOR_PLAN.md` naming table to mark migrated crates.
GATE: `cargo build --workspace` + `cargo test --workspace` green.
