# Durable provider route receipts — October 7, 2026

The source implementation now retains broker-verified upstream evidence in
`arda.execution-receipt.v4`. Installed Engine/worker rollout and live acceptance
remain separate; this change does not claim the canonical hard-budget gate is closed.

## Evidence ownership

The retained broker hashes the exact captured profile bytes and records each
complete successful HTTP response after validating its route ID, provider, model
and `tool_oriented` class against the captured selection. The worker sends this
structured evidence in its supervisor response, outside payload stdout/stderr.
The adapter requires it for configured Chat and includes it in the receipt digest.
Model-visible `ARDA_MANWE_ROUTE` text cannot supply this evidence.

The shared contract belongs to arda-core; Engine, the Workbench executor and Aule
use the same validation. Exported usage keeps its truthful wrapper provider, while
`inference_routes` separately identifies the observed upstream. Exported model
identity must agree with the verified selection. At 256 successful responses the
broker rejects further requests before sending them, rather than silently dropping
evidence. Error responses and cancelled/incomplete bodies do not become successful
route observations. This list is not an accounting ledger for billable retries.

## Compatibility and deployment

Historical v3 receipts omit the new optional field during serialization, preserving
their digests. Deterministic Close and legacy execution remain v3. No historical
receipt is upgraded or backfilled. A configured older worker that omits the evidence
fails with reconciliation required; it cannot silently produce an attested receipt.
Deploy Engine and workers together for new admissions. Preserve retained workers
and completed runs; do not recapture historical authority to manufacture evidence.

## Billing policy remains open

Unknown billing means no usable reported or estimated cost was supplied. Numeric
zero plus `cost_measurement: unknown` is not proof of free execution. The current
adapter compares reported cost with the ceiling after execution. That can reject
a result but cannot prevent or reverse already incurred charges.

A hard spending limit needs admission before each chargeable request, a conservative
reservation or explicit policy for unpriced routes, and accounting across retries.
Subscription access can lack per-request billing; unknown alone proves neither a
charge nor a free request. This receipt change does not alter spending policy.

## Verification

Results are recorded in `target/qualification/r06-route-receipts-20261007/`.

The commit was exported from the Git index into an isolated source tree so unrelated
working-tree refactors were not needed for qualification. Tests ran on the host
where local sockets are required, serially to avoid the existing maintenance-lease
fixture race observed in the working-tree parallel run.

| Isolated staged source check | Result |
| --- | --- |
| Engine library | 214 passed, 9 ignored |
| Snapshot worker | 13 passed, 1 ignored |
| Harness run integration | 31 passed, 1 ignored |
| Hermes adapter contract | 31 passed, 14 ignored |
| Route receipt persistence/compatibility | 3 passed |
| Workbench executor library | 25 passed |
| Aule full-cli library, after removing obsolete test | 358 passed |
| Read-only installed M4 receipt inventory | All 12 validate through both updated typed readers; original file SHA-256 values unchanged |

The focused cases cover legacy encoding, reopening v4 from disk, downstream exact
receipt admission, changed route/digest rejection, invalid or downgraded evidence,
model disagreement, forged stderr, bounded evidence, and successful HTTP versus
cancelled/error observations. Two recovery supervisor fixtures now explicitly
supply route evidence; an obsolete recovery receipt fixture now uses canonical v3.
The ignored namespace/live tests were not claimed as executed.

The obsolete Aule test that still called removed execution helpers was deleted.
Those helpers now belong to the Workbench executor; the receipt compatibility
checks above exercise the shared canonical reader. Unrelated pending Aule changes
remain outside this commit.

Aule production compilation passes with `cargo check --offline -p arda-aule --features full-cli --lib`.

The seven source suites total 675 passing tests. Temporary build and isolated source directories are removed after verification; compact logs and the read-only inventory probe are retained.
