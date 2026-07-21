# Spine Tooling — Focus Deep-Dives

The five crates you named, plus `charon` (the gateway it all routes through).
Each dive lists: what it is, who consumes it, and the on-demand reroute.
All "used-by" and dependency facts are verified from Cargo.toml.

=====================================================================
1. annunimas-mnemosyne  (layer: memory)  — SUBSTRATE, convert FIRST
=====================================================================
Depends on: core, governance, plutus.
Used by: athena, charon, chronos, cli, hades, hermes, human, prometheus (8).

What it is: the memory/continuity store (persistence layer for the whole system).

Reroute plan (LIBRARY, [MANUAL]):
- Expose `MnemosyneService` as a plain library API (no IPC daemon).
- The 8 consumers currently `send_command` over IPC. Replace with direct
  `annunimas_mnemosyne::MnemosyneService::*` calls (library, in-process).
- For `cli`/`hermes` (the two you're also converting): they call it via
  `mnemosyne_ipc_send_command`. Swap to library calls when you rewrite them.
- Keep a lazy on-disk store; no resident process needed.
- DO THIS BEFORE touching athena/hades/hermes — they all lean on it.

=====================================================================
2. annunimas-athena  (layer: executors)  — INGEST
=====================================================================
Depends on: core, governance, mnemosyne, plutus.
Used by: cli, hermes.

What it is: ingest / knowledge triage (crawl4ai, scrapling, AthenaStore).

Reroute plan (LIBRARY, [MANUAL]):
- Convert `AthenaService` + `crawl4ai_fetch_markdown`/`scrapling_fetch_markdown`
  into a library + a thin `arda ingest <url>` CLI command.
- `cli` (`src/commands/athena.rs`, `src/commands/learning.rs`) currently
  `send_command` to the athena daemon. Replace with direct lib calls.
- `hermes` imports athena for handoff — see #5.

=====================================================================
3. annunimas-hades  (layer: runtime)  — ORG AUDIT (dead weight)
=====================================================================
Depends on: core, governance, mnemosyne, plutus.
Used by: cli, hermes.

What it is: org/structure auditor. Per REFACTOR_PLAN it is overkill and
watches only `core`/`docs`/`config` (stale scope) — blind to `crates/`,
`apps/`, the second workspace. Confirmed anti-pattern.

Reroute plan (DECOMMISSION, [MANUAL]):
- `cli` (`src/commands/hades.rs`) + `hermes` import it.
- Replace the two call sites with an on-demand `arda audit` job that runs the
  same checks (missing README/INDEX) against the CURRENT scope
  (`crates/`, `apps/`, `core/`) — fix the stale watch-path while migrating.
- After both call sites are gone, `hades` has zero dependents → delete crate.

=====================================================================
4. annunimas-cli  (layer: interface)  — ROOT ORCHESTRATOR
=====================================================================
Depends on: 15 crates (apollo, athena, charon, chronos, core, forge-mind,
governance, hades, hermes, mnemosyne, onboarding, oracle, plutus,
prometheus, warden). Used-by: none (it's the root bin).

What it is: the IPC orchestrator. `main.rs` imports every daemon's
`*_ipc_send_command` and dispatches ~30 subcommands under `src/commands/`.

Reroute plan (LIBRARY / becomes `arda` CLI, [MANUAL]):
- This is the central piece. As each daemon becomes a library, its `cli`
  subcommand (`src/commands/<crate>.rs`) switches from `send_command` to
  direct lib calls. The subcommand FILES stay; only the transport inside
  changes (IPC → in-process lib).
- `hades`/`athena`/`hermes` subcommands are the ones that unlock
  decommission of hades and slim of athena/hermes.
- No new daemon is added; `cli` becomes the single `arda` CLI surface
  (per REFACTOR_PLAN single entry point).

=====================================================================
5. annunimas-hermes  (layer: interface)  — COMMS BRIDGE
=====================================================================
Depends on: athena, charon, core, governance, hades, mcp, mnemosyne,
oracle, plutus, warden (10). Used by: cli, prometheus.

What it is: comms bridge (Discord, email, IMAP, boardroom). This is one of
the TWO crates that must stay resident (the `orome` bridge).

Reroute plan (MERGE→orome, KEEP-RESIDENT, [MANUAL]):
- Fold into the `arda` daemon as the resident comms surface (`orome`).
- Its imports: drop `hades` (audit rerouted), keep `mnemosyne` (now lib),
  keep `athena` handoff (now lib call), keep `mcp` (expose MCP from orome).
- `cli` + `prometheus` keep calling it, but as the resident bridge, not IPC
  to a separate daemon — or via the `arda` daemon's IPC if prometheus stays
  daemonized (prometheus is LIBRARY in the matrix; reroute prometheus first).

=====================================================================
6. annunimas-charon  (layer: runtime)  — GATEWAY (the other resident)
=====================================================================
Depends on: core, governance, mnemosyne, plutus, oracle, warden.
Used by: cli, hermes, prometheus.

What it is: multi-provider inference router. Per REFACTOR_PLAN → `manwe`,
local port 7171 (NOT 5110).

Reroute plan (MERGE→manwe, KEEP-RESIDENT):
- This is the ONLY other resident daemon. Keep it as the gateway.
- Slim it: the 41-file routing mesh is overkill for one box. Keep provider
  routing + echo gate; drop bandit/quota/telemetry sprawl unless used.
- `cli`/`hermes`/`prometheus` already target it via IPC — keep that surface.
