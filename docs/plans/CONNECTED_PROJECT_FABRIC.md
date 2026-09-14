---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-25"
  tags: ["projects", "registry", "audits", "portfolio", "soterion"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: RUMIL | status: active | reviewed: 2026-08-25

# Connected Project Fabric

## Outcome

Arda knows which projects the operator owns, what each is for, how they relate, what state and plans are authoritative, what work is incomplete, how each may be inspected or changed, and what proves completion. Every approved project can participate in the same task, memory, scheduling, provider, review, and receipt loop.

## Verified starting point

Search found 20 `arda-project.json` paths, but they are three Workbench language fixtures plus copies in Arda worktrees—not 20 unique portfolio contracts. `data/workbench/projects.json` already provides a real Workbench project-contract registry with three staged/proof records rooted at `.`. Several real repositories are dirty and must be treated as operator-owned work in progress until inspected. The task is to extend and activate this prior mechanism, not invent a replacement registry.

The initial discovered set is:

- Arda
- Arda-Agent-Loop-Contract
- Arda-Council
- Arda-Forge-Mind
- Arda-HUD
- Arda-Human
- Arda-Service-Registry
- Arda-Signal-Grid
- Arda-Tool-Gate
- CoverCoINC
- filamentDB
- ravensnestweb
- realmgateWarriors
- samsy-ninja-test
- signal-router
- skylightpros
- wakita
- wgtt

Discovery is not attachment and this list does not imply permission to mutate every repository.

## Project record

Each connected project needs:

- stable ID, name, purpose, class, owner, and lifecycle status;
- canonical root and repository identity;
- relationships to Arda and other projects;
- authoritative requirements, plans, issue sources, and documentation;
- languages, adapters, build/test/lint/run commands, and artifacts;
- acceptance criteria and human-visible outcomes;
- read/write/network/secret/deployment authority;
- protected paths and dirty-worktree policy;
- rollback and recovery strategy;
- current risks, stale assumptions, unfinished work, and next objective;
- Vairë memory namespace and Soterion indexing scope.

## Implementation sequence

### F1 — Discovery and classification

Build a read-only inventory command for declared roots. Detect Git state, remotes, manifests, build systems, docs/plans, services, deployments, and existing automation. Classify projects as core Arda, Arda satellite, personal, business, experiment, archived, or excluded.

### F2 — Reconcile existing contracts

Compare any existing root-level `arda-project.json`, the Workbench registry, and live repository evidence. Generate a draft where no real project contract exists. Never promote test/worktree fixtures into portfolio authority, and never invent commands or authority. Dirty repositories default to read-only. Missing or stale acceptance criteria become explicit review items.

### F3 — Operator review and attachment

Present contracts in coherent batches. The operator approves project identity and consequential authority once; routine work then follows that policy. Attach through the loopback Workbench API with durable approval/idempotency evidence.

### F4 — Converge production attachment

Retain valid project structures, separate proof/demo records from production authority, and converge Workbench attachment on canonical manifest identity. Bind queue tasks to actual project IDs and roots. Verify adapter commands run in the declared repository rather than `.` by accident.

### F5 — Rúmil audit cycle

For each connected project, Rúmil periodically compares:

- operator-authored purpose and requirements;
- active plans and queue state;
- source, tests, runtime/deployment state, and open failures;
- prior receipts and unresolved reviews;
- dependency/security/toolchain drift;
- current external approaches relevant to actual goals.

It emits a cited current-state record, identifies stale or falsely completed work, and promotes bounded outcomes into the canonical completion loop according to policy.

### F6 — Cross-project graph

Record dependencies and shared contracts so a change in one repository can create ordered tasks in others. Require compatibility checks at boundaries. Do not clone code or create parallel authorities merely to claim connection.

### F7 — Soterion metadata coverage

Apply Soterion as the intended machine-readable YAML header convention:

- add or correct headers on Markdown and selected `.rs`/`.ts` files where metadata materially improves discovery;
- validate the declared fields and keep them grep-friendly;
- use metadata filters to narrow ordinary file, text, and symbol searches;
- follow the located source to its owning contract and live evidence before making decisions;
- never treat a Soterion label or index entry as proof of completeness, freshness, approval, or runtime truth.

## Rollout waves

1. **Core system:** Arda, Arda-HUD, Arda-Agent-Loop-Contract, Arda-Council, Arda-Service-Registry, Arda-Signal-Grid, Arda-Tool-Gate.
2. **Supporting intelligence/data:** Arda-Forge-Mind, Arda-Human, filamentDB, signal-router.
3. **Operator business/personal projects:** CoverCoINC, ravensnestweb, realmgateWarriors, skylightpros, wakita, wgtt.
4. **Experiments:** samsy-ninja-test and any subsequently discovered experimental roots.

Wave membership must be corrected after repository-purpose inspection; names alone are not authority.

## Acceptance

- Every in-scope project has an approved truthful contract or an explicit exclusion reason.
- A cross-project objective decomposes into correctly rooted tasks and passes each repository's checks.
- Dirty operator work is preserved.
- Soterion metadata narrows discovery to the relevant owned files, after which canonical sources provide task, receipt, project, and runtime truth.
- Rúmil finds at least one real stale/incomplete claim and carries its correction through verified closure.

## Done

The fabric is complete when project connectivity changes what Arda can correctly plan, execute, resume, and verify—not when a registry merely lists repository names.
