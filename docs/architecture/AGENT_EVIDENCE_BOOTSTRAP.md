# Agent Evidence Bootstrap Contract

Status: active
Authority: repository evidence, not model recollection

Before describing or changing an Arda subsystem, a fresh agent instance must establish:

1. Ownership: read the relevant crate `INDEX.md`, `README.md`, `OWNERSHIP.md`, `BREAKDOWN.md`, and `STATUS.md` where present.
2. Source wiring: trace public APIs, Cargo dependencies, producers, consumers, tests, and runtime composition.
3. Runtime truth: inspect current services, state projections, append-only ledgers, receipts, and freshness timestamps.
4. Historical boundary: classify archived or Annunimas-named artifacts as compatibility ABI, stale evidence, or still-current authority; names alone do not decide.
5. State labels: distinguish documented, compiled, tested, wired, currently running, workflow-proven, held/degraded, and planned.
6. Completeness: architecture work must account for every relevant distributed authority rather than centering the first crate found.

For governed learning and task automation, the minimum authority set is:

- Varda/Athena: evidence ingest, provenance, comparison, contradiction, and policy-readiness records;
- Vairë: durable memory, correction, supersession, revocation, and rollback history;
- `arda-governance`: action classification and approval authority;
- Aulë/Arandur autopilot: proposal, planning, queue-operation, and observability records;
- canonical universal queue: `core/projects/tasks/queue.jsonl`;
- bounded execution authority and its completion receipts;
- ARDA HUD Operations: human review and active-queue projection, never an independent authority.

No agent may infer a missing capability merely because it has not yet found the owning crate. Plans are consulted only after implemented ownership and runtime evidence have been established.

## Canonical Queue Execution Runtime

Operator-approved queue work is consumed by the bounded Workbench adapter, not
by HUD or by the read-only Aulë timer:

1. `arda-workbench-queue-executor.timer` invokes
   `arda-cli prometheus autopilot execute-approved-task` once per minute.
2. The adapter exclusively locks `core/projects/tasks/queue.jsonl`, replays the
   append-only ledger, validates recommendation and approval packet lineage,
   and appends a deterministic claim with a lease and Workbench run ID.
3. It submits the exact task to the loopback-only engine harness. The engine
   retains project-contract validation, human-approval lineage, worker
   admission, retry budgets, cancellation, Hermes process supervision, durable
   run events, resource usage, and execution receipts.
4. The adapter appends the Workbench receipt digest and terminal state to the
   same canonical task ID, then replay-safely projects the task/run/approval
   lineage into Aulë outcomes, Varda learning, Vairë work memory, and the
   governance execution ledger. A per-run receipt prevents duplicate
   institutional writes after restart. A second poll observes terminal state
   and does not dispatch it again.

Cancellation is explicit through
`arda-cli prometheus autopilot cancel-approved-task <TASK_ID> --reason <TEXT>`.
Failed non-cancelled work may be retried explicitly with `retry-approved-task`;
each retry receives a distinct Workbench run ID while preserving task and
approval lineage. Before dispatch, expired claims are reconciled against the
deterministic Workbench run ID, so an existing running or terminal run wins over
redispatch. HUD Operations displays run, lease, terminal detail, and receipt
lineage and routes cancellation/retry through these CLI authorities.
The installed `arda-aule-autopilot-read-only.timer` remains independent and
read-only; it does not execute canonical queue work.

Runtime proof on 2026-08-13 used a temporary approved queue fixture and live
`arda.service` harness. Workbench run `queue-runtime-proof-20260813-v2`
completed through Hermes, persisted the execution receipt under
`data/runs/queue-runtime-proof-20260813-v2/`, appended a terminal queue record,
and returned idle on the duplicate poll. A separate live run,
`queue-runtime-cancel-proof-20260813`, reached `running`, was cancelled through
the canonical harness endpoint, emitted durable `cancelled` events, terminated
its Hermes worker, and appended the matching terminal queue state. These prove
the bounded runtime and cancellation paths; they do not grant global autonomy
or convert unapproved queue records into eligible work.

The first incomplete proof used the obsolete
`arda.hermes.execution_receipt.v1` output contract and stopped at a running
node. It is retained as non-authoritative diagnostic evidence under
`data/archive/runtime-proofs/incomplete-20260813/queue-runtime-proof-20260813/`,
not under active durable runs. The resource ledger contains provider identity,
request count, joules, and cost only; worker output and credentials are absent.