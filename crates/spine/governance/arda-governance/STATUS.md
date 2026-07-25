# arda-governance status

Crate: `crates/spine/governance/arda-governance`
Current state: active
Branch: `manwe`
Test evidence: 2026-07-25 Phase 9 verification listed below.
Documentation set: `README.md`, `BREAKDOWN.md`, `FIRST_CLASS_CHECKLIST.md`,
`GOVERNANCE_PROVENANCE.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md`, and indexes.
Governance and memory integration scenario: `arda-governance`/`arda-vaire` test exists and asserts expected.

Current signature: Triad/chain evaluation and typed evidence assessment active; async-first
local/optional-LLM scoring receipts; validated realm/action policy with atomic reload audit;
one readiness/review/rollback/operator-gated runtime blocking authority; explicit Love-
Dynamics compatibility proxy, independent Nonconformist Bee and Empirical Distrust modules,
separate philosopher resonance metadata, and lifecycle-receipted operator evidence.
The backwards-compatible contract remains enforced via `tests/fixtures/public_api_v1.json`.
Environmental degraded/unavailable semantics and producer-rate/backpressure, burst,
recovery, malformed-ledger, rotation, and disk-failure paths have executable fixtures.
Deprecated `calculate_resonance*` paths will be removed before 0.3.0; migration required in consumers.

Operational expectation: use `enqueue_bacon_lite` on production paths, consume typed
results/receipts, and verify with all-feature governance tests plus consumer-specific tests.

## Phase 7 verification

- `cargo fmt -p arda-governance -- --check`: passed.
- `cargo test -p arda-governance --test phase7_philosopher_expansion`: 6 passed.
- `cargo test -p arda-governance --all-features`: 110 passed including doctests.
- `cargo clippy -p arda-core --all-targets --all-features -- -D warnings`: passed.
- `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`: passed,
  including dependency linting through `arda-core`.
- `cargo check -p arda-aule`: passed, including the operator consumer of receipted verdicts.
- Focused `arda-aule --features full-cli` test execution is blocked before test discovery by
  the consumer's pre-existing feature-build failures (unresolved legacy imports/dependencies);
  the default-feature consumer check remains clean.
- Workspace-wide formatting is blocked by unrelated pre-existing diffs outside this crate.

## Phase 8 verification

- `cargo fmt -p arda-governance -- --check`: passed.
- `cargo test -p arda-governance --all-features`: passed, including 7 Phase 8 tests and
  rustdoc tests.
- `cargo clippy -p arda-core --all-targets --all-features -- -D warnings`: passed.
- `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`: passed.
- `cargo check -p arda-aule`: passed against the additive crate-root consumer surface.
- `cargo doc -p arda-governance --no-deps --all-features`: passed.
- Repository realm policy validates via the injected-root path-independence integration test;
  its global and named-scope blocking flags are all false.

## Phase 9 verification and readiness

Passed on 2026-07-25:

- `cargo fmt -p arda-governance -- --check`.
- `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`.
- `cargo test -p arda-governance --all-features`: 117 passed (67 unit, 47 integration,
  3 doctests).
- `cargo doc -p arda-governance --no-deps --all-features`.
- `cargo test -p manwe --features adaptive`: 292 passed.
- One default-feature focused run for `arda-aule`, `arda-orome`, `arda-varda`,
  `arda-mandos`, `arda-economics`, and `arda-vaire` passed.

Release evidence:

- Manwe adaptive routing now exercises `RealmPolicyStore`, `evaluate_realm_governance`,
  `LocalGovernanceScorer`, and `RuntimeBlockingAuthority` on production preview/selection
  paths. Typed scorer and blocking receipts are serialized in route decisions and selected-
  route evidence; passing and non-passing tests keep the conservative non-blocking boundary.
- `cargo fmt --all -- --check` passes.
- `cargo check -p arda-aule --features full-cli --all-targets`, full-feature tests, and
  strict all-target/all-feature Clippy pass. Process-level tests validate the supported
  `governance-metrics` and `governance-status` operator contracts.
- `GOVERNANCE_PROVENANCE.md` records exact external sources and adaptation boundaries. The
  project creator completed human release review and approved the public algorithmic adapters
  on 2026-07-25; no upstream prose, media, or code is incorporated.

The current default readiness report remains deliberately **not autonomy-ready**. Missing
independent review receipts, scoped runtime evidence, rollback proof, and explicit operator
control continue to prevent any realm/action scope from enabling autonomous blocking.

See `CRATE_PLAN.md` and `OWNERSHIP.md` for implementation priorities.
