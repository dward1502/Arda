---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "audit"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-25"
  tags: ["autonomy", "task-loop", "providers", "projects", "soterion"]
---

> 🜏 Soterion: 📜 audit | owner: RUMIL | status: active | reviewed: 2026-08-25

# Autonomous System Gap Audit

## Operator intent

Arda is not complete when it can demonstrate a handful of conversational actions. It must turn operator intent and discovered system needs into governed work, choose among local and hosted capability, execute against connected projects, inspect its own results, continue until acceptance criteria pass, and ask the operator only when authority, ambiguity, cost, or risk genuinely requires a decision.

It must also improve itself and the operator's connected projects every day by combining current internet research, local repository evidence, prior plans, durable memory, and verified execution—not by producing reports that never become completed work.

## Verified current state

### The intended organism architecture already exists

The active architecture is not a blank queue-centered design. `ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` defines capability composition rather than application silos. `DIGITAL_ORGANISM_AUTHORITY_TRANSPORT_MAP.md` assigns distinct authority to Core/Engine objectives and runs, Hermes sessions and agent execution, Manwë provider routing, Oromë/A2A handoff, Varda/Warden evidence, Vairë memory, Aulë observability, Rúmil lifecycle audit, governance approvals, systemd/outpost execution, and typed receipts.

The correction is to compose existing owners through their declared contracts. It is not to replace them with a new universal queue service or make one crate absorb every operation.

### Crate-owned operations already exist

The repository contains substantial domain operations, including Aulë/Prometheus planning and execution-intent transitions, Varda ingestion/deep-queue/external-evidence evaluation, Vairë memory and continuity operations, Rúmil project and maintenance audit surfaces, Mandos reasoning/validation, Manwë provider routing and health, Oromë work-envelope/A2A routing, governance evaluation, and Engine RunStore recovery and transition enforcement.

These are real implementations, not merely names. The unproven part is their production composition: the installed path does not yet demonstrate that discovered work flows through the correct domain owner, canonical objective/run lineage, placement, verification, review, continuation, and terminal receipt without a human repeatedly reconnecting the stages.

### Canonical task queue and scheduler

- `core/projects/tasks/queue.jsonl` is the canonical task ledger.
- `arda-workbench-queue-executor.timer` runs the installed executor every minute.
- The installed service executes `arda-cli prometheus autopilot execute-approved-task` with a 25-minute bound.
- Recent service receipts were `idle/no_eligible_task`; the timer is alive, but no approved task was eligible.
- The executor selects only queue records already marked `approval.status = approved` and eligible for the `arandur` lane.

### Execution graph mismatch

The engine supports durable `plan → approval → execute → verify → review → close` graphs, retries, failed-verification recovery, restart recovery, cancellation, and receipt lineage. The installed queue executor does not construct that graph. It creates only `plan → approval → execute`, uses a fixed Arda project ID, runs one Execute node, and marks the queue record completed when that node succeeds.

Therefore a successful queue receipt currently means that one adapter invocation succeeded. It does not prove independent verification, review, acceptance-criterion closure, defect correction, or autonomous continuation until the problem is finished.

### Task creation mismatch

- The Phase 1 deterministic planner knows only five hard-coded goal IDs.
- Knowledge triage deliberately produces review records and does not mutate the task queue without an explicit pivot.
- The three pending operator objectives are review/future-gated and therefore cannot be selected by the installed executor.
- No general decomposition path turns an operator objective or discovered defect into a dependency-aware sequence of executable, verifiable queue records.

### Provider mismatch

- Manwë is reachable on `127.0.0.1:7171` and reports 22 configured providers.
- At inspection time, 3 were ready: two local routes (`edge_core`, `edge_beelink_light`) and the subscription-backed `openai_sub` route. Three enabled local routes were unhealthy, two enabled cloud routes lacked credentials, and the remaining routes were disabled.
- Arda's adaptive-placement endpoint can select worker/critic/adjudicator roles from Manwë's live provider and mesh projections and execute through Manwë.
- The installed Workbench queue executor instead launches `hermes chat` without a provider or model override. The active Hermes default is `openai-codex`, so canonical queued work does not consume Manwë's local-first placement decisions.
- Hermes delegation uses `custom:arda-manwe`, but that separate configuration does not make the Workbench queue executor adaptive.
- Several Hermes auxiliary routes still target `127.0.0.1:5110`; no service was listening there during inspection. Manwë was listening on `7171`.

### Daily research mismatch

Repository unit files define daily Warden internet research and repository-survey timers. They were not installed in the active user systemd unit set. The survey unit also submits `%h/Arda`, not the canonical `%h/Eregion/Arda` root. No current daily path was found that takes research, compares it with project needs, promotes bounded work, executes a change, verifies it, reviews it, and continues later if incomplete.

The installed read-only Aulë autopilot is not a substitute. Its latest output remained blocked by stale/missing preflight evidence and it does not own the canonical queue.

### Project connection mismatch

Repository discovery returned 20 `arda-project.json` paths under `/var/home/mythos/Eregion`, but they collapse to three language fixtures in `crates/engine/tests/fixtures/workbench/` plus copies in Arda worktrees. No root-level manifest was found for the directly discovered sibling repositories. `data/workbench/projects.json` is nevertheless a real existing project-contract registry with three staged/proof records. The contract mechanism and prior work must be preserved rather than described as absent or replaced by a newly invented registry.

The gap is portfolio coverage and operational use: the directly discovered projects are not represented by root-level manifests, the Workbench registry currently contains staged/proof records rooted at `.`, and no demonstrated cross-project run shows project contracts driving bounded execution and verification end to end.

### Soterion's actual role

Soterion is the machine-readable YAML metadata header used on Markdown and, where useful, selected source files such as `.rs` and `.ts`. Its purpose is to make grep, indexing, filtering, ownership lookup, and system discovery faster. It labels material so ordinary source/search tools can find the right files without repeatedly scanning everything blindly.

Soterion is not the authority for completeness, task decomposition, project registration, evidence, memory, or execution. Those remain with their owning contracts and crates. Any Soterion work in this program is limited to consistent header coverage, validation, and fast metadata-assisted discovery; deeper text and symbol inspection continues through repository search and source reads.

### AIPKG and external assimilation already exist

The architecture already anticipates adopting other people's open-source work. `arda-core::external_capability` defines a strict adapter/service boundary with source and SBOM digests, capabilities, protocols, data classes, resource/retry/health limits, provenance, secret references, and a ceiling that prevents external code from owning task, memory, or governance authority. `arda-core::aipkg` defines package identity, provenance, license, capability, permission, digest/signature, resource, and preflight/execute/validation receipt checks. `Task` carries an optional `aipkg_manifest`, and `dispatch_full` validates it before allocating execution resources. `arda-engine::adapters::assimilation` defines a restart-safe lifecycle from discovery and evidence collection through need matching, isolation, measured trial, proposal, governance, landing or adapter retention, verification, rollback, and removal. Its nightly policy explicitly allows bounded comparison, isolated fixtures, reports, patches, tests, and adoption proposals while denying silent installation, authority expansion, private-data mutation, and consequential merge.

This means the operator's model is correct, with three complementary layers: the external-capability contract bounds adapters and services; AIPKG is a governed portable-package and task-dispatch contract; assimilation is the broader adoption lifecycle for a repository, package, adapter, service, tool, or idea. The present gap is operational wiring before and after the existing validation gates. Search found the assimilation lifecycle exercised in `crates/engine/tests/assimilation.rs`, but not a production coordinator that acquires a candidate, chooses the correct integration form, prepares the relevant contract, runs the isolated trial, submits the governance decision, lands the adapter or implementation, verifies it, and retains rollback/removal evidence.

### Planning mismatch

`CORE_ARDA_USEFULNESS_REPAIR.md` promoted a seven-step human journey to the primary product gate. Those interactions can be useful acceptance evidence, but they are not the operator's requested system goal. The rejected framing must not continue as active authority.

## Root causes

1. Implemented components and crate-owned operations were accepted independently without wiring their existing authorities into one production loop.
2. Test-complete graphs and archived program stages were treated as system completion even when the installed executor used a narrower path.
3. Queue, project contracts, provider placement, research/evidence, external assimilation, and review each acquired separate partial flows; Soterion can help locate their files but does not connect their runtime authority.
4. Plans optimized for demonstrable slices rather than the operator's end state: a system that owns bounded work through completion.
5. Runtime installation drift was not continuously reconciled against repository intent.

## Required correction

The active program must make one canonical loop real:

`observe/capture → retrieve context → define outcome → decompose → authorize → schedule → place → execute → verify → review → revise/retry → close → learn → report`

The loop must support both operator-authored objectives and discovered improvement opportunities. It must preserve explicit approval for consequential actions while allowing policy-approved local analysis, planning, testing, and reversible code work to advance without daily prompting.

## Evidence boundary

This audit establishes planning evidence only. The new plans are not implementation, and none of the missing behavior may be claimed complete until exercised through the installed runtime against real connected projects with durable receipts and operator-visible outcomes.
