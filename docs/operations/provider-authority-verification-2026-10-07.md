# R06 admission and provider-receipt verification — October 7, 2026

Follow-up: [durable route receipt source implementation](provider-route-receipts-2026-10-07.md) addresses the persistence finding below. Installed rollout and the spending gate remain open.

Verification passed for the tested authority boundaries, but **P2/P3 and the
canonical budget/fallback gate remain open**. Actual upstream route identity is
checked during execution but not retained in the typed execution receipt. Unknown
billing is disclosed, but does not prove compliance with a hard spending ceiling.

## Executed checks

Source baseline: `9ef4444fbcd08bca15aab629458da81e2685f19a`, including the existing
working-tree Engine changes. No source behavior was changed in this verification.
Exact inspected source hashes and logs are under
`target/qualification/r06-authority-20261007/`.

| Check | Result |
| --- | --- |
| `cargo test --offline -p arda-engine --test hermes_adapter_contract -j 4` | 31 passed, 14 ignored. Covers missing approval before spawn, tool authority, task/receipt binding drift, observed evidence, deadlines and process termination. |
| `cargo test --offline -p arda-engine --bin arda-snapshot-worker inference_ -j 4` | Seven passed; ten unrelated tests filtered out. Covers pinned profile policies, Rust/Python agreement, real loopback HTTP forwarding, cancellation, retry-error handling and rejection of mismatched success attestation. |
| Explicit ignored `keeper_adapter::keeper_store_admission_chat_export_reopen_and_durable_release` | One passed using real user/mount namespaces and a fixture Hermes executable. Exercises store admission, retained execution/export, reopen, cancellation and durable release. |
| Installed health/catalog | Both HTTP 200; root PID 973416 and Manwe PID 1356962 unchanged, root restart count zero; mesh services inactive. |
| Completed M4 receipts | All 12 inspected receipt files retained identical SHA-256 before/after verification. No canonical run was resumed or mutated. |

The default suite's remaining ignored tests require additional namespace/recovery
fixtures or installed Hermes with approved live local inference. They were not
silently counted as passing. This run made no production model requests. The
temporary build directory was removed afterward (1,171,681,280 allocated bytes).

## Requirement-to-receipt trace

1. `WorkerExecutionSpec` carries role, worker/route labels, prompt digest,
   toolsets, deadline and evidence policy; the node separately carries cost/energy
   budgets. These labels are not themselves upstream Manwe route attestations.
2. `HermesNodeTask::authority_binding_digest` binds the complete admitted task,
   excluding mutable node execution progress. The adapter rejects absent approval,
   inappropriate tools and stale task/receipt authority.
3. The retained broker reads the captured profile through a retained descriptor.
   `inference_policy.rs` requires a fixed local gateway, explicit provider/model
   and local/cloud origin, correct local-only setting, tools, and fallback disabled.
   Payload attempts to change destinations, model, routing or arbitrary fields
   are rejected. This is pinned-profile enforcement, not proof of requirement-based
   provider selection or an authorized canonical fallback implementation.
4. `inference_broker.rs` verifies successful response headers for actual route ID,
   provider, model and route class. A provider/model mismatch is rejected before
   the body reaches the payload. Failed 429/503 responses retain sanitized status
   for retries without acquiring success authority. The Python guard independently
   checks successful route headers and emits `ARDA_MANWE_ROUTE` to stderr.
5. `HermesAdapter` constructs its typed receipt from the exported session.
   `split_chat_output` reads session headers from stderr but does not collect
   `ARDA_MANWE_ROUTE`. `HermesExecutionReceipt` has no typed upstream-route list;
   canonical stage projection takes `receipt.usage.provider` and `.model`.

Relevant owners: `crates/engine/src/adapters/hermes.rs`,
`crates/engine/src/bin/snapshot_runtime/{inference_policy.rs,inference_broker.rs,bootstrap.py,session_provenance.py,profile_guard.py}`,
`crates/engine/src/objectives/workbench/receipt_validation.rs`, and
`crates/spine/governance/arda-core/src/run_graph.rs`.

## Observed gaps and next implementation

**Durable actual-route evidence:** the completed M4 objective
`operator-objective-c98d83591ddab744` has nine model-backed stage receipts with
provider `retained-manwe`, model `gpt-6-astra`, and cost measurement `unknown`.
Three Close receipts instead identify `arda-workbench/deterministic-close` with
observed cost. None of the 12 has a typed actual-route field. The wrapper identity
is truthful configured provenance; it must not be relabelled as an observed
upstream provider. Next preserve broker-verified route evidence through the
existing adapter/receipt owner and validate its binding during canonical replay.
Keep historical receipts and their original digests intact.

**Spending authority:** normalization maps absent billing to numeric zero plus
`CostMeasurement::Unknown`. The existing test explicitly permits an unknown-cost
receipt. The adapter compares normalized reported cost with the node ceiling
after execution; the broker's captured route policy does not receive a monetary
reservation or cumulative retry-spend limit. Therefore unknown is not observed
free, and retrospective rejection is not a pre-spend cap. A canonical hard-budget
claim needs explicit treatment of unpriced routes plus request/retry accounting
before further spending is admitted. This verification did not implement that policy.

Finish durable upstream-route binding first, then verify canonical budget and
fallback handling under an isolated admitted task. Preserve the accepted M3/M4/M5
records; do not mark G2 complete from the passing source and fixture tests above.
