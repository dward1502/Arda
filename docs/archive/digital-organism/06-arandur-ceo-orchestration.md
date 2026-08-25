---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "complete"
  reviewed: "2026-08-23"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: complete | reviewed: 2026-08-23

# Stage 6 — Arandur/CEO Unified Orchestration

## Objective

Turn the existing Prometheus/Arandur planner, council, queue analysis, proposal, observability, and review surfaces into one bounded organism-level executive loop instead of a detached autopilot subsystem.

## Role definition

Arandur is the organism’s executive/orchestrator persona and policy consumer. It does not own conversation, memory, task storage, model routing, transport, evidence, governance, or execution. It reads their canonical projections, proposes the smallest composition, watches receipts, reassesses outcomes, and asks the operator when judgment or authority is required.

## Work packets

### S6.1 — Audit and collapse current CEO surfaces

Trace `arda-aule` CEO/autopilot modules, Prometheus service/library code, Arandur recommendation ledgers, HUD review commands, timers, queue analyzer, council, heartbeat, orders, planner, and archived wrapper decisions. Classify duplicate/stale paths before changing code.

### S6.2 — Define one executive cycle

```text
observe current organism state
  → recover active objective/context
  → identify one meaningful gap or next action
  → compose the smallest capable roles
  → governance and resource assessment
  → propose or dispatch through existing authority
  → monitor receipts and failures
  → assess acceptance conditions
  → emit operator update and learning candidate
```

The cycle must be restart-safe and idempotent.

### S6.3 — Use canonical topology and placement

Arandur requests roles/capabilities; it does not select raw endpoints or hard-code models. Manwë/node placement returns a receipted decision. Arandur may challenge or request review but cannot bypass it.

### S6.4 — Integrate council/MoA deliberately

Council policy is disabled by default. Use critic-only for named risk, adjudication for material unresolved tension, and full deliberation only for explicitly approved cases. Persist opinions and tensions as advisory evidence. Discussion never marks a task approved.

### S6.5 — Unify operator interaction

Hermes carries the live conversation. Arda/HUD presents the same recommendation ID, reason, evidence, resource impact, authority class, proposed workers/nodes, and approve/reject/defer controls. Decisions append to one recommendation/approval lineage.

### S6.6 — Prove assess-and-replan

Run one objective whose first composition encounters a real failed assumption. Arandur must cite the failure receipt, revise the plan within existing authority, avoid repeating failed work blindly, and present the revised next action.

## Verification

- no duplicate CEO/queue/recommendation authority;
- read-only cycle creates no task/commitment;
- reviewed cycle appends one canonical decision;
- restart/idempotency fixture;
- bounded council and resource budgets;
- failed assumption triggers evidence-backed replan;
- operator can understand and stop the cycle.

## Exit gate

Arandur completes one full `review → record → plan → execute → assess` organism cycle through canonical authorities, with node/worker placement and recovery receipts visible through Hermes and the HUD. The operator confirms the behavior reflects the intended executive role.

## Engineering evidence (2026-08-22)

Implementation is complete and the explicit operator-acceptance gate was satisfied on 2026-08-23.

- `arda.arandur.executive_cycle_receipt.v1` records one governed decision per stable `(cycle_id, phase)` key, rejects conflicting replays, and uses a cross-process lock plus append-only JSONL.
- The receipt cites objective/context, recommendation, approval/governance, requested role capabilities, placement handoff, execution outcome, failure/replan, council mode, resource use, and operator-facing next actions without storing provider or node identities.
- Prometheus/Aulë projects this receipt in the canonical `CycleReport`; a live read-only run reported `no_selected_objective`, performed no ledger append, and preserved the review gate.
- The forced-failure fixture launches a real worker process, terminates it, records the direct process observation, replans using role/capability requests, reopens the store, and proves byte-stable replay with one durable ledger row.
- Evidence: `.hermes/evidence/digital-organism/stage6-arandur-ceo-cycle-receipt.json` and `.hermes/evidence/digital-organism/stage6-read-only-cycle.json`.
- Verification: 4 Stage 6 integration tests passed; 29 existing runner tests passed; `cargo build -p arda-aule --features full-cli` passed.
- Operator acceptance: the operator explicitly approved the bounded executive behavior after reviewing the implementation and evidence summary.
- Truth boundary: source integration, focused tests, build, a live read-only projection, and operator acceptance are proven. The installed timer remains read-only and currently holds all candidate objectives at their review gates; productive autonomous execution is not claimed.
