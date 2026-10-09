---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "operations_acceptance"
  owner: "PROMETHEUS"
  status: "draft"
  reviewed: "2026-10-08"
---

# R09 state lifecycle and installation qualification

Status: in progress. Owner: persistence modules, root and launcher. R08 is
accepted at `2165adb3`; general G6.5 calibration is not reopened here.

## Ownership and disposition

Classification follows producers and consumers, not Git tracking or filename.
No entry below authorizes deletion, live migration or identity-marker replacement.

| Class / paths | Owner and recovery contract | Export / retention |
|---|---|---|
| Shipped source, Cargo/pnpm locks, `apps/*/packaging`, configuration templates | Root/native build owners; install exact manifest-bound artifacts. Config templates are inputs, not operator settings. | Retain release source, manifests and prior executable through rollback acceptance. Exclude runtime files from a distribution even if tracked. |
| Operator configuration: selected XDG `arda` config; development `config/manwe.providers.toml`, service drop-ins and credential references | Respective config/provider owners; preserve overrides and permissions. Beta operations own XDG copies only. | Private backup; secret-name exclusion is not content redaction. Do not publish archives. Credentials require separately authorized encrypted storage or re-entry. |
| `data/arda/objectives.sqlite3`, authority sidecar; keeper database/snapshots; `data/runs` | Engine ObjectiveStore and keeper own schema, leases, authority and run lineage. A database copy changes filesystem identity; copying its marker does not authorize the copy. Existing authority checks must refuse mismatched identity. | Preserve original database, sidecars, run receipts and related keeper evidence together. No age-only expiry. Schema migration belongs to Engine migrations, not launcher or a generic restore script. |
| `core/state/memory`, Vaire Books/indexes; Orome sessions; Workbench project/run records | Vaire/Orome/Workbench owners retain canonical history and correction/revocation semantics. Derived indexes are rebuildable only from retained canonical sources through their owner. | Private export with provenance; retain referenced records and accepted M3/M4/M5 evidence. No automatic history TTL introduced by R09. |
| `data/manwe`, `data/governance`, `data/prometheus`, resource ledgers | Respective append/replay owners. Malformed historical records remain evidence; do not silently rewrite or truncate. Existing Bacon Lite rotation does not establish retention for other writers. | Writer-owned rotation/compaction only after replay and reference checks. Raw content is private even without secret-shaped names. |
| `target/qualification`, `docs/operations/*evidence*`, accepted outcome receipts and rollback archives | Acceptance owner retains exact source/artifact identities, failures and dispositions. `target/` is not uniformly disposable. | Keep pinned/referenced acceptance and rollback material; publish only reviewed bounded summaries/hashes. Private raw archives stay local mode 0600. |
| Cargo/Vite caches, unreferenced build intermediates, regenerable metrics projections | Build/metrics owner. Distinguish running/rollback binaries and evidence before reclamation. | Eligible for explicit capacity cleanup after active-use/reference checks; not a history retention mechanism. |
| Launcher WebKit data, runtime sockets/locks | Launcher/runtime owner. UI cache reset does not erase canonical history. Active locks/sockets must not be copied as authority. | Excluded from beta config/data archive; reset has separate quarantine semantics. |
| `tests/fixtures`, synthetic temp HOME and qualification copies | Test owner; isolated synthetic inputs only. A copied private database remains private evidence, not a public fixture. | Disposable only after preserving required receipts; never point fixtures at live state. |
| Operator project workspaces, `core/Beren`, unrelated dirty changes | Operator/project owner, outside installer mutation scope. | Preserve exactly; not installer cache or an implicit migration input. |

## Backup and migration boundaries

Existing owner: `scripts/arda_beta_ops.py`, with regressions in
`tests/test_arda_beta_ops.py`. It rejects repository-contained mutable roots;
therefore a successful XDG archive cannot mean the development composition is
backed up. Use quiescent isolated copies for qualification. Whole-runtime backup
requires a coordinated writer stop or storage snapshot across ObjectiveStore,
keeper, runs, memory and communication state. Per-file stability checks cannot
prove a cross-store transaction boundary.

Backups preserve bytes, not execution authority. Never edit a copied authority
marker to claim successful runnable restore. Engine's
`crates/engine/src/objectives/store/authority.rs` binds device/inode and database
identity; Engine migrations and keeper reconciliation own runnable recovery.
Live migration/rebinding requires its own reviewed recovery procedure and operator
authorization. Launcher upgrade changes launcher artifacts, not these schemas.

Default beta archives omit secret-named files but are not redacted exports.
Diagnostic export is a distinct existing command with best-effort pattern
redaction; review its output before sharing. Retention defaults to preservation:
no new scheduled deletion, external log truncation or tracked-state removal.

## Runnable restore: bounded owner design

Original-identity reinstatement and backup-copy authorization are different gates.
The installed qualification rejected an extracted database plus its copied marker
at the canonical path, then reinstated the displaced original by same-filesystem
rename. Device, inode, marker hash and complete ObjectiveStore SQL history matched
before/after; full managed root/keeper restart retained the terminal release and
passed selected-endpoint health. Private evidence is
`/var/home/arda-qualification/evidence/reviewed-runtime-05623ba3/original-reinstatement.json`.
This proves original recovery, not authorization of a new backup inode. The probe
is qualification evidence, not a shipped recovery command.

Independent owner design review requires a narrow Engine-owned offline transfer
before runnable backup-copy acceptance. Implement alongside
`objectives/store/authority.rs` and expose through `arda-objective-store`; keep
Python beta operations an extraction/validation caller, not authority owner.
The first version must:

- Authenticate the operator outside archive contents and bind account, canonical
  destination, request ID, archive digest and expected predecessor identity. An
  arbitrary operator string or copied marker is not authentication.
- Require the surviving trusted original authority and exclusive maintenance
  ownership. Fence the complete managed stack, including keeper and workers.
- Accept only quiescent, terminal/settled, history-equivalent state; compare the
  latest WAL-visible history and paired keeper/run inventory. Refuse stale or
  divergent backups, unresolved leases/publications and unacknowledged cleanup.
  Do not merge historical SQL or infer cleanup acknowledgement from process exit.
- Publish exactly one successor and retire the predecessor. A durable bound
  intent/receipt and exclusion across both inodes must fail closed at every crash
  boundary; per-inode locks alone cannot authorize transfer safely.
- Preserve terminal receipts, releases/revocations and gateway replay protection.
  Refuse replay of retired authority, changed-payload request reuse, aliases and
  path substitution; exact request replay must be idempotent.

Required tests cover stale backups after each terminal/replay event, copied and
missing markers, active/lost leases, runtime/maintenance races, symlink/hard-link
and special-file inputs, every publication interruption, and predecessor replay.
The installed positive case must use a genuinely new inode and restart the whole
managed stack without resurrecting old work. Disaster recovery without surviving
trusted continuity, schema conversion and cross-account migration are excluded;
operator approval cannot establish missing historical facts.

## Installation acceptance boundary

Reuse [launcher acceptance](launcher-local-runtime-acceptance.md) and
[private-beta operations](../operator/private-beta-install-recovery.md). Separate:

1. Copy-only archive integrity and restore failure recovery.
2. Exact-artifact installation/upgrade/rollback mechanics in an isolated HOME.
3. Native library, desktop startup, truthful required-service readiness,
   independent lifetimes, reconnect and bounded outage recovery.
4. Clean OS-user/VM installation and supported-profile qualification.

Temporary HOME mechanics do not establish item 4. Historical August acceptance
is provenance, not current-source evidence. The existing package requires a
complete source checkout; development service composition and source-backed
state must not be advertised as self-contained packaged startup.

R06 live budget/route acceptance, G6.5 general calibration and R04 combined-scenario
obligations remain open. Mesh inference stays off. R10's seven consecutive installed
daily cycles cannot be replaced with rapid fixture runs; R11 follows its dependency.
