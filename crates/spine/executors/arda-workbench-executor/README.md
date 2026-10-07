# Workbench executor

Owns explicit Workbench HTTP execution, receipt reconciliation, deterministic
Close, and the process-local execution/admission contracts used by Engine.
Engine remains the owner of scheduling, runtime admission, workspace authority,
provider leases and recovery grants. Serialized work items grant no authority.

The default feature set provides the contracts. Enable `runtime` for
`WorkbenchExecutionAdapter` and the work-item receipt/payload operations. The
runtime depends on Core project/run contracts, Vaire context, and Orome's typed
handoff receipt; it does not depend on Aule or its telemetry/queue modules.

Aule retains compatibility exports for existing callers. Its retired queue
adapter consumes the shared receipt, graph and project-binding operations in
`compatibility`; no queue scheduling or persistence moves into this crate.
`ExecutionGraphLimits` projects the existing legacy leaf limits without importing
its planner type. Graph fields, receipt schemas/digests and authority checks are
preserved by the extraction.

Run `cargo test -p arda-workbench-executor --features runtime` for the executor,
authorization, recovery, receipt and contract suites. Host-local sockets are
required. The broader Engine restart tests cover the runtime composition.
See [the execution boundary map](../../../../docs/operations/workbench-execution-boundary.md).
