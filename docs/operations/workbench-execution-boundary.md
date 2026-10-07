# Explicit Workbench execution boundary

R05 source qualification, October 7, 2026. This records the first separation step;
the concrete executor still resides in Aule. It is not an installed deployment
or completion of the remaining R04 combined acceptance scenario.

## Ownership and callers

| Boundary | Current caller/owner | Authority and retained state |
| --- | --- | --- |
| Leaf selection, leases, attempts, controls and scheduling | Engine `ObjectiveRuntime` and `ObjectiveStore` | `data/arda/objectives.sqlite3`; callbacks validate the exact objective/leaf/run and retain maintenance-exclusion leases. |
| Context assembly, stage receipt projection and outcome promotion | Engine `objectives/workbench.rs` | Original context bindings in ObjectiveStore; Vairë context-use/outcome ledgers and governed memory. Adapter output alone cannot close a leaf. |
| Runtime admission | Engine `objectives/store/explicit_runtime.rs` implements `ExplicitRuntimeAdmission` | Called before validation, HTTP or Close persistence. The returned lease grants no workspace permission. |
| Retained workspace and recovery authorization | Engine `objectives/workbench/{retained_authorization,recovery_authorization}.rs` | Process-local callbacks and opaque provider leases; bounded recovery windows, mutation fences and retained store identities. These permissions are not serialized work-item fields. |
| Execute, retry inspection, receipt-only reconciliation | `WorkbenchLeafExecution<E>` calls `ExplicitWorkbenchExecution`; Aule's `WorkbenchExecutionAdapter` implements it | Adapter validates exact work-item/graph/receipt bindings and calls the configured Harness. Reconciliation must not dispatch a provider. |
| Node execution and terminal publication | Engine Harness, RunStore and Hermes adapter | `data/runs/<run>/`; journal/checkpoint, stage receipts, original provider authority and deterministic Close. Recovery publication calls existing Close helpers. |
| Compatibility queue execution | Aule's crate-private `WorkbenchQueueExecutor`, queue runner and migration fixtures | Legacy queue/decomposition/continuation code shares helpers with the explicit adapter. It is not an alternate resident intake or scheduling authority. |
| Observability | Aule metrics and projections | Observes canonical outcomes; does not acquire resident admission or workspace authority by reporting them. |

The concrete adapter is constructed by Engine's default
`WorkbenchLeafExecution::new`; tests and other composition code can use
`with_adapter`. Engine recovery/publication paths also consume explicit work
items. Compatibility imports under `prometheus::autopilot` remain valid.

## Extracted contract

[`arda_aule::execution_contract`](../../crates/spine/observability/arda-aule/src/execution_contract.rs)
now owns the work item, receipt reference/outcome, runtime admission/lease,
workspace authorization/recovery-window types and `ExplicitWorkbenchExecution`.
The concrete trait implementation moved from Engine to the Aule adapter. Engine
policy modules import contract types directly, while the default adapter remains
a composition dependency.

The contract contains no HTTP, filesystem, queue, metric or provider operations.
It requires the existing typed Vairë context and shared serialization/future
types. Its defaults preserve the previous behavior: no reconciliation result,
no provider lease, and refusal of unsupported retained execution. Unknown wire
fields remain rejected. No receipt schema, digest algorithm, authority grant,
retry policy, state path or execution order changed.

Feature coverage:

| Build | Coverage |
| --- | --- |
| `arda-aule --no-default-features --features execution-contract` | Contract consumers compile without `full-cli`; inspected dependency graph excludes OpenTelemetry, Varda, Orome and Manwe. Existing Aule base and Vairë dependencies remain. |
| `arda-aule --features full-cli` | Enables the contract plus the existing concrete executor and CLI dependencies. |
| Engine's existing `arda-aule/full-cli` dependency | Unchanged for runtime composition and other Aule consumers; this step does not claim removal of Engine's broad dependency. |

## Validation and next slice

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

Next isolate the explicit HTTP/receipt implementation from legacy queue helpers,
keeping its authority/recovery tests intact, then select the executor owner and
move that implementation. Only after that separation should Engine's `full-cli`
feature requirement and Aule's observer dependencies be narrowed. Moving the
large combined module unchanged would retain its queue and observation coupling.
R05's implementation-ownership exit gate therefore remains open.
