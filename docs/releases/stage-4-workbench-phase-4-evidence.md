# Stage 4 Workbench Phase 4 evidence

Date: 2026-07-31

Scope: deterministic repository golden paths, crash/restart exact-once proof, and the requested verification ladder. This packet does not claim live paid-provider validation, packaging, or external evaluator sign-off.

## Golden proofs

| Proof | Test | Result |
|---|---|---|
| Rust repository | `cargo test -p arda-engine --test workbench_rust_golden -- --test-threads=1` | PASS; 237 ms install-to-result, one transient failure, one automatic retry, one bounded diff |
| Python adapter | `cargo test -p arda-engine --test workbench_python_golden -- --test-threads=1` | PASS; 162 ms install-to-result, one denied malformed request, one corrected retry, one bounded diff |
| Boundary recovery | `cargo test -p arda-engine --test workbench_boundary_recovery -- --test-threads=1` | PASS; nine forced process terminations, two execute attempts, one observable mutation |

Machine-readable records:

- `docs/evidence/workbench-stage-4/rust-golden-result.json`
- `docs/evidence/workbench-stage-4/python-golden-result.json`
- `docs/evidence/workbench-stage-4/boundary-recovery-result.json`

Reproduction instructions: `docs/operator/workbench-stage-4-golden.md`.

## Correlation evidence

The Rust and Python result packets each carry one stable `run_id` with:

- objective and graph node IDs;
- approval receipt;
- model/adapter route and normalized cost;
- tool and test evidence;
- project contract digest and request/receipt digests;
- bounded Git diff;
- intervention/failure/recovery record;
- project-memory summary;
- closed graph projection.

The deterministic Hermes route is `local-fixture/scripted-hermes-golden` and records normalized fixture cost `0.001 USD`. The Python reference adapter accurately records `no-model` and zero cost. No live provider was contacted.

## Verification ladder

All commands below passed on 2026-07-31:

1. `cargo test -p arda-core --test project_contract --test run_graph -- --test-threads=1`
2. `cargo test -p arda-engine --test run_recovery --test harness_projects --test harness_runs -- --test-threads=1`
3. `uv run --with jsonschema python -m unittest tests/test_workbench_contract_fixtures.py -v`
4. `python3 -m pytest sdk/python/tests/test_conformance.py -q`
5. `(cd apps/arda-hud && pnpm test && pnpm lint && pnpm build)`
6. `cargo check --workspace --all-targets --all-features`
7. `cargo test --workspace --all-features -- --test-threads=1`

The host Python initially lacked `pytest`; the rerun used an isolated `/tmp/arda-stage4-venv` populated with `uv pip install pytest`. This was an environment prerequisite correction, not a product-code failure.

The HUD had no `lint` script at the first ladder attempt. Commit `1834160` added an Oxlint command and pinned dependency; the exact combined HUD ladder then passed with 101 existing warnings and zero lint errors. React test-suite `act(...)` warnings and existing Rust unused-code/import warnings remain non-blocking.

## Security and review disposition

An independent read-only security reviewer returned **PASS** for commits `360b3cb`, `52139ac`, and `f5b14fa` plus the evidence exporter and operator documentation, with no critical or high-severity findings. A separate correctness reviewer inspected the fixtures, run correlation, out-of-workspace Python launch, and restart test; its provider returned no prose verdict, so this packet does not count it as an additional sign-off.

The focused security review found:

- temporary repositories are isolated and begin from clean Git commits;
- the reference adapter runs in a canonical project cwd with a cleared allowlisted environment;
- fixture operations accept only an exact operation and exact old/new values;
- no network or external credentials are used;
- evidence exports are redacted normalized receipts, diffs, digests, and metrics;
- recovery replays an idempotent bounded mutation and proves one observable effect.

Remaining limitations are medium or lower release-gate items: live-provider prompt-injection evaluation, packaging, signed artifact provenance, and external non-author evaluation are not covered by this deterministic Phase 4 packet.
