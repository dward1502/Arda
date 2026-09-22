---
soterion:
  sigil: "SCROLL"
  role: "operations_guide"
  owner: "RUMIL"
  status: "draft"
  reviewed: "2026-09-15"
---

# Rúmil nightly hygiene audit

## Authority and execution

The existing Hermes job `58feda71d6ba` runs at 03:17 local scheduler time. Its
compatibility wrapper `~/.hermes/scripts/arda_rumil_nightly_organization.sh` invokes
`python3 scripts/rumil_nightly_hygiene.py` from the canonical Arda checkout.
It remains script-only: no new scheduler, repair worker, or executable queue.
The older `rumil_organization_maintenance.sh` remains the explicit legacy/manual
HUD audit entrypoint; its `storage_hygiene_last.json` is NOT the new nightly receipt.

- Exit **0**: complete scan, either clean or with advisory findings.
- Exit **1**: findings when explicitly invoked with `--strict` (CI/manual gate).
- Exit **2**: failed/incomplete scan; never a successful empty result.
- `--report` forces a human summary. Normal unchanged runs emit no stdout;
  new/reopened/resolved findings, newly actionable findings, significant growth,
  partial execution, and a seven-day reminder emit bounded summaries.
- Emission is not delivery acknowledgement; Hermes owns delivery/error reporting.

## Whole-repository coverage

Every directory is traversed for metadata, including ignored files, archives,
`.git`, dependency trees, build outputs, runtime data, configuration, code and
documentation. Inventory is logical/apparent file bytes, not unique disk usage:
hardlinks and sparse files can differ from physical allocation.

Symlinks are counted but not followed. External linked trees and remote fleet
filesystems are NOT included. Special files are counted, never read. The audit's
own `data/rumil/hygiene` output is excluded to avoid self-generated growth.
Default budgets are 500,000 regular files and 90 seconds (`--max-files`, `--seconds`).
Unreadable content and exhausted budgets produce explicit partial coverage and
retain previous unobserved findings rather than falsely resolving them.

Content policy is intentionally narrower than inventory:

- Dependencies/build/private agent state: metadata only; never execute or parse it.
- Known secret paths, runtime logs/state, historical archive content: metadata only.
- Authored Markdown: existing local-link semantics, excluding fenced examples.
- Authored Python: AST syntax only; never import the module.
- Strict JSON: syntax only; JSONC-bearing TypeScript/JavaScript/editor configs
  are explicitly counted as not syntax-checked, not falsely called invalid JSON.
- Rust, TypeScript, JavaScript, shell, configuration and other supported authored
  source extensions: merge-conflict-marker detection, not semantic correctness.
- Authored backup/original files and top-level temporary storage: review candidates,
  not deletion targets. Content over 1 MiB is counted as skipped.

Root-level growth alerts require at least both 64 MiB and 10% growth versus the
previous complete scan. A build can explain growth; growth is not waste proven.

## Finding lifecycle and review

`data/rumil/hygiene/state.sqlite3` is the serialized, transactional finding state;
`data/rumil/hygiene/latest.json` is its atomically replaced consumer projection.
The state holds stable kind/path/detail IDs, first/last seen, observation counts,
resolved records, reopened findings, current proposals and coverage. A resolved
finding means it is no longer observed under the current policy, not that a repair
was executed or independently accepted. This distinction matters for false-positive
classifier corrections and policy changes.

Broken links must recur before they become actionable review proposals. A unique
same-basename target is a suggestion only; no automatic retargeting. Findings retain
the same identity if their Markdown line number changes. No source content excerpts
or secret values are included in summaries.

Engine's existing next-action publisher consumes a fresh (at most 48 hours),
complete, read-only hygiene receipt and offers **Review Rúmil hygiene candidates**.
It rejects malformed, oversized, future-dated, stale and execution-enabled reports.
This is a review action, not a new objective or permission to modify a file.
It uses the existing priority selection and does not preempt higher-priority work.
The reviewer inspects exact paths and evidence digests, then submits any approved
scope through existing authenticated Engine objective intake. Cleanup additionally
requires exact targets, current-use checks, approval, rollback/retention decisions,
and post-change verification. The auditor contains no cleanup executor.

The source integration is tested; installing/restarting the daemon is a separate
live acceptance gate. Do not claim the running daemon consumes it just because
Rust tests or JSON generation succeed. Full governed pivot and accepted-repair
feedback remain owned by the [daily improvement loop](../plans/DAILY_RESEARCH_IMPROVEMENT_LOOP.md).

## Avoiding fragile document navigation

Use the stable [documentation index](../INDEX.md) for entry navigation. Discover
active plans from `docs/plans/`; do not enumerate fluid plan files in every root
status document. Keep precise archival/evidence links where the historical identity
matters. When moving an authoritative document, update its actual inbound references
in the same change and run the strict existing Markdown gate before merging.

A redirect file for every retired plan would pollute the active queue. A basename
search is not a reliable identity resolver. Stable entry indexes plus move-time
reference repair are the default; introduce persistent document IDs only when an
actual cross-tool resolver and migration policy exist. Nightly detection is the
backstop, not a replacement for those checks.

## Deeper code audit path

Nightly checks currently provide inventory, conflict markers and bounded Python/JSON
syntax checks. They do **not** prove Rust/TypeScript compilation, test correctness,
security, dependency safety, dead-code absence or architecture conformance.
Extend the existing daily/weekly loop with Rúmil's project profiles for bounded
changed-code review, consuming existing CI/lint/test/security receipts where possible.
Compilation and tests can execute project code/build scripts: place those behind
existing project authority and isolated worker budgets, not this read-only scanner.
Use the same finding IDs, review decisions and verified-remedy feedback; no second
code-repair scheduler. Track remaining acceptance in the owning daily-loop plan.

## Verification

```sh
python3 -m unittest tests.test_rumil_nightly_hygiene tests.test_rumil_markdown_link_check
cargo test -p arda-engine --test next_action_projection
python3 scripts/rumil_nightly_hygiene.py --report
```

Source tests cover unchanged suppression, strict findings versus execution errors,
whole-tree inventory, symlink/secret/generated boundaries, transient missing links,
move suggestions, partial coverage, resolution/reopening, JSONC exclusion and
review-only Engine consumption without queue or objective writes.
