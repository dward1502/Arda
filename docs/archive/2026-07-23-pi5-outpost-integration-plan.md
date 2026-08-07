# Pi5 Outpost Deployment, Fleet, and Recovery Plan

> **For Hermes:** This plan owns only shared Pi5 deployment, fleet inventory, SSH reachability, and recovery mechanics. Warden/Varda backend work belongs to the governed-learning plan; research product work belongs to the Warden Research plan; all RELIC/CITADEL presence and presentation work belongs to the RELIC/CITADEL plan.

**Lifecycle status:** COMPLETE locally on 2026-08-01; archived after all PI5-1 through PI5-3 acceptance gates passed
**Stage posture:** Supporting infrastructure; not a Workbench release-candidate gate
**Canonical backend authority:** [Warden → Varda → Aulë governed learning loop](2026-07-27-warden-varda-ceo-learning-loop.md)
**Canonical research product authority:** [Warden Research](2026-07-29-warden-research-application-plan.md)
**Retained presence/presentation operations:** [RELIC/CITADEL](../operations/relic-citadel-presence.md)

## Reconciliation decision

The previous plan duplicated research receipts, Varda ingestion, CITADEL chat, companion presentation, and council/presence work. Those tasks are removed here rather than left as parallel unchecked work.

This plan remains active because three responsibilities are not absorbed elsewhere:

1. reproducible AArch64 build and deployment of current Warden source;
2. canonical fleet/SSH identity and reachability checks shared by both Pi5 nodes;
3. cross-outpost status, restart, reboot, and rollback procedures.

Application-specific bridge, renderer, kiosk, stale-scene, and soak behavior remains exclusively in the RELIC/CITADEL plan.

## Closeout baseline verified on 2026-08-01

- `config/fleet.toml` contains active `node-pi5-warden` and `node-pi5-citadel-avatar` entries, canonical Tailscale addresses, SSH users, health URLs, and explicit restart commands.
- `core/state/warden_scout_runtime.json` records the Warden scout, SearXNG, research timer, and survey timer as active/enabled.
- Stage 4 used Warden discovery in a live explicit-question research chain and linked the result to a Workbench run as advisory evidence: `docs/evidence/stage-4-private-beta/research-chain-live-stage4-research-20260731T181502Z.json`.
- Repository source now provides bounded `scripts/pi5_outpost_status.sh`,
  `scripts/pi5_outpost_restart.sh`, and `scripts/pi5_warden_scout_delivery.sh`
  helpers with offline fixtures and live two-node evidence below.
- Current-source AArch64 build/redeploy, reboot recovery, receipt persistence,
  rollback, and deterministic artifact checks are proved by PI5-1 evidence below.
- RELIC/CITADEL application state and deployment claims are intentionally not repeated here; its canonical plan owns them.

## Authority boundaries

- Deployment tooling may inspect or restart only explicitly named units on enrolled nodes.
- No helper may embed passwords, provider credentials, or unrestricted remote shell snippets.
- Reachability, SSH authentication, user-systemd state, application health, and data freshness are separate gates.
- Restart/deploy success never grants Warden knowledge authority or CITADEL execution authority.

## PI5-1 — Reproducible Warden AArch64 delivery

**Owner:** this plan
**Depends on:** the governed-learning plan's current Warden protocol/runtime contract, but not its later autonomous-learning phases

**Files**

- `outposts/arda-outpost-protocol/`
- `outposts/arda-outpost-scout/`
- `config/systemd/arda-warden-scout.service`
- a repository-owned AArch64 build/deploy helper added only after the supported toolchain is chosen

**Open work**

- [x] Pin and document the supported AArch64 build path for current source.
- [x] Produce a checksummed Warden scout artifact and record source revision/toolchain identity.
- [x] Deploy atomically, preserve the prior binary, restart the named unit, and verify `/health` plus one bounded source-cited request.
- [x] Reboot Warden and prove service recovery, receipt persistence, and no duplicate replay.
- [x] Exercise rollback to the prior binary without losing append-only scout evidence.

**Evidence — 2026-08-01**

- Added `scripts/pi5_warden_scout_delivery.sh`, a fixed-scope helper with no
  caller-selected host, service, remote command, credential, or password input.
  `outposts/arda-outpost-scout/README.md` documents the supported rootless path:
  Rust `1.94.0`, `cross` `0.2.5`, Podman, and target
  `aarch64-unknown-linux-gnu`.
- Two consecutive `build` runs produced the same AArch64 ELF SHA-256:
  `42c15c9184d0e7e15e37e16e50c972f3c85ee00a142d42320866331ef90bed4e`.
  The manifest records source revision
  `3dec4d9f3a83c6f0668daac630e1082835c5deaa`, Cargo.lock SHA-256
  `d5dd0ba7ddc7dae5edf13042a0f2b24f31837391a906aa663ccf64d02d47f9a8`,
  and cross image digest
  `sha256:9e5d86740280e021e5f372afcad2eda7367676f33ec40085b49ee88a2652cfe5`.
- Atomic deployment preserved prior binary SHA-256
  `7c8cbaa998d3dd2e21426fcb359794daee2a20b3ce69731050257b24f5804870`,
  installed the current artifact, restarted only
  `arda-warden-scout.service`, and returned active/enabled plus
  `/health => status=ok, source=node-pi5-warden, authority=advisory`.
- The bounded smoke request returned three HTTP(S) sources and receipt
  `mem_868775d8c7ed4407a4d4ad8dce8722b5`; pre-reboot recall returned exactly one
  matching record.
- Reboot changed boot ID from `1522b6f3-a5ed-4883-a458-449b550a2369` to
  `9184e294-6434-40ff-af2f-0567c6e3f4ee`; SSH, the named unit, and `/health`
  recovered, and post-reboot recall still returned exactly one matching receipt.
- Rollback restored the prior checksum and healthy unit. That older binary could
  not project the newer receipt through its pre-repair recall schema, but the
  append-only file remained at
  `~/.local/share/arda/warden/episodic/2026-08/mem_868775d8c7ed4407a4d4ad8dce8722b5.jsonl`.
  Rolling forward to the current artifact restored recall of the same single
  record; no evidence was deleted or replayed.

## PI5-2 — Fleet and SSH truth checks

**Owner:** this plan

**Open work**

- [x] Validate required Pi fields in `config/fleet.toml`: node ID, role, host alias, SSH user, Tailscale address, health URL, restart scope, and restart command.
- [x] Add finite-timeout, `BatchMode=yes` checks for the canonical `warden` and `citadel` aliases; retain `raspberrypi` only as a documented compatibility alias.
- [x] Report Tailscale reachability, TCP/22, SSH authentication, and service health independently.
- [x] Fail closed when a fleet record and observed host identity disagree.

**Evidence — 2026-08-01**

- `config/fleet.toml` now records explicit `ssh_alias` and allowlisted
  `restart_group` values for both Pi records; CITADEL alone retains
  `ssh_compat_aliases = ["raspberrypi"]`.
- `scripts/pi5_outpost_status.sh all` independently passed fleet identity,
  Tailscale identity/IP, TCP/22, key-only SSH user/hostname/AArch64 identity,
  Warden's three named units and two health URLs, and CITADEL's two named units
  and RELIC health URL.
- Live identity evidence was `numenor@warden` through `ssh warden` and
  `citadel@raspberrypi` through `ssh citadel`; Tailscale independently reported
  `warden (100.110.85.37)` and `raspberrypi-1 (100.119.130.127)`.

## PI5-3 — Shared status, restart, and recovery helpers

**Owner:** this plan
**Boundary:** shared node/service mechanics only; RELIC bridge/kiosk behavior stays in the RELIC/CITADEL plan

**Files**

- Create: `scripts/pi5_outpost_status.sh`
- Create: `scripts/pi5_outpost_restart.sh`
- Modify only if validation requires it: `config/fleet.toml`
- Update operator recovery guidance in `docs/archive/deferred/MIRROMERE_RELIC_OUTPOST_VISION.md`

**Open work**

- [x] Implement read-only status output with finite connection and command timeouts.
- [x] Require an explicit node and allowlisted service group for every restart; never provide an all-fleet restart default.
- [x] Verify post-restart unit state and application health separately.
- [x] Add reboot-path and unreachable-node fixtures that leave other nodes untouched.
- [x] Record rollback instructions and the last-known-good artifact identity without storing secrets.

**Evidence — 2026-08-01**

- `scripts/tests/pi5_outpost_helpers_test.sh` passed unreachable-Warden,
  independent-CITADEL-status, rejected-all-node-mutation, exact CITADEL restart,
  and finite Warden reboot fixtures. Command logs prove each mutation fixture
  leaves the other node untouched.
- Live `warden scout` restart changed only `arda-warden-scout.service`; unit and
  `:8092/health` checks passed on attempt 1. Live `citadel presence` restart
  changed only `relic.service` and `citadel-kiosk.service`; both units and
  `:8091/` passed on attempt 1. A subsequent full status run passed every gate.
- `docs/archive/deferred/MIRROMERE_RELIC_OUTPOST_VISION.md` records exact status, scoped restart,
  reboot, and Warden rollback commands plus last-known-good artifact SHA-256
  `42c15c9184d0e7e15e37e16e50c972f3c85ee00a142d42320866331ef90bed4e`.
  No helper accepts passwords, credentials, arbitrary hosts, arbitrary units,
  or arbitrary remote commands.

## Acceptance

- [x] Current Warden source is reproducibly built for AArch64, checksummed, deployed, reboot-tested, and rollback-tested.
- [x] Both canonical Pi aliases pass independent fleet, network, SSH, service, and health checks.
- [x] Shared recovery helpers are bounded, non-interactive, secret-free, and tested against unreachable/degraded fixtures.
- [x] No research backend, watchlist/product, RELIC renderer, CITADEL companion, or council/presence task is owned by this plan.

## Stage 5 dependency

None of PI5-1 through PI5-3 blocks the Workbench release candidate. If Warden Research is included in a Stage 5 beta, PI5-1 becomes a beta deployment gate. If RELIC/CITADEL is enabled, PI5-2 and PI5-3 are prerequisites for remote operational support, while application-specific recovery remains governed by the RELIC/CITADEL plan.