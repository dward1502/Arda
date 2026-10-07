# Explicit Workbench execution boundary

R05 source qualification, October 7, 2026. The explicit executor now resides in
`arda-workbench-executor`, outside Aule. This is source qualification, not an
installed deployment or completion of the remaining R04 combined acceptance scenario.

## Ownership and callers

| Boundary | Current caller/owner | Authority and retained state |
| --- | --- | --- |
| Leaf selection, leases, attempts, controls and scheduling | Engine `ObjectiveRuntime` and `ObjectiveStore` | `data/arda/objectives.sqlite3`; callbacks validate the exact objective/leaf/run and retain maintenance-exclusion leases. |
| Context assembly, stage receipt projection and outcome promotion | Engine `objectives/workbench.rs` | Original context bindings in ObjectiveStore; Vairë context-use/outcome ledgers and governed memory. Adapter output alone cannot close a leaf. |
| Runtime admission | Engine `objectives/store/explicit_runtime.rs` implements `ExplicitRuntimeAdmission` | Called before validation, HTTP or Close persistence. The returned lease grants no workspace permission. |
| Retained workspace and recovery authorization | Engine `objectives/workbench/{retained_authorization,recovery_authorization}.rs` | Process-local callbacks and opaque provider leases; bounded recovery windows, mutation fences and retained store identities. These permissions are not serialized work-item fields. |
| Execute, retry inspection, receipt-only reconciliation | `WorkbenchLeafExecution<E>` calls `ExplicitWorkbenchExecution`; `arda_workbench_executor::WorkbenchExecutionAdapter` implements it | Adapter validates exact work-item/graph/receipt bindings and calls the configured Harness. Reconciliation must not dispatch a provider. |
| Node execution and terminal publication | Engine Harness, RunStore and Hermes adapter | `data/runs/<run>/`; journal/checkpoint, stage receipts, original provider authority and deterministic Close. Recovery publication calls existing Close helpers. |
| Compatibility queue execution | Aule's crate-private `WorkbenchQueueExecutor`, queue runner and migration fixtures | Legacy queue/decomposition/continuation code consumes the executor’s shared graph, receipt and project-binding helpers. It is not an alternate resident intake or scheduling authority. |
| Observability | Aule metrics and projections | Observes canonical outcomes; does not acquire resident admission or workspace authority by reporting them. |

The concrete adapter is constructed by Engine's default
`WorkbenchLeafExecution::new`; tests and other composition code can use
`with_adapter`. Engine recovery/publication paths also consume explicit work
items. Compatibility imports under `prometheus::autopilot` remain valid.

## Executor ownership and compatibility

[`arda-workbench-executor`](../../crates/spine/executors/arda-workbench-executor/README.md)
owns the explicit adapter and contracts. Its `execution` module performs HTTP
execution, authorization rechecks, receipt-only reconciliation and deterministic
Close. Runtime-admission and workspace-authority tests moved with that code.
The crate has no dependency on Aule, telemetry, Manwe or Varda.

Its default build provides only the work item, receipt reference/outcome,
admission/lease, workspace authorization/recovery-window types and
`ExplicitWorkbenchExecution`. The `runtime` feature enables execution and
work-item receipt/payload operations. Runtime dependencies include Core project
and run contracts, Vaire context and Orome's typed handoff receipt.

Engine imports this executor directly. Its remaining Aule calls render presence
projections, so its Aule dependency no longer enables `full-cli`. Aule's old
`execution_contract` and `prometheus::autopilot` paths re-export the same types
for compatibility. The existing contract tests exercise those aliases.

Legacy queue scheduling, continuation writers, locks and migration fixtures stay
in Aule. A small `compatibility` surface in the executor owns their shared graph,
receipt and project-binding operations. `ExecutionGraphLimits` projects the
legacy planner's limits into that surface; the executor does not import the
planner or queue types. The historical graph fields and provenance strings,
receipt schemas/digests, authority checks and execution order are unchanged.

| Build | Coverage |
| --- | --- |
| `arda-workbench-executor --no-default-features` | Contracts without executor HTTP/receipt I/O. |
| `arda-workbench-executor --features runtime` | Concrete executor without Aule, OpenTelemetry, Manwe or Varda. |
| `arda-aule --features execution-contract` | Compatibility contract exports. |
| `arda-aule --features full-cli` | Legacy queue/CLI plus executor compatibility exports. |
| Engine | Direct executor runtime dependency; Aule presence projections with no `full-cli` requirement. |

## First extraction validation

Before extraction, the existing executor suite passed 85 tests. Three new
contract-only integration tests check wire defaults/receipt shape, rejection of
serialized authority and safe defaults without dispatch. Existing runtime-denial,
workspace-binding, recovery, Close and replay tests remain at the adapter/Engine
boundaries rather than being replaced with mocks of the new interface.

Commands and exact results are retained in
`target/qualification/r05-boundary-20261007/`. The initial broad run's fixture
Git-admission failures are retained separately: a `.git` marker above `/tmp`
fixtures prevented admission. Host reruns use `TMPDIR=/var/tmp/arda-r05-tests`;
no admission guard was disabled and no operator Git metadata was removed.

Final library qualification: Aule 391 passed; Engine 216 passed, 9 ignored.
Restart-continuity integration: 12 passed, 5 ignored.
The contract-only suite passed 3 tests; contract-only compilation, dependency
inspection, strict library Clippy and workspace formatting passed. These checks
ran against the current working tree, including pre-existing uncommitted work;
the commit contains only the boundary extraction and acceptance documentation.

The first retry exhausted disk space. Package-scoped Cargo debug cleanup freed
54.4 GiB without deleting installed binaries or qualification artifacts. That
cleanup also removed the generated snapshot worker used by five Engine tests;
rebuilding it restored the prerequisite and the full Engine library rerun passed.
The failed attempts remain in the evidence directory.

## Executor extraction qualification

Evidence is retained in `target/qualification/r05-executor-20261007/`.
The executor and Engine dependency trees independently verify the separation.
Final working-tree results: Aule 364 passed, Engine 216 passed / 9 ignored,
executor 27 passed; restart continuity 12 passed / 5 ignored. The executor's
contract-only tests and Aule compatibility tests each passed 3 tests. Strict
library Clippy and workspace formatting passed.

A separate checkout containing exactly the staged source compiled Aule, Engine
and the executor together. Its executor runtime suite passed 25 tests and its
contract suite passed 3. The two additional working-tree executor tests belong
to the pre-existing dependency-report transport work; that work moved with its
owner but remains uncommitted. The staged Engine check reports its existing
unused `independent_critic_review` helper warning. The strict Clippy result above
applies to the working tree. The temporary checkout was removed after validation.
Sharing its Cargo target directory caused the first root consumer check to reuse
a staged Core artifact lacking the working tree's pending workspace-policy module.
A package-scoped Core artifact cleanup forced the final root check to rebuild it;
no source or policy workaround was introduced. The root consumer check passed.

This closes R05's source ownership/feature boundary and regression gate. No
installed binary was replaced or daemon restarted. Continue the reliability
list at R06 provider and communication qualification; the accepted bounded M5
result and remaining R04 combined-scenario obligations are unchanged.
