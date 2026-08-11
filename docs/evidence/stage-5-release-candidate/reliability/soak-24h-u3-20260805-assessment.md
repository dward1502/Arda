# 24-hour diagnostic soak assessment — U3

**Assessment date:** 2026-08-06  
**Receipt:** `soak-24h-u3-20260805.json`  
**Disposition:** valid failed diagnostic; does not satisfy the Stage 5 reliability gate

## Receipt integrity

- Requested and elapsed duration: 86,400 seconds
- Validity: `valid`
- Source integrity: unchanged
- Frozen source SHA-256: `8f24584d08ba12ddd0f574e44126e309afdab402c0ed549fc1884cceb1f1b54c`
- Protected-state growth: 0 files / 0 bytes
- Storage floor: preserved; minimum observed free space 78,989,627,392 bytes
- Scenario executions: 2,851
- Passed: 2,850 (99.964925%)
- Failed: 1

The receipt is internally valid, but the nonzero failure count makes its top-level `status: fail` authoritative. It cannot be promoted to a passing gate.

## Sole failure

At `2026-08-05T16:13:44.471153Z`, the `model-timeout` scenario ran exactly one registered test:

`cargo test -p arda-engine --test hermes_adapter_contract graph_node_timeout_terminates_and_reaps_hermes -- --exact`

The adapter returned the expected typed timeout, but the test then failed while reading the fake Hermes PID file. The test used an 80 ms graph-node deadline. Under sustained soak load, the deadline could expire while `/usr/bin/python3` was still starting, before the fixture wrote its PID files. That made the test occasionally measure interpreter/scheduler startup latency instead of the intended process-tree termination behavior.

The scenario passed 258 of 259 runs (99.613900%). Every other scenario had zero failures. Latency, source-integrity, protected-state, and storage budgets were preserved.

## Corrective action and focused verification

The timeout/reaping test now gives the fixture 1,000 ms to publish its PID files before the adapter deadline. The fake process still sleeps for ten seconds, so the test continues to exercise the timeout, termination, and descendant-reaping path rather than a clean exit.

Verification on corrected current source:

- Exact test repeated 20 times: 20 passed; every invocation reported `running 1 test`.
- `cargo test -p arda-engine --test hermes_adapter_contract`: 11 passed, 0 failed.
- `cargo fmt --all --check`: pass.
- `git diff --check`: pass.

## Gate consequence

This receipt remains diagnostic because its source snapshot predates later release-audit corrections and because it contains one genuine harness failure. After history sanitation and final source freeze, the corrected complete matrix requires a fresh all-scenario smoke followed by a new uninterrupted 24-hour soak with zero failures.
