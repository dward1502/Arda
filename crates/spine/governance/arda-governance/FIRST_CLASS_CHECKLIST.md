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

- [x] Define a versioned structured governance-evidence schema rather than coupling
  scorers to ad-hoc description substrings.
- [x] Map existing `Task.result` payloads and relevant caller metadata into explicit
  evidence anchors, action intent, justified urgency, cooperation, defection,
  disconfirming evidence, risk boundary, and fallback path fields.
- [x] Prefer validated structured fields in Aurelius/Bacon/Sun-Tzu scoring; retain
  keyword heuristics only as a disclosed fallback.
- [x] Add an evidence-grade model and apply an explicit penalty when required evidence is
  absent instead of silently treating heuristic-local output as adequate proof.
- [x] Add adversarial fixtures for keyword stuffing, negation, translation/rephrasing,
  contradictory evidence, and malformed structured payloads.
- [x] Redesign incomplete phi-harmonic components so missing timing, JouleWork, or
  clarification data is represented as missing/zero-weight evidence rather than three
  neutral 50s.
- [x] Re-normalize resonance weights when phi components are absent and test that real
  signal has greater influence than missing data.
- [x] Add scorer/property tests for finite values, bounded ranges, determinism, monotonic
  expectations, and malformed/non-finite inputs.

Verification:

- [x] A task with structured source evidence clears the Bacon evidence threshold without
  requiring evidence keywords in its description.
- [x] A keyword-stuffed task without evidence does not receive evidence-grade status.
- [x] Missing phi inputs are disclosed and do not bias the composite toward 50.

Phase 2 evidence (2026-07-21): `arda.governance.evidence.v1` is parsed from
`Task.result.governance_evidence`; legacy evidence/provenance/source/recommendation keys
map to disclosed partial evidence. Triad and chain results now serialize evidence grade,
scoring source, missing fields, and validation errors. Validated evidence drives all
three default lenses; heuristic and malformed fallbacks are disclosed, and Bacon's
non-validated fallback is capped below its pass threshold.

The adversarial suite covers keyword stuffing, safe negation, non-English/rephrased
descriptions, contradictory intent, malformed structured payloads, legacy mapping,
non-finite task inputs, bounded/deterministic scores, and monotonic evidence enrichment.
Initial RED verification observed all three core behavior tests fail before implementation.
Phi dimensions now report missing inputs and zero available weight; partial dimensions
renormalize over real evidence, and resonance redistributes an absent phi allocation.
Varda and Plutus now populate the structured schema from their existing runtime metadata.
Verification passed in the primary worktree: `cargo test -p arda-governance --all-features`
(75 tests plus 3 doctests), `cargo test -p arda-economics --all-features` (15 tests),
`cargo test -p arda-varda --all-features` (81 tests), the focused Manwe adaptive route
test, and `cargo doc -p arda-governance --no-deps --all-features`. Strict Clippy remains
blocked by pre-existing `arda-core` `derivable_impls` findings outside this phase's scope.

## Phase 3 — Normalize and version every policy result

This phase consolidates Optimization B4/C3/E3 and the game-theory confidence-band idea.
`chain_version` already provides a version hook; it must become an enforced policy for
all score semantics.

- [x] Define a single 0.0–1.0 internal normalization contract for resonance, Love proxy,
  Joule honesty, and Triad pass-rate inputs to game-theory weighting.
- [x] Keep display conversion to 0–100 at API/UI boundaries only.
- [x] Add `GameTheoryConfidenceBand` (`High`, `Medium`, `Low`, `NoData`) with documented
  thresholds and serialize it alongside numeric confidence.
- [x] Replace string-joined veto reasons with typed gate names and structured veto
  reasons while preserving a compatibility renderer.
- [x] Establish scorer/policy version constants and include them in Triad, chain,
  resonance, Bacon-Lite, and game-theory receipts.
- [x] Add backward-deserialization fixtures for existing ledger/result records and
  forward-version rejection or downgrade behavior.
- [x] Update `chain_version` whenever thresholds, evidence semantics, or weighting change.

Verification:

- [x] Golden fixtures demonstrate stable output for each supported policy version.
- [x] Consumers match typed veto reasons and confidence bands without parsing strings.

Phase 3 evidence (2026-07-21): `UnitInterval` now owns finite/clamped 0.0–1.0
normalization, legacy percentage migration, and explicit percentage rendering. Resonance
weighting and game-theory weighting operate on normalized values; existing resonance
component/result surfaces retain their documented 0–100 display contract. Game-theory
task scores now return 0.0–1.0, and legacy percentage-valued `AgentScore` records are
normalized at selection time.

`GameTheoryConfidenceBand` serializes `no_data`, `low`, `medium`, or `high` using the
documented 0.50/0.75 thresholds. Triad and generalized-chain failures carry typed
`GovernanceGateName`, `GovernanceVetoCode`, required-pass, and observed-pass data while
`veto_reason` remains the compatibility renderer. Semantic version constants are carried
by Triad, governance-chain, resonance, Bacon-Lite, game-theory, Love-proxy, and JouleWork
results. Missing version fields deserialize to named v1 semantics; unknown future chain
versions fail closed. The default chain and `config/governance/chains.toml` now use
`structured_evidence_v2`, while `heuristic_local_v1` remains an accepted legacy version.

Verification passed: `cargo fmt -p arda-governance -- --check`;
`cargo clippy -p arda-governance --all-targets --all-features --no-deps -- -D warnings`;
`cargo test -p arda-governance --all-features` (83 tests plus 3 doctests); and
`cargo check -p arda-orome -p arda-vaire -p arda-varda -p arda-mandos -p arda-economics`.
The policy-version suite covers current receipts, typed consumer matching, confidence
bands, legacy downgrade defaults, normalized scoring, and future-version rejection; the
public compatibility fixture locks the serialized result fields. The Manwe and workspace
formatting blockers recorded during Phase 3 were subsequently resolved and are covered by
the passing Phase 9 evidence below.

## Phase 4 — Make Bacon-Lite a durable, non-blocking ledger service

This phase deduplicates Optimization A4/C2 and the shared-ledger transport idea in
`BREAKDOWN.md`.

- [x] Define a shared governance-ledger sink trait at the lowest dependency layer that
  does not introduce a crate cycle; use `arda-core` only if ownership remains coherent.
- [x] Separate event construction/validation from persistence.
- [x] Provide a bounded asynchronous writer with batching, flush/shutdown semantics,
  backpressure policy, and explicit dropped-event/error counters.
- [x] Keep a synchronous adapter only for tests, migrations, and explicitly cold paths.
- [x] Migrate hot-path `record_bacon_lite` consumers to enqueue events rather than opening
  both files inline.
- [x] Add rotation/retention and malformed-line behavior for the JSONL ledger.
- [x] Add a windowed ledger reader that reports per crate/action pass rate, mean
  confidence, lens-outcome distribution, scorer version, and malformed-record count.
- [x] Expose the summary through an `arda-aule` operator command and machine-readable
  JSON output.
- [x] Add burst, queue-saturation, restart/recovery, and concurrent-writer tests.

Verification:

- [x] Hot-path latency tests show persistence work is not performed on the request path.
- [x] Forced queue saturation and disk errors are visible and do not silently lose
  accountability.
- [x] Ledger summary fixtures aggregate mixed versions and malformed lines correctly.

Phase 4 evidence (2026-07-22): `arda-core::ledger::GovernanceLedgerSink<E>` owns only
the generic non-blocking transport contract and saturation/closed errors, while
`arda-governance` owns the Bacon-Lite event schema and durable implementation. Event
construction and validation are explicit and side-effect free. The production enqueue
path uses a lazily initialized bounded writer thread with drop-newest backpressure,
configurable batching and flush cadence, flush/shutdown barriers, advisory file locking,
and accepted/written/dropped/failed/error counters. The retained synchronous adapters are
documented as cold-path compatibility surfaces. Repository search finds no production
`record_bacon_lite(...)` caller; Athena/Varda, Hermes/Orome, and Prometheus/Aule enqueue
instead.

Machine and human ledgers rotate under the same lock with bounded retention. The JSONL
reader includes retained generations, applies inclusive RFC 3339 windows, and either
counts/skips or fails on malformed records. It reports per-crate/action counts, pass rate,
mean confidence, per-lens pass/fail/conditional/unknown distributions, scorer-version
counts, and malformed-record totals. `arda-cli bacon-lite-summary` renders an operator
summary; `--json` emits the complete machine-readable structure, with `--since`,
`--until`, `--path`, and `--strict-malformed` controls.

The Bacon-Lite suite covers 400-event bursts, deterministic queue saturation while the
writer is lock-blocked, enqueue latency under blocked persistence, disk failures and
counters, restart/append recovery, four concurrent producers, rotation retention, mixed
scorer versions, lens outcomes, and malformed records. Verification passed:
`cargo test -p arda-governance --all-features` (58 unit tests, 32 integration tests, and
3 doctests); `cargo check -p arda-varda -p arda-orome`; `cargo check -p arda-aule --bin
arda-cli`; `cargo check -p arda-aule --features full-cli`; strict no-dependency Clippy for
`arda-governance` and the `arda-cli` binary; `git diff --check`; and an executed
`arda-cli bacon-lite-summary --json` smoke test returning the expected empty-ledger JSON
schema.

## Phase 5 — Add first-class observability and operator explanations

This phase combines Optimization C1 with alignment-plan CLI/operator report work.

- [x] Choose and document caller-driven versus library-owned metric collection; the
  library must not own a metrics HTTP server.
- [x] Emit or expose counters for Triad validation verdicts and per-lens outcomes.
- [x] Emit or expose Bacon-Lite pass/fail counters.
- [x] Emit or expose histograms for resonance, Love proxy/Dynamics, and Joule honesty.
- [x] Include policy/scorer version and review mode without creating unbounded metric
  labels.
- [x] Wire the metrics into an existing scrape surface such as `arda-aule`/Manwe.
- [x] Render compact philosopher evidence, typed veto reasons, confidence bands,
  readiness gaps, and source maturity in operator reports.
- [x] Add a governance status/dashboard command that joins current readiness, recent
  ledger summary, and metric snapshot without claiming autonomy.

Verification:

- [x] A deterministic integration fixture produces expected metric deltas and bounded
  label sets.
- [x] JSON and human-readable operator output expose the same decision, evidence source,
  policy version, and reason.

Phase 5 evidence (2026-07-22): collection is library-owned and in-process so scorer entry
points cannot silently omit instrumentation, while transport remains caller-owned.
`arda-governance` owns no socket, HTTP server, exporter task, or Prometheus dependency; it
exposes serializable `GovernanceMetricsSnapshot` values. Triad pass/fail and three per-lens
outcomes, Bacon-Lite pass/fail, and fixed-bucket normalized histograms for resonance, Love
proxy, Love Dynamics projected empathy, and Joule honesty are instrumented at their public
entry points. Bacon-Lite writer accepted/written/dropped/failed/error accountability is
included in the same snapshot when the writer exists.

Metric cardinality is closed over verdict, Aurelius/Bacon/Sun-Tzu, pass/fail/conditional,
the four typed review modes, and `legacy`/`current`/`other` classes for policy and scorer
versions; raw version, model, provider, task, crate, and action strings are not labels.
`arda-aule::render_governance_prometheus` owns scrape-compatible text exposition, and the
active `arda-cli governance-metrics` command emits Prometheus text or `--json` without
moving server ownership into the governance library.

New Bacon-Lite events carry review mode, source maturity, evidence source, typed veto,
confidence band, and compact philosopher evidence with backward-safe defaults. `arda-cli
governance-status` joins the conservative readiness report, inclusive-window recent ledger
summary, latest valid event across retained generations, and metric snapshot. Human and
JSON renderers consume the same typed report and preserve `not_autonomy_ready` rather than
promoting subsystem claims.

TDD RED runs first failed on the missing metrics/operator API, missing latest-event reader,
and missing Aule renderer. GREEN verification passed: `cargo test -p arda-governance
--all-features` (58 unit tests, 34 integration tests, 3 doctests); focused deterministic
metric/cardinality and JSON/human parity tests; the Aule Prometheus renderer unit test;
`cargo check -p arda-varda -p arda-orome -p arda-aule --features arda-aule/full-cli`;
strict no-dependency Clippy for all governance targets and Aule library/binaries; and live
`governance-status` JSON/human plus `governance-metrics --json` CLI smoke checks against the
repository ledger. The live latest record is a backward-compatible v1 event and therefore
honestly reports evidence source `unavailable`; new v2 fixture coverage proves the richer
evidence, veto, confidence, maturity, and philosopher projection.

## Phase 6 — Resolve and integrate environmental governance signals

This phase deduplicates Optimization A3/D1/D2 and the `BREAKDOWN.md` environmental
signal, bounded parallel-fetch, and telemetry ideas.

- [x] Make an explicit product decision for Solar: integrate it as advisory context or
  remove it; never let external geomagnetic data become an undisclosed blocking gate.
- [x] Add a typed `GovernanceSignal` enum/envelope for audio, vision, solar, source,
  timestamp, freshness, confidence, and degraded/unavailable state.
- [x] Define composite environmental-coherence semantics and distinguish measured data
  from defaults or synthetic placeholders.
- [x] Remove hardcoded Solar `bz` and `solar_flux` values unless they are clearly typed as
  unavailable/defaulted rather than measurements.
- [x] Reuse a pooled HTTP client, support endpoint configuration, enforce bounded
  timeouts, cache the last valid sample with TTL, and define neutral degraded behavior.
- [x] Fetch independent HUD signals concurrently through Arda's bounded async utilities.
- [x] Wire one advisory signal through a real engine/executor or HADES decision receipt
  and through live telemetry.
- [x] Add fixture-based tests; do not depend on NOAA availability in the normal test
  suite.

Verification:

- [x] Quiet/storm/stale/unavailable fixtures produce documented advisory outcomes.
- [x] External timeout or malformed NOAA data cannot block or falsely approve a governed
  action.

Phase 6 implementation decision: Solar remains advisory-only environmental context under
`environmental-coherence-v1`; it is never a gate. `GovernanceSignalEnvelope` discloses
freshness, confidence, measurement quality, and degraded/unavailable state. NOAA Kp/Dst
collection now uses a pooled configurable client, explicit timeout, bounded concurrent
requests, and last-valid TTL caching. Unfetched Bz/solar-flux compatibility projections are
zero with typed `unavailable` quality rather than fabricated measurements. Varda batch
executor receipts can carry the resulting advisory evidence, and the same assessment/source
states enter the caller-exposed governance metrics snapshot. Local fixture servers cover
valid, malformed, timeout, and TTL-cache paths; composite fixtures cover quiet, storm, stale,
and unavailable outcomes without contacting NOAA.

## Phase 7 — Complete philosopher and Love-Dynamics expansion

This phase consolidates all remaining recommendations from
`GOVERNANCE_ALIGNMENT_PLAN.md` and the Love-proxy naming issue from `BREAKDOWN.md`.

- [x] Rename or expose `love_equation.rs` explicitly as a compatibility proxy and
  deprecate `love_equation_score` in favor of a documented Love-Dynamics compatibility
  wrapper.
- [x] Decide, with golden tests, whether philosopher verdicts reweight resonance or remain
  separate decision metadata; do not change weights implicitly.
- [x] Add Nonconformist Bee as a first-class, independently testable module rather than
  an embedded signal field.
- [x] Add Empirical Distrust as a first-class, independently testable module rather than
  an embedded signal field.
- [x] Complete philosopher corpus/profile lifecycle boundaries: human-authored source,
  generated artifact, review authority, promotion criteria, and immutable receipt.
- [x] Integrate or retire the Socrates/corpus-loader prototype identified in Phase 0.
- [x] Add cross-module arbitration tests for sycophancy, costly truthful work,
  cooperation/defection shifts, and conflicting philosopher recommendations.

Verification:

- [x] Public and operator surfaces cannot confuse the Love proxy with canonical Love
  Dynamics.
- [x] Every philosopher-derived action discloses profile source, maturity, authority,
  and review mode.

Phase 7 evidence: `tests/phase7_philosopher_expansion.rs`, the additive public API fixture,
and the governance operator projection cover the compatibility label, golden score boundary,
independent modules, lifecycle receipt, and conflicting arbitration cases. The retired
Socrates/corpus-loader paths remain absent and are recorded in `src/INDEX.md`.

## Phase 8 — Add realm policy and async scorer extensibility

This phase combines Optimization E1/E2/B1b with the runtime-policy-toggle request from
`BREAKDOWN.md`.

- [x] Define an async-first governance scorer trait with a deterministic local
  implementation and explicit timeout/error/degraded verdict semantics.
- [x] Keep optional LLM-backed scoring behind a feature/config gate with task-hash cache,
  provenance, model/provider identity, and reproducibility limits in every receipt.
- [x] Add per-realm/per-action-class chain policy configuration for required lenses,
  weights, thresholds, strictness, and review requirements.
- [x] Validate weights and thresholds, reject unknown lenses, and preserve safe defaults.
- [x] Replace scattered hard rejection of `autonomous_blocking_enabled` with one runtime
  policy authority only after scoped policy, independent-review receipts, rollback, and
  operator controls exist.
- [x] Keep the global default non-blocking; enable blocking only for explicitly named
  scopes whose readiness report reaches `AutonomyReadyForScope`.
- [x] Add configuration reload/versioning and audit receipts for policy changes.

Verification:

- [x] Realm fixtures demonstrate different Bacon/Sun-Tzu emphasis without code changes.
- [x] Timeout, unavailable scorer, stale cache, and invalid policy all fail to documented
  safe/degraded states.
- [x] No configuration can enable global autonomous blocking accidentally.

Phase 8 evidence: `src/scorer.rs` provides the object-safe async scorer contract,
deterministic local scorer, timeout/error receipts, and feature/config-gated LLM scorer with
task-hash cache semantics. `src/realm_policy.rs` owns validated exact-scope policy,
weighted async evaluation, atomic reload receipts, and the sole runtime blocking authority.
`config/governance/realm_policies.toml` keeps both repository scopes non-blocking.
`tests/phase8_realm_policy.rs` covers differing Bacon/Sun-Tzu fixture emphasis, invalid
policy, timeout, unavailable backend, stale cache, atomic reload, and scoped readiness/
review/rollback/operator gates. Strict Clippy, all-feature tests, rustdoc, and the `arda-aule`
consumer check pass as recorded in `STATUS.md`.

## Phase 9 — First-class release gate

- [x] All production consumers use the documented result and receipt contracts.
- [x] No dead source modules, stale path contracts, or undocumented compatibility
  defaults remain.
- [x] README, rustdoc, indexes, operator docs, and this checklist agree with live code.
- [x] `GOVERNANCE_PROVENANCE.md` identifies the exact upstream sources, versions,
  adaptation boundaries, and license/notice requirements for every non-original concept.
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`
  passes, or workspace-owned pre-existing exceptions are recorded with evidence.
- [x] `cargo test -p arda-governance --all-features` passes.
- [x] Focused consumer tests pass for Manwe adaptive routing, `arda-aule` governance
  policy/operator commands, `arda-orome`, `arda-varda`, `arda-mandos`, `arda-economics`,
  and `arda-vaire` integrations.
- [x] Ledger load/recovery and metrics integration tests pass under burst and failure
  fixtures.
- [x] Serialized compatibility fixtures and policy-version golden tests pass.
- [x] A current readiness report remains conservative and lists all missing evidence for
  any scope not yet autonomy-ready.
- [x] Archive or mark the three source plans as superseded for execution only after every
  open idea is represented here and active documentation links to this checklist.

Phase 9 evidence (2026-07-25):

- Manwe adaptive routing now owns a validated `RealmPolicyStore`, evaluates every preview
  and selected route through `evaluate_realm_governance` with `LocalGovernanceScorer`, and
  projects the exact scorer receipts and `RuntimeBlockingDecision` into `RouteGovernance`
  and route-selected evidence. Runtime policy reloads return the documented atomic audit
  receipt. Passing and non-passing integration tests prove that receipts reach the production
  decision/event surfaces while the conservative readiness report keeps blocking disabled.
- Migrated Manwe's remaining adaptive hot-path Bacon-Lite calls from the synchronous
  compatibility adapter to `enqueue_bacon_lite`; repository search finds no production
  `record_bacon_lite`, `calculate_resonance_basic`, or `love_equation_score` caller.
- `cargo test -p arda-governance --all-features`: 117 tests passed (67 unit, 47 integration,
  3 doctests). The Bacon-Lite suite includes burst batching, saturation/latency,
  concurrent producers, restart recovery, rotation/retention, malformed records, and disk
  failure counters. Observability, public API compatibility, and policy-version suites pass.
- `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`,
  `cargo fmt -p arda-governance -- --check`, and
  `cargo doc -p arda-governance --no-deps --all-features` pass.
- `cargo test -p manwe --features adaptive`: 292 tests passed, including adaptive routing
  policy/receipt behavior. Default-feature tests for `arda-aule`, `arda-orome`,
  `arda-varda`, `arda-mandos`, `arda-economics`, and `arda-vaire` pass in one focused run.
- `cargo check -p arda-aule --features full-cli --all-targets`,
  `cargo test -p arda-aule --features full-cli`, and strict all-target/all-feature Clippy
  pass. The supported `arda-cli` surface now compiles only implemented operator contracts;
  integration tests execute `governance-metrics` and `governance-status` and validate their
  machine-readable outputs. Uncompiled imported monolith modules are no longer attached to
  the Aule library surface.
- `cargo fmt --all -- --check` passes after applying rustfmt across the workspace.
- The project creator completed human release review on 2026-07-25 and approved the narrow
  public algorithmic adaptations recorded in `GOVERNANCE_PROVENANCE.md`.
- `GOVERNANCE_PROVENANCE.md` records Brian Roemmele's dated JouleWork and Love Equation
  publications, the SSRN corroboration, NOAA terms, exact Arda adaptation boundaries, and
  the project creator's 2026-07-25 human release approval.
- `BREAKDOWN.md`, `GOVERNANCE_ALIGNMENT_PLAN.md`, and `OPTIMIZATION_PLAN.md` are retained as
  design/audit evidence but marked superseded for execution by this checklist.

## Deferred decisions that must not be silently assumed

- [x] Solar integration versus removal: retained as quality-tagged advisory evidence only.
- [x] Philosopher verdict as resonance weight versus separate decision metadata: separate
  metadata; it does not silently change the numeric resonance score.
- [x] Caller-driven versus library-owned metric collection: bounded-label library-owned
  in-process snapshots; callers own serving/export.
- [x] Exact compatibility-removal version for default Triad purity and Love proxy APIs:
  remove before `0.3.0`; current `0.1.0` deprecations remain explicit.
- [x] Optional LLM scorer decision: retained behind `llm-scorer`, with receipted timeout,
  cache, source, and reproducibility boundaries; deterministic local scoring remains the
  default.
- [x] Which initial realm/action scopes, if any, may pursue autonomous blocking after
  independent review and rollback evidence exist: none are autonomy-ready; named scopes
  remain non-blocking until independent receipts and rollback/operator evidence exist.
