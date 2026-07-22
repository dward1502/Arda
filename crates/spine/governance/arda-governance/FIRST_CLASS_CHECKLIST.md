# arda-governance First-Class Expansion Checklist

This is the canonical execution checklist for making `arda-governance` a first-class
Arda spine crate. It consolidates and deduplicates the work described in:

- [`BREAKDOWN.md`](BREAKDOWN.md)
- [`GOVERNANCE_ALIGNMENT_PLAN.md`](GOVERNANCE_ALIGNMENT_PLAN.md)
- [`OPTIMIZATION_PLAN.md`](OPTIMIZATION_PLAN.md)

The source documents remain design and audit evidence. New execution status belongs
here so that stale line references or duplicate status tables do not become competing
plans.

## Completion rules

- Check an item only after its implementation and listed verification evidence exist.
- Preserve compatibility only through an explicit, documented migration window.
- Keep heuristic, independently reviewed, and autonomy-ready results distinguishable
  in serialized output and operator surfaces.
- Do not enable autonomous blocking merely because local heuristics pass.
- Any scoring or policy change must carry a version and regression fixtures.
- Every runtime integration must have both a success-path and degraded/failure-path test.

## Audited baseline

- [x] Love Dynamics and Triad Philosopher arbitration are implemented and exported.
- [x] Resonance can attach Love Dynamics and philosopher metadata without changing its
  legacy numeric score.
- [x] Prometheus autopilot consumes philosopher verdicts in its governance policy.
- [x] Governance chain and philosopher-profile TOML schemas load and reject unsafe
  autonomous-blocking settings.
- [x] The canonical profile path is `config/governance/philosophers.toml`.
- [x] Capability and action-class filtering replaced the old Athena-only game-theory
  candidate filter.
- [x] Love Equation proxy scoring no longer multiplies the raw value by 100 before
  clamping.
- [x] Live Triad and live configurable-chain inputs are available to resonance and
  disclose their source.
- [x] Readiness projection and independent-review receipt types exist, with a CLI
  projection through `arda-aule oracle readiness`.
- [x] Baseline verification passes: `cargo test -p arda-governance` (57 tests on
  2026-07-21: 45 unit, 6 alignment-stack, 6 philosopher-profile).

## Deduplication and source-coverage map

| Source items | Canonical phase |
|---|---|
| Optimization A1; consumer wiring; live telemetry | Phase 1 |
| Optimization A2 and B3 | Audited baseline (already resolved) |
| Optimization A3, D1, and D2; improvement ideas 8–10 | Phase 6 |
| Optimization A4 and C2; improvement idea 4 | Phase 4 |
| Optimization B1 and B2; improvement idea 6; alignment slice 2 | Phase 2 |
| Optimization B4, C3, and E3; improvement idea 5 | Phase 3 |
| Optimization C1; alignment slice 3 | Phase 5 |
| Optimization E1 and E2; improvement idea 3 | Phase 8 |
| Improvement ideas 1–2 | Baseline plus Phase 0 |
| Improvement idea 7; alignment slices 1, 4, and 5 | Phase 7 |

This map is intentionally many-to-one: duplicated recommendations share one execution
phase and one completion status instead of being copied as separate tasks.

## Phase 0 — Establish the first-class public contract

- [x] Define and document the crate boundary: policy evaluation and governance evidence
  belong here; orchestration, storage scheduling, UI rendering, and metrics serving stay
  in their owning crates.
- [x] Replace the crate README's one-line purpose with a public API guide covering Triad,
  configurable chains, resonance, Love Dynamics, JouleWork, readiness, Bacon-Lite,
  philosopher profiles, game-theory selection, and environmental signals.
- [x] Document stability and compatibility expectations for serialized public result
  types and feature flags.
- [x] Replace repository-relative profile/path coupling with injected or explicitly
  resolved base directories; unit and integration tests must work outside the repository
  root.
- [x] Decide the fate of unregistered source files `corpus_loader.rs` and
  `philosophers/socrates.rs`: integrate them as tested public/internal modules or remove
  them; do not leave source-shaped dead paths.
- [x] Correct crate-local indexes and stale canonical paths (`crates/arda-governance` →
  `crates/spine/governance/arda-governance`), and include this checklist plus
  `BREAKDOWN.md` in `INDEX.md`.
- [x] Add rustdoc examples for the primary chain-evaluation, resonance, and readiness
  paths.
- [x] Add a public API/serialization compatibility test suite for all externally
  consumed result types.

Verification:

- [x] `cargo doc -p arda-governance --no-deps` completes without warnings attributable
  to this crate.
- [x] `cargo test -p arda-governance` passes, including public API and serialization
  compatibility fixtures.

Phase 0 evidence (2026-07-21): `cargo fmt -p arda-governance -- --check`,
`cargo test -p arda-governance --all-features` (62 tests plus 3 doctests), and
`cargo doc -p arda-governance --no-deps --all-features` passed with the exact governance
tree overlaid on detached `e4ee615` dependencies. The detached verification avoided an
unrelated in-progress `arda-core` module re-export in the primary worktree.

## Phase 1 — Make live governance evidence the default path

This phase completes Optimization A1 and the consumer-wiring ideas from `BREAKDOWN.md`.
The live-input APIs exist, but compatibility callers still receive a hardcoded 0.7
Triad-purity signal.

- [x] Inventory every non-test `calculate_resonance_basic` and legacy
  `calculate_resonance` caller.
- [x] Migrate callers that already compute a `TriadResult` or `GovernanceChainResult` to
  `calculate_resonance_with_triad` or
  `calculate_resonance_with_governance_chain` without re-validating the task.
- [x] For callers without a result, evaluate the configured chain once and pass the
  receipt into resonance.
- [x] Deprecate the compatibility-default Triad-purity path and define its removal
  release/version.
- [x] Ensure serialized resonance output always identifies whether its Triad component
  is live, absent, or compatibility-defaulted.
- [x] Add regression tests proving failed/conditional/pass lens combinations produce
  distinct Triad-purity and resonance values.
- [x] Wire at least one live governance result into an engine/executor path and one
  operator/telemetry surface; Manwe's adaptive route-policy integration may satisfy the
  first requirement after end-to-end verification.

Verification:

- [x] Repository search shows no production caller silently using compatibility-default
  Triad purity unless it has an explicit migration annotation.
- [x] Integration tests prove one failing and one passing live chain reach both the
  consumer decision and its emitted receipt/telemetry.

Phase 1 evidence (2026-07-21): the only external production compatibility callers were
Varda deep analysis and Plutus runtime telemetry; both already held a `TriadResult` and
now pass it to `calculate_resonance_with_triad`. No production caller without a live
result was found. Manwe's adaptive route decision continues to use a live configurable
chain and now verifies both passing and failing serialized decisions. Varda deep records
emit `resonance_score` plus `triad_purity_source = "live_triad"`; Plutus status telemetry
emits passing and failing live-Triad records. Compatibility APIs are deprecated for
removal in 0.3.0, while the explicit degraded API emits `"absent"`.

Detached `e4ee615` dependency verification with the exact changed crate trees passed:
`cargo test -p arda-governance --all-features` (64 tests plus 3 doctests),
`cargo test -p arda-economics --all-features` (15 tests),
`cargo test -p arda-varda --all-features` (81 tests),
`cargo test -p manwe --features adaptive route_decision_carries_live_governance_chain_metadata`,
and `cargo doc -p arda-governance --no-deps --all-features`.

## Phase 2 — Promote structured, evidence-grade scoring

This phase combines Optimization B1/B2, the `BREAKDOWN.md` evidence-grade penalty, and
alignment-plan work on structured evidence/cooperation/defection extraction.

- [ ] Define a versioned structured governance-evidence schema rather than coupling
  scorers to ad-hoc description substrings.
- [ ] Map existing `Task.result` payloads and relevant caller metadata into explicit
  evidence anchors, action intent, justified urgency, cooperation, defection,
  disconfirming evidence, risk boundary, and fallback path fields.
- [ ] Prefer validated structured fields in Aurelius/Bacon/Sun-Tzu scoring; retain
  keyword heuristics only as a disclosed fallback.
- [ ] Add an evidence-grade model and apply an explicit penalty when required evidence is
  absent instead of silently treating heuristic-local output as adequate proof.
- [ ] Add adversarial fixtures for keyword stuffing, negation, translation/rephrasing,
  contradictory evidence, and malformed structured payloads.
- [ ] Redesign incomplete phi-harmonic components so missing timing, JouleWork, or
  clarification data is represented as missing/zero-weight evidence rather than three
  neutral 50s.
- [ ] Re-normalize resonance weights when phi components are absent and test that real
  signal has greater influence than missing data.
- [ ] Add scorer/property tests for finite values, bounded ranges, determinism, monotonic
  expectations, and malformed/non-finite inputs.

Verification:

- [ ] A task with structured source evidence clears the Bacon evidence threshold without
  requiring evidence keywords in its description.
- [ ] A keyword-stuffed task without evidence does not receive evidence-grade status.
- [ ] Missing phi inputs are disclosed and do not bias the composite toward 50.

## Phase 3 — Normalize and version every policy result

This phase consolidates Optimization B4/C3/E3 and the game-theory confidence-band idea.
`chain_version` already provides a version hook; it must become an enforced policy for
all score semantics.

- [ ] Define a single 0.0–1.0 internal normalization contract for resonance, Love proxy,
  Joule honesty, and Triad pass-rate inputs to game-theory weighting.
- [ ] Keep display conversion to 0–100 at API/UI boundaries only.
- [ ] Add `GameTheoryConfidenceBand` (`High`, `Medium`, `Low`, `NoData`) with documented
  thresholds and serialize it alongside numeric confidence.
- [ ] Replace string-joined veto reasons with typed gate names and structured veto
  reasons while preserving a compatibility renderer.
- [ ] Establish scorer/policy version constants and include them in Triad, chain,
  resonance, Bacon-Lite, and game-theory receipts.
- [ ] Add backward-deserialization fixtures for existing ledger/result records and
  forward-version rejection or downgrade behavior.
- [ ] Update `chain_version` whenever thresholds, evidence semantics, or weighting change.

Verification:

- [ ] Golden fixtures demonstrate stable output for each supported policy version.
- [ ] Consumers match typed veto reasons and confidence bands without parsing strings.

## Phase 4 — Make Bacon-Lite a durable, non-blocking ledger service

This phase deduplicates Optimization A4/C2 and the shared-ledger transport idea in
`BREAKDOWN.md`.

- [ ] Define a shared governance-ledger sink trait at the lowest dependency layer that
  does not introduce a crate cycle; use `arda-core` only if ownership remains coherent.
- [ ] Separate event construction/validation from persistence.
- [ ] Provide a bounded asynchronous writer with batching, flush/shutdown semantics,
  backpressure policy, and explicit dropped-event/error counters.
- [ ] Keep a synchronous adapter only for tests, migrations, and explicitly cold paths.
- [ ] Migrate hot-path `record_bacon_lite` consumers to enqueue events rather than opening
  both files inline.
- [ ] Add rotation/retention and malformed-line behavior for the JSONL ledger.
- [ ] Add a windowed ledger reader that reports per crate/action pass rate, mean
  confidence, lens-outcome distribution, scorer version, and malformed-record count.
- [ ] Expose the summary through an `arda-aule` operator command and machine-readable
  JSON output.
- [ ] Add burst, queue-saturation, restart/recovery, and concurrent-writer tests.

Verification:

- [ ] Hot-path latency tests show persistence work is not performed on the request path.
- [ ] Forced queue saturation and disk errors are visible and do not silently lose
  accountability.
- [ ] Ledger summary fixtures aggregate mixed versions and malformed lines correctly.

## Phase 5 — Add first-class observability and operator explanations

This phase combines Optimization C1 with alignment-plan CLI/operator report work.

- [ ] Choose and document caller-driven versus library-owned metric collection; the
  library must not own a metrics HTTP server.
- [ ] Emit or expose counters for Triad validation verdicts and per-lens outcomes.
- [ ] Emit or expose Bacon-Lite pass/fail counters.
- [ ] Emit or expose histograms for resonance, Love proxy/Dynamics, and Joule honesty.
- [ ] Include policy/scorer version and review mode without creating unbounded metric
  labels.
- [ ] Wire the metrics into an existing scrape surface such as `arda-aule`/Manwe.
- [ ] Render compact philosopher evidence, typed veto reasons, confidence bands,
  readiness gaps, and source maturity in operator reports.
- [ ] Add a governance status/dashboard command that joins current readiness, recent
  ledger summary, and metric snapshot without claiming autonomy.

Verification:

- [ ] A deterministic integration fixture produces expected metric deltas and bounded
  label sets.
- [ ] JSON and human-readable operator output expose the same decision, evidence source,
  policy version, and reason.

## Phase 6 — Resolve and integrate environmental governance signals

This phase deduplicates Optimization A3/D1/D2 and the `BREAKDOWN.md` environmental
signal, bounded parallel-fetch, and telemetry ideas.

- [ ] Make an explicit product decision for Solar: integrate it as advisory context or
  remove it; never let external geomagnetic data become an undisclosed blocking gate.
- [ ] Add a typed `GovernanceSignal` enum/envelope for audio, vision, solar, source,
  timestamp, freshness, confidence, and degraded/unavailable state.
- [ ] Define composite environmental-coherence semantics and distinguish measured data
  from defaults or synthetic placeholders.
- [ ] Remove hardcoded Solar `bz` and `solar_flux` values unless they are clearly typed as
  unavailable/defaulted rather than measurements.
- [ ] Reuse a pooled HTTP client, support endpoint configuration, enforce bounded
  timeouts, cache the last valid sample with TTL, and define neutral degraded behavior.
- [ ] Fetch independent HUD signals concurrently through Arda's bounded async utilities.
- [ ] Wire one advisory signal through a real engine/executor or HADES decision receipt
  and through live telemetry.
- [ ] Add fixture-based tests; do not depend on NOAA availability in the normal test
  suite.

Verification:

- [ ] Quiet/storm/stale/unavailable fixtures produce documented advisory outcomes.
- [ ] External timeout or malformed NOAA data cannot block or falsely approve a governed
  action.

## Phase 7 — Complete philosopher and Love-Dynamics expansion

This phase consolidates all remaining recommendations from
`GOVERNANCE_ALIGNMENT_PLAN.md` and the Love-proxy naming issue from `BREAKDOWN.md`.

- [ ] Rename or expose `love_equation.rs` explicitly as a compatibility proxy and
  deprecate `love_equation_score` in favor of a documented Love-Dynamics compatibility
  wrapper.
- [ ] Decide, with golden tests, whether philosopher verdicts reweight resonance or remain
  separate decision metadata; do not change weights implicitly.
- [ ] Add Nonconformist Bee as a first-class, independently testable module rather than
  an embedded signal field.
- [ ] Add Empirical Distrust as a first-class, independently testable module rather than
  an embedded signal field.
- [ ] Complete philosopher corpus/profile lifecycle boundaries: human-authored source,
  generated artifact, review authority, promotion criteria, and immutable receipt.
- [ ] Integrate or retire the Socrates/corpus-loader prototype identified in Phase 0.
- [ ] Add cross-module arbitration tests for sycophancy, costly truthful work,
  cooperation/defection shifts, and conflicting philosopher recommendations.

Verification:

- [ ] Public and operator surfaces cannot confuse the Love proxy with canonical Love
  Dynamics.
- [ ] Every philosopher-derived action discloses profile source, maturity, authority,
  and review mode.

## Phase 8 — Add realm policy and async scorer extensibility

This phase combines Optimization E1/E2/B1b with the runtime-policy-toggle request from
`BREAKDOWN.md`.

- [ ] Define an async-first governance scorer trait with a deterministic local
  implementation and explicit timeout/error/degraded verdict semantics.
- [ ] Keep optional LLM-backed scoring behind a feature/config gate with task-hash cache,
  provenance, model/provider identity, and reproducibility limits in every receipt.
- [ ] Add per-realm/per-action-class chain policy configuration for required lenses,
  weights, thresholds, strictness, and review requirements.
- [ ] Validate weights and thresholds, reject unknown lenses, and preserve safe defaults.
- [ ] Replace scattered hard rejection of `autonomous_blocking_enabled` with one runtime
  policy authority only after scoped policy, independent-review receipts, rollback, and
  operator controls exist.
- [ ] Keep the global default non-blocking; enable blocking only for explicitly named
  scopes whose readiness report reaches `AutonomyReadyForScope`.
- [ ] Add configuration reload/versioning and audit receipts for policy changes.

Verification:

- [ ] Realm fixtures demonstrate different Bacon/Sun-Tzu emphasis without code changes.
- [ ] Timeout, unavailable scorer, stale cache, and invalid policy all fail to documented
  safe/degraded states.
- [ ] No configuration can enable global autonomous blocking accidentally.

## Phase 9 — First-class release gate

- [ ] All production consumers use the documented result and receipt contracts.
- [ ] No dead source modules, stale path contracts, or undocumented compatibility
  defaults remain.
- [ ] README, rustdoc, indexes, operator docs, and this checklist agree with live code.
- [ ] `GOVERNANCE_PROVENANCE.md` identifies the exact upstream sources, versions,
  adaptation boundaries, and license/notice requirements for every non-original concept.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`
  passes, or workspace-owned pre-existing exceptions are recorded with evidence.
- [ ] `cargo test -p arda-governance --all-features` passes.
- [ ] Focused consumer tests pass for Manwe adaptive routing, `arda-aule` governance
  policy/operator commands, `arda-orome`, `arda-varda`, `arda-mandos`, `arda-economics`,
  and `arda-vaire` integrations.
- [ ] Ledger load/recovery and metrics integration tests pass under burst and failure
  fixtures.
- [ ] Serialized compatibility fixtures and policy-version golden tests pass.
- [ ] A current readiness report remains conservative and lists all missing evidence for
  any scope not yet autonomy-ready.
- [ ] Archive or mark the three source plans as superseded for execution only after every
  open idea is represented here and active documentation links to this checklist.

## Deferred decisions that must not be silently assumed

- [ ] Solar integration versus removal.
- [ ] Philosopher verdict as resonance weight versus separate decision metadata.
- [ ] Caller-driven versus library-owned metric collection.
- [ ] Exact compatibility-removal version for default Triad purity and Love proxy APIs.
- [ ] Whether optional LLM scorers are warranted after structured deterministic evidence
  scoring is measured.
- [ ] Which initial realm/action scopes, if any, may pursue autonomous blocking after
  independent review and rollback evidence exist.
