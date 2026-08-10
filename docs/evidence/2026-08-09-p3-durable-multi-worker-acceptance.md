# P3 durable multi-worker acceptance — 2026-08-09

## Verdict

`P3_VERIFIED` for the repository-owned durable orchestration boundary.

This receipt covers canonical worker contracts, deterministic scheduling and joins,
Hermes execution receipts, failed-verification revision lineage, cancellation,
restart recovery, and runtime worker admission. It does **not** claim that a live
hosted-provider session was used: the external-project acceptance uses the bounded,
Hermes-compatible deterministic provider fixture so that mutation, process, receipt,
and recovery behavior is reproducible without accepting worker self-report.

## P3.1 — durable worker graph

- [`run_graph.rs`](../../crates/spine/governance/arda-core/src/run_graph.rs) persists
  optional, backward-compatible worker execution metadata: role, worker/route
  identity and class, prompt digest, toolsets, dependencies, deadline, output
  contract, evidence policy, authority, retry checkpoint, and resource budget.
- Graph validation rejects hidden dependency differences, malformed worker
  identities/contracts, invalid deadlines, and verifier evidence that is not
  project-native and independent.
- [`orchestrator.rs`](../../crates/engine/src/runs/orchestrator.rs) deterministically
  selects ready workers, permits independent parallel nodes, releases joins only
  after parent receipts match, projects worker progress, and reconciles receipts,
  orphans, cancellation, retries, and superseded work after restart.
- [`hermes.rs`](../../crates/engine/src/adapters/hermes.rs) consumes the persisted
  worker route and toolset declaration, rejects tool escalation and elapsed
  deadlines before spawn, bounds child lifetime, and emits canonical tool, test,
  artifact, usage, and lineage receipts. Cost is explicitly classified as
  `observed`, `estimated`, or `unknown`.
- [`runs.rs`](../../crates/engine/src/harness/runs.rs) persists `Running` before
  provider I/O, propagates canonical cancellation into the live child, prevents a
  late child result from overwriting cancellation, recovers orphaned attempts, and
  exposes node/run worker progress from canonical state.

## P3.2 — external Rust project acceptance

The retained acceptance is
`clean_rust_repository_completes_approved_vertical_slice_with_one_run_id` in
[`workbench_rust_golden.rs`](../../crates/engine/tests/workbench_rust_golden.rs).
It copies the Rust fixture into a temporary directory outside the Cargo workspace
and exercises the bounded provider process against that external project.

Evidence retained by the test:

1. planner/approval lineage unlocks a separately identified implementer;
2. implementer attempt 1 fails without mutation, is durably marked failed, and is
   retried with an incremented checkpoint and recovery token;
3. implementer attempt 2 changes only the approved project file and emits its own
   canonical Hermes receipt;
4. the independent verifier receives the implementation receipt as input and uses
   a separate worker/route identity;
5. an actual project-native `cargo test` failure is retained as a verification
   receipt and blocks review;
6. revision returns the verifier through `Failed -> Ready -> Running`, increments
   checkpoint lineage, and keeps the failed-verification receipt in
   `parent_receipts` rather than overwriting it;
7. project-native `cargo test` then passes and the verifier emits a distinct
   canonical receipt;
8. an independent worker is durably run and cancelled, and final worker progress
   preserves that cancelled state;
9. the store is closed and reopened before review/close; recovered state preserves
   the succeeded implementation, revised verifier, cancelled worker, and complete
   event history;
10. the final artifact digest and concise projection are produced only after the
    verifier succeeds.

The test asserts seven succeeded graph nodes, one cancelled node, at least 22
canonical transitions, distinct implementer/verifier receipts, retained
failed-verification lineage, one mutation, and one restart recovery.

## P3.3 — worker admission and budgets

- [`worker_orchestration.toml`](../../config/runtime/worker_orchestration.toml)
  versions hard total/local/hosted concurrency and run/cycle/daily cost and energy
  limits as `arda.worker-limits.v1`.
- Runtime provider dispatch calls the deterministic scheduler before spawn. Active
  provider route registrations enforce total/local/hosted concurrency without a
  check/insert race because admission and registration occur under the Workbench
  mutation lock.
- Remaining attempts reserve each node's declared maximum cost and energy, so a
  retry cannot bypass run, orchestration-cycle, daily, or energy limits.
- Local admission consults Manwë `/healthz`; thermal pressure and explicitly
  degraded route signals block or queue the worker instead of silently switching
  to a more expensive route.
- Completed provider usage is appended to the canonical resource ledger. Missing
  provider/cost attribution remains visibly fallback/unknown rather than being
  treated as independently observed evidence.
- [`worker_orchestrator.rs`](../../crates/engine/tests/worker_orchestrator.rs)
  proves independent parallel selection and joins, total/local/hosted caps,
  unavailable local and degraded route behavior, deadline blocking, run/cycle/daily
  cost and energy exhaustion, retry reservation, receipt reconciliation, orphan
  retry, and duplicate-success prevention.

## Verification

Passed from the canonical repository root on 2026-08-09:

```text
cargo fmt --all -- --check
  passed

git diff --check
  passed

cargo test -p arda-core --test run_graph
  8 passed, 0 failed

cargo test -p arda-engine
  137 passed, 2 ignored, 0 failed
  ignored: the subprocess-only boundary helper and authenticated live-provider test

cargo test -p arda
  6 passed, 0 failed

cargo clippy -p arda-engine --all-targets --no-deps -- \
  -A clippy::unnecessary-to-owned -A clippy::len-zero -D warnings
  passed

RUSTDOCFLAGS='-D warnings' cargo doc -p arda-engine --no-deps
  passed
```

The two Clippy allowances are the plan's documented unrelated findings; no broader
warning suppression was used. The phase remains bounded to durable repository-owned
orchestration and does not imply live hosted-provider availability or P4 council
acceptance.
