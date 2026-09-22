---
soterion:
  sigil: "SCROLL"
  glyph: "[scroll]"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "RUMIL"
  status: "in_progress"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "audits", "portfolio", "soterion"]
---

> Soterion: [scroll] implementation_plan | owner: RUMIL | status: in_progress | reviewed: 2026-09-17

# Connected Project Fabric

## Outcome and evidence boundary

Arda knows which projects are in scope, their purpose and relationships, exact
roots, authority, current requirements and acceptance. Approved projects participate
in the same scheduling, execution, memory and review loop; discovery grants no
mutation authority.

Partial. data/workbench/projects.json and root-level arda-project.json contracts
already exist. Historical discovery counted proof fixtures/worktree copies, not
unique connected production projects. Old repository lists and rollout waves are
candidates, not a current approved scope. Revalidate declared roots rather than
turning those names into blanket permission. The historical inventory
(docs/plans/archive/2026-09-15-plan-reconciliation/CONNECTED_PROJECT_FABRIC.md)
preserves those candidates without keeping a duplicate active backlog.

## Contract requirements

Reuse the existing Workbench registry and contract shape. Each selected project
must have stable ID/name/purpose/class/owner/lifecycle, canonical root and repository
identity, relationships, authoritative requirements/plans/issues, language/adapters,
exact build/test/lint/run commands and artifacts, human-visible acceptance,
read/write/network/secret/deployment authority, protected paths and dirty-worktree
policy, rollback/recovery, risks/current next objective, and scoped Vairë/Soterion
references. Missing information is a review item, not an invented field value.

## Work checklist

- [x] F1 -- Inventory explicitly declared roots read-only: Git state/remotes, manifests,
  docs/plans, services/deployments and automation. Deduplicate physical roots and
  worktrees; classify core/satellite/personal/business/experiment/archive/excluded.
  Obtain an explicit in-scope list and exclusion reasons before expanding the fabric.
  (Completed: docs/plans/archive/2026-09-17-f1-inventory/EREGION_PHYSICAL_ROOT_INVENTORY.md)

- [x] F2 -- Reconcile existing manifest and registry identities against actual source
  and commands. Draft only missing contracts; preserve dirty work as read-only until
  its ownership and approved mutation scope are understood.
  (Completed for 3 business apps: docs/plans/archive/2026-09-17-f2-reconciliation/BUSINESS_APPS_RECONCILIATION.md;
  Completed for 14 remaining projects: docs/plans/archive/2026-09-17-f2-reconciliation/REMAINING_14_RECONCILIATION.md)

- [x] F3 -- Review identities and consequential authority in coherent operator batches;
  attach through existing Workbench authority with approval/idempotency receipts.
  No direct registry rewrite or discovery-as-approval shortcut.
  (Completed for 3 business apps:
  docs/plans/archive/2026-09-17-f3-contracts/BUSINESS_APPS_CONTRACTS.md;
  attached to registry (25 entries);
  Completed for 14 remaining projects:
  docs/plans/archive/2026-09-17-f3-contracts/REMAINING_14_CONTRACTS.md;
  17 total deferred -- see below)

- [x] F4 -- Separate proof/demo records from production attachment without erasing
  historical lineage. Bind resident leaves to real IDs and roots; prove commands run
  in the declared repository, not an accidental . or proof-copy directory.
  (Completed: docs/plans/archive/2026-09-17-f4-proof-demo-separation/SEPARATION.md)

- [x] F5 -- Feed Rumil project-purpose/requirements/source/test/runtime/receipt and
  dependency-drift comparisons into the existing daily-loop owner. Retain cited
  stale/incomplete findings; Rumil proposes, never authorizes its own mutation.
  (Completed: docs/plans/archive/2026-09-17-f5-rumil-supply/RUMIL_SUPPLY.md)

- [x] F6 -- Record cross-project dependencies/shared contracts and compatibility checks.
  A change may create ordered leaves through canonical authority, not cloned code
  or a competing queue.
  (Completed: docs/plans/archive/2026-09-17-f6-cross-project-deps/CROSS_PROJECT_DEPENDENCIES.md)

- [x] F7 -- Apply/validate Soterion metadata only where it improves discovery; use
  metadata to find owned files then read actual sources. Labels do not prove runtime,
  freshness or approval. Preserve language-valid metadata conventions in code.
  (Completed: docs/plans/archive/2026-09-17-f7-soterion-metadata/SOTERION_METADATA.md)

## Remaining work after F3 closure

F2/F3 for the 14 in-scope projects is **CLOSED 2026-09-17**: contracts drafted,
commands live-verified where possible, and all 14 attached to
`data/workbench/projects.json` (registry 25→37 entries). The per-project notes
below were accurate at survey time and are retained for history; verification
results supersede "needs survey" entries.

### Verification results (2026-09-17, this session)

All 7 Rust crates + Arda-HUD had `cargo test` / `pnpm test` run live against
their canonical roots. Results recorded in the attached contracts' `commands[].verified`
and `checks[].status` fields.

| Project | test command | result |
|---|---|---|
| Arda-Agent-Loop-Contract | cargo test | 2 passed |
| Arda-Signal-Grid | cargo test | 6 passed (2 lib + 4 smoke) |
| Arda-Council | cargo test | 3 passed (3 smoke + 3 doc) |
| Arda-Forge-Mind | cargo test | 7 passed (3 lib + 4 smoke) |
| Arda-Human | cargo test | 3 passed (1 lib + 2 target_local) |
| Arda-Service-Registry | cargo test | 3 passed (3 smoke) |
| Arda-HUD | pnpm test (vitest) | 258 passed, 69 files |

### Deferred beyond F3

- **Build commands not verified** — `cargo build` / `pnpm run build` / `tauri:build:stable`
  remain `verified: null` in the attached contracts. Test passing is a positive signal
  but build is a separate gate.
- **Arda-HUD dirty work** — 4 modified files (`scripts/node_monitor.sh`,
  `src-tauri/Cargo.toml`, `src/lib/ardaBundleTypes.ts`, `src/lib/ardaSource.ts`)
  were stashed on the `refactor` branch as `f3-attachment-20260917` before attachment.
  Stash fate (restore to main, merge, or discard) is operator decision.
- **wakita + ravensnestweb command verification** — RavensNestInc org repos; operator
  confirmed guest access with local-only mutation authority. No local npm test run.
  Contracts record `push: false` and the ownership boundary explicitly.
- **filamentDB + ravensnestweb deeper survey** — filamentDB deeper survey done (src/,
  amplify/, env names identified, yarn confirmed). ravensnestweb still needs per-package
  survey of `apps/web/package.json` and `apps/api/package.json`.
- **relic-kiosk + citadel-avatar** — no git, no buildable commands; contracts reflect
  prototype/runtime-projection status with no verification possible.

### Open gates (see whole-system program)

G3.2, G3.3, G3.4 remain open (see Gate 3 acceptance below). These are tracked in the
whole-system completion program, not re-created here.

### Per-project survey notes (historical)

Detailed per-project survey notes (manifests, git state, env file presence, command
inference) were captured during F2 and are preserved in the archive:
- F2 reconciliation: `docs/plans/archive/2026-09-17-f2-reconciliation/REMAINING_14_RECONCILIATION.md`
- F3 contracts: `docs/plans/archive/2026-09-17-f3-contracts/REMAINING_14_CONTRACTS.md`

The attached contracts in `data/workbench/projects.json` are the live source of truth;
verification results from this session supersede "needs survey" entries in those docs.

## Gate 3 acceptance

- [x] G3.1 -- Every explicitly in-scope project has a truthful approved contract or
  recorded exclusion; fixture/worktree copies are not counted as extra projects.
  (Closed 2026-09-17: 3 business apps (CoverCoINC, wgtt, skylightpros) already attached;
  14 remaining projects contracted and attached in this session --
  `data/workbench/projects.json` 25→37 entries. F1 inventory exclusions preserved.
  M4 fixtures in `data/arda/acceptance/m4/` are acceptance stubs, not counted as
  additional projects.)

- [ ] G3.2 -- The Gate 1 M4 outcome proves correctly rooted cross-project work,
  per-project checks and dirty-work preservation. Reuse that receipt rather than
  running a duplicate scenario. (Not yet verifiable -- M4 fixtures in
  `data/arda/acceptance/m4/` are acceptance stubs: `project-a/acceptance.txt` =
  "resident objective runtime accepted this project." and v2–v5 =
  "resident-objective-acceptance-fixture-ok". No real cross-project outcome with
  rooted work, per-project checks and dirty-work preservation is recorded. G1 has
  not passed; no M4 receipt exists to reuse. Gate 1 (autonomous loop) and the
  whole-system program's next-step sequence step 2–3 are the path to a genuine M4.)

- [ ] G3.3 -- Metadata-guided discovery reaches the right canonical sources and scoped
  memory without implying permission to change them. (F7 completed; needs live verification)

- [ ] G3.4 -- A real stale/incomplete project claim receives an approved correction,
  verification and follow-up audit through the daily-loop owner.
  (docs/plans/DAILY_RESEARCH_IMPROVEMENT_LOOP.md; not yet exercised)

## Exit gate

Approved production project connectivity demonstrably changes planning,
execution, continuation and verification. A registry containing names is insufficient.
Two bounded contracts are enough to unblock M4; broader portfolio rollout is not
an arbitrary prerequisite to the first useful outcome.

The 3 business apps (CoverCoINC, wgtt, skylightpros) are attached and constitute
the first bounded set. F2/F3 for the 14 remaining projects (7 core Arda parts +
3 additional business + 2 internal + ravensnestweb) is drafted but not attached;
reconciliation docs and contracts are in the archive tree. Attachment awaits
operator approval with idempotency receipts.

## Verification and retirement

Use existing registry/attachment/contract validation tests, rooted-command and
idempotency checks, before/after dirty-work evidence and canonical run receipts.
Observe effects under reviewed project authority; do not execute every discovered
repository's build scripts during inventory. Link accepted evidence to the
whole-system program (docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md) and retire the active
plan when its approved scope and gates pass.
