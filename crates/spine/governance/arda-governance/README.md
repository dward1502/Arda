---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-28"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-28

# arda-governance

Deterministic governance primitives, configuration contracts, scoring projections,
and evidence records for Arda applications.

## Crate boundary

This crate owns:

- Triad and configurable governance-chain evaluation;
- versioned structured evidence extraction and evidence-grade assessment;
- philosopher profile parsing, validation, lifecycle receipts, and status projection;
- resonance, the explicit Love task-value compatibility proxy, canonical Love Dynamics,
  JouleWork, Nonconformist Bee, Empirical Distrust, and philosopher arbitration;
- governance readiness projections;
- async-first governance scoring and versioned per-realm/per-action policy;
- Bacon-Lite event construction, bounded asynchronous persistence, and ledger reads;
- library-owned in-process governance metric collection and read-only operator projections;
- game-theory-labelled local agent-selection heuristics;
- audio, vision, and solar governance signal types.

This crate does not own provider dispatch, daemon/process lifecycle, a metrics HTTP
server, policy enforcement outside returned verdicts, or claims of autonomous consensus.
Metrics transport and Prometheus exposition remain caller-owned; `arda-aule` renders the
scrape-compatible text surface.

## Public API map

| Surface | Primary input | Primary output | Failure behavior |
| --- | --- | --- | --- |
| `triad_validate` | `&arda_core::Task`, optional `TriadConfig` | `TriadResult` | deterministic; no I/O |
| `evaluate_governance_chain` | task and validated `GovernanceChainConfig` | `GovernanceChainResult` | deterministic; no I/O |
| `assess_governance_evidence` | `Task.result` and caller metadata | `GovernanceEvidenceContext` | malformed/unsupported evidence is disclosed and downgraded |
| `load_governance_chain[_from_str]` | explicit path or TOML | `GovernanceChainConfig` | `GovernanceChainError` preserves read/parse/validation class |
| `load_philosopher_profiles[_from_str]` | explicit path or TOML | `PhilosopherProfileSet` | `PhilosopherProfileError` preserves read/parse/validation class |
| `load_realm_policy[_from_str]` | explicit path or TOML | validated `RealmPolicyConfig` | rejects unknown lenses, invalid weights/thresholds, wildcard scopes, and any blocking global default |
| `evaluate_realm_governance` | task, resolved realm/action policy, async scorer, timeout | `RealmGovernanceVerdict` plus scorer receipts | timeout, unavailable/error scorer, and stale cache produce zero-score degraded non-passing receipts |
| `RuntimeBlockingAuthority::evaluate` | realm policy, named scope, readiness report, operator control | `RuntimeBlockingDecision` | remains non-blocking unless scoped readiness, rollback, independent review receipts, and operator controls all pass |
| `calculate_resonance*` | task plus optional live governance/environment signals | `ResonanceScore` | deterministic; missing optional signals are represented in metadata |
| `evaluate_love_dynamics` | `LoveDynamicsInput` | `LoveDynamicsScore` | canonical cooperation/defection dynamics; non-finite/unit inputs are normalized conservatively |
| `love_dynamics_compatibility_proxy` | `&Task` | `LoveEquationScore` | legacy task-value proxy; explicitly not canonical Love Dynamics |
| `assess_nonconformist_bee` | `&Task` | `NonconformistBeeAssessment` | advisory independence/sycophancy assessment |
| `assess_empirical_distrust` | `&Task` | `EmpiricalDistrustAssessment` | advisory evidence-grade/falsifiability assessment |
| `profile_joulework` | `&Task` | `JouleWorkProfile` | reports measurement source; does not upgrade estimated data to observed truth |
| `interpret_alignment` | `AlignmentSignals` | `TriadPhilosopherVerdict` | deterministic advisory result with an immutable-by-value lifecycle receipt |
| `default_governance_readiness_report` | none | `GovernanceReadinessReport` | conservative projection; defaults are not autonomy-ready |
| `enqueue_bacon_lite` | task and context | `BaconLiteEvent` | non-blocking; reports saturated/closed transport errors and increments accountability counters |
| `record_bacon_lite_to` | task, context, explicit `BaconLiteLogPaths` | `BaconLiteEvent` | synchronous cold-path compatibility adapter for tests and migrations |
| `global_governance_metrics().snapshot()` | instrumented scorer results | `GovernanceMetricsSnapshot` | in-process bounded-label snapshot; owns no server or background exporter |
| `build_governance_status_report` | readiness, ledger summary, metrics, latest event | `GovernanceStatusReport` | read-only projection that preserves conservative autonomy truth |
| `GameTheory::select_agent_with_policy` | task/action class | `GameTheorySelectionResult` | explicit fallback policy and reason when no candidate qualifies |
| `collect_environmental_signals` | independent audio/vision futures and pooled `SolarClient` | `EnvironmentalCoherence` | bounded concurrent collection; unavailable sources become neutral advisory evidence |

The crate-root re-exports in `src/lib.rs` are the supported consumer surface. Public
modules remain available for specialised types, but consumers should prefer root
re-exports where provided.

## Environmental advisory semantics

Audio, vision, and NOAA geomagnetic context use a typed `GovernanceSignalEnvelope` carrying
source timestamp, collection timestamp, freshness, confidence, measurement quality, and
healthy/degraded/unavailable state. `EnvironmentalCoherence` is always marked
`advisory_only = true`: it cannot approve, reject, or block an action. A score at or below 50 is
`caution`; a score of at least 75 is `supportive` only when at least two fresh, healthy
sources are available; all other states are `neutral`. Defaults and unavailable values are
excluded from the weighted score. Stale values are down-weighted and can never produce a
supportive advisory.

`SolarClient` pools HTTP connections, accepts configurable Kp/Dst endpoints and request
timeouts, executes both NOAA requests through Arda's bounded async gate, and caches the last
valid sample for a configurable TTL. A refresh failure may return the last sample as stale,
degraded evidence; no sample returns unavailable evidence. The v1 collector does not fetch
Bz or solar flux. Their compatibility numeric projections are zero and their adjacent
`MeasurementQuality` fields are explicitly `unavailable`, so they must not be interpreted
as measurements.

`AthenaStore::ingest_batch_with_environment` carries this evidence into Varda executor
receipts without changing acceptance or deduplication. Environmental assessments and each
source health/freshness/quality state are also recorded in the in-process governance metric
snapshot for caller-owned live exposition.

## Structured evidence and scoring

`arda.governance.evidence.v1` is the canonical structured evidence schema. Place it at
`Task.result.governance_evidence` with evidence anchors, action intent, optional justified
urgency, cooperation and defection values, disconfirming evidence, a risk boundary, and a
fallback path. Legacy result keys such as `evidence`, `provenance.path`, `source_id`, and
`recommendation` are mapped into a disclosed partial evidence record.

Triad results serialize an evidence assessment with one of `no_evidence`,
`heuristic_only`, `structured_partial`, or `structured_validated`. Structured validated
fields drive the Aurelius/Bacon/Sun-Tzu scorers. Keyword heuristics remain a disclosed
fallback and Bacon cannot receive a passing evidence score from keywords alone.

`GovernanceScorer` is the async-first extension boundary. `LocalGovernanceScorer` preserves
deterministic structured-evidence scoring. `score_governance_with_timeout` converts timeout,
backend errors, unavailable scorers, and invalid scores into explicit zero-score degraded
receipts rather than implicit approval. Every receipt names its task hash, scorer, provider,
model, provenance, cache state, and reproducibility limits.

`RealmPolicyConfig` resolves exact `(realm, action_class)` scopes over a safe non-blocking
global default. Rules configure required lenses, weights, thresholds, strictness, minimum
weighted score, and review requirements. `RealmPolicyStore` validates and atomically swaps
configuration while returning an applied/rejected versioned audit receipt. Legacy
`autonomous_blocking_enabled` fields in chain/profile files are non-authoritative;
`RuntimeBlockingAuthority` is the only execution-facing decision point.

Manwe's adaptive route preview and selection paths are the production consumer of this
contract. They resolve request realm/action metadata, score through the local async scorer,
serialize the returned scorer receipts and blocking decision in `RouteGovernance`, and copy
the same evidence into selected-route records. The current integration supplies the
conservative default readiness report, so operator configuration alone cannot make a scope
blocking; a future autonomy-ready readiness source must still satisfy the authority gates.

Missing phi-harmonic inputs are listed in `phi_missing_inputs` and carry zero available
weight. Available phi dimensions are normalized over their actual weight, and resonance
redistributes the omitted phi allocation rather than injecting neutral 50-point values.

## Filesystem and configuration

Library code does not infer the repository root from `CARGO_MANIFEST_DIR`.
Construct `GovernancePaths::new(base_dir)` and pass the resulting path to loaders,
or pass any explicit path directly. Production hot paths use `enqueue_bacon_lite` and
the bounded writer; `record_bacon_lite[_to]` is retained only for tests, migrations, and
explicitly cold paths. Default paths resolve from `ARDA_ROOT`, then the process working
directory, with `ARDA_BACON_LITE_LOG_PATH` and `ARDA_BACON_LITE_HUMAN_PATH` as
individual overrides.
The repository realm policy is `config/governance/realm_policies.toml` and resolves through
`GovernancePaths::realm_policy()`.

```rust
use arda_governance::{load_governance_chain, GovernancePaths};

let paths = GovernancePaths::new("/srv/arda");
let config = load_governance_chain(paths.chain_config())?;
# Ok::<(), arda_governance::GovernanceChainError>(())
```

## Observability and operator ownership

Collection is **library-owned and in-process**: Triad, Bacon-Lite, resonance, Love proxy,
Love Dynamics, JouleWork, and environmental advisory entry points update one
`GovernanceMetrics` collector.
Transport is **caller-owned**: this crate only returns serializable counter/histogram
snapshots and never binds a socket or starts a metrics server. `arda-aule` provides
`governance-metrics` as Prometheus text or JSON and `governance-status` as human or JSON
operator output.

Metric labels are intentionally closed: verdict, the three known lenses, three outcomes,
four review modes, three environmental sources, bounded health/freshness/measurement-quality
states, and `legacy`/`current`/`other` policy/scorer version classes. Raw policy,
model, provider, task, crate, and action strings are never labels. Score histograms use
fixed normalized buckets. The operator report joins readiness gaps, recent Bacon-Lite
evidence, typed vetoes, confidence band, philosopher evidence, source maturity, and metric
snapshot; it retains `default_autonomy_ready = false` unless the readiness evidence itself
proves a scoped state.

## Compatibility contract

- Removing or renaming a crate-root re-export, public field, or serialized field is
  a breaking change.
- Existing enum wire names and schema-version strings are stable within the current
  major version.
- New serialized fields must be additive and deserializable from older records;
  use Serde defaults where omission is valid.
- New enum variants require consumer impact review because exhaustive matches may
  break even when serialization remains compatible.
- Error variants may be added, but existing failure classes and their source errors
  should not be collapsed into unstructured strings.
- `tests/fixtures/public_api_v1.json` and `tests/public_api_compat.rs` guard the
  externally consumed result shapes and wire encodings.

`love_equation_score` is a deprecated name retained for source compatibility; new callers
must use `love_dynamics_compatibility_proxy`, whose result identifies itself as
`task_value_proxy` and `not_canonical_love_dynamics`. Canonical cooperation/defection behavior
exists only behind `evaluate_love_dynamics`.

Triad Philosopher output is intentionally `separate_decision_metadata` in resonance. It is
serialized and operator-visible but does not silently reweight the numerical score. Every
derived verdict carries profile source, source revision, maturity, authority, review mode,
review authority, generated-artifact identity when applicable, and promotion criteria.

The synthetic `calculate_resonance` and `calculate_resonance_basic` paths are deprecated
and scheduled for removal in `arda-governance` 0.3.0. New production code must evaluate
the Triad or configured chain once and pass that result to resonance. A degraded caller
that genuinely has no governance result must call `calculate_resonance_without_governance`,
which serializes `triad_purity_source = "absent"` instead of inventing a score.

The optional `llm-scorer` feature exposes only the provider-neutral LLM scorer/backend
contract; no provider is selected by default. Runtime configuration must also set
`enabled = true`. LLM receipts use a task-hash/lens/provider/model cache key and reject stale
entries into an explicit degraded state. `default = []` remains intentional, so enabling all
features does not alter the deterministic default path or existing wire formats.

## Documentation map

- `STATUS.md` — current stability, verification evidence, and known boundaries.
- `BREAKDOWN.md` — implementation and module map for maintainers and agents.
- `PLAN.md` — future-work discussion queue; it contains no current commitments.
- `OWNERSHIP.md` — authority and integration boundaries.
- `GOVERNANCE_PROVENANCE.md` — algorithm/source provenance and release review.
- `INDEX.md` — deterministic crate-document index.
- `src/README.md` and `tests/README.md` — local source and test navigation.

## Verification

From the workspace root:

```text
cargo fmt -p arda-governance -- --check
cargo check -p arda-governance --no-default-features
cargo test -p arda-governance --no-default-features -- --test-threads=1
cargo check -p arda-governance --all-targets --all-features
cargo test -p arda-governance --all-features
cargo clippy -p arda-governance --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p arda-governance --no-deps --all-features
```
