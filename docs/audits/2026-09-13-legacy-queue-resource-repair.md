---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "verification_record"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-09-13"
---

# Legacy queue replay resource repair

## Scope and cause

The [objective-runtime cutover](../plans/2026-09-01-arda-objective-runtime-cutover.md)
retired global JSONL admission. Installed behavior still replayed that history:
the daemon rebuilt its operator projection on a two-second timer using the full
queue, next-action selection also loaded the queue, and a boot/ten-minute
`arda-aule-autopilot.timer` invoked the legacy one-shot.

The queue contained 1,193,714 records and measured 914.1 MiB at diagnosis.
At cutover it was 960,207,355 bytes. The old daemon's RSS was 6.48 GiB and its
service cgroup held 7.68 GiB. The earlier startup autopilot cgroup peak was
10.96 GiB; that is not a claim of process-only RSS.

## Repair and boundaries

- Operator and next-action readers use read-only current ObjectiveStore summaries,
  without creating/migrating SQLite, reading execution payloads/capsules, or
  falling back to legacy queue/schedule/current-runs-registry authority.
- Checkpoints are selected through exact nonterminal resident leaf/run bindings;
  next-action workbench candidates additionally require the requested operator.
- Consumed/quarantined schedules are excluded. Paused/unapproved objectives do
  not expose a dispatch wake. Old schemas are not migrated by readers.
- Legacy CLI `autopilot once/run/status` rejects before queue/world hydration.
  Both installed autopilot timers are disabled/inactive; reinstall cannot enable
  replay. Broader legacy-producer retirement remains a cutover-plan gate.
- Local queue/schedule files are preserved. Exact ignore rules and staged
  untracking stop future source snapshots from adding these files. The previous
  index is saved at `.git/index.before-legacy-queue-untracking`.

Independent review identified a remaining historical Workbench registry fallback
in next-action selection. A focused regression reproduced the defect, the path
was replaced with resident bindings, and exact-hash corrective review approved
`next_action.rs` (`690db0b4aa548b08b9af89a8d62b909245e037c80a1a0761a85cbd8f94273790`)
and its test (`e33ca771ae7ba293496c14625f19702465113344269ecefe5273b0c79cc02ee9`).

## Verification

- `cargo test -p arda-engine --all-features --quiet`: 344 passed, 10 ignored.
  Ignored namespace/provider tests are not claimed as new cutover acceptance.
- `cargo test -p arda-aule --all-features --quiet`: 403 passed.
- `cargo test -p arda --test root_daemon --quiet`: 6 passed.
- Engine/Aulë all-feature, all-target Clippy with `-D warnings`: passed;
  Engine Clippy and complete Engine tests rerun after the review correction.
- `bash scripts/test_install_arda_automation_units.sh`: passed rollback and
  successful-reinstall timer-retirement checks.
- HUD TypeScript check passed; Vitest expanded the requested filter to the full
  suite and passed 616 tests in 150 files.
- Scoped formatting/diff checks passed. Workspace-wide `cargo fmt --check` still
  flags pre-existing wrapping in `crates/engine/src/bin/snapshot_admission/mod.rs:89`.

## Installed measurement

Cutover occurred at 2026-09-14 01:43 UTC (September 13 local time). Preflight found
no unfinished objectives or unexpired leases. Three retained leases belonged to
failed work and were expired; they were not reset. Old daemon/Manwë PIDs 1821
and 1908 disappeared before replacement. SQLite and old binaries were backed up
at `/var/home/mythos/.local/state/arda/rollback/20260914T014306Z-legacy-queue`.

Candidate, installed and running daemon SHA256 matched:

- `arda`: `4863a985a85bfd6ccd0e8d61894e13d32b206efa8ff01e47900d781ebfd90393`
- `arda-cli`: `415ff16c6096f470630af2eb88ae658a12272daa4a240c33a76bd68ae956c14b`

A 60.01-second observation of installed daemon PID 86472 across repeated publisher
ticks measured 17.93–18.00 MiB RSS, 29.072 MiB logical reads, and zero disk-read
bytes. The final service-cgroup sample was 18.33 MiB (12.96 MiB anonymous,
3.51 MiB file cache); cgroup charging and per-process RSS are distinct metrics.
Queue and schedule sizes and nanosecond mtimes were unchanged. Installed CLI
refusal measured 3,780 KiB peak RSS with immediate nonzero exit, as intended.

`/health`, `/v1/status`, `/v1/operator-projection`, and `/v1/next-action` returned
HTTP 200. ObjectiveStore dependency was ready with no active objectives/runs.
The supervisor reported Manwë PID 86488 healthy, with zero restarts. Existing
OTLP network-export failure remains unrelated and unresolved.

## Still open

This proves the installed idle replay/memory repair, not gameplay frame pacing,
provider execution under load, retained-snapshot acceptance, or the whole
Autonomous Task Completion Program. The operator's combined game/YouTube/work
workload has not been exercised by this repair.

With subsequent operator authorization, the four unpublished commits were rebuilt
without `core/projects/tasks/queue.jsonl` and `schedules.jsonl`. Each replacement
tree was verified to differ only in these paths; messages and authorship were
preserved. The original history is retained locally at
`backup/pre-queue-sanitize-20260914T015221Z`, which must not be pushed.
The rewritten tip is `d591223cdac17d54d87fcbb90ad12a8adf84e049` before the
resource-repair commit. Local runtime files and unrelated working-tree/index
changes were preserved. Published history was not rewritten.
