# arda-mandos (Oracle) Improvement Checklist

Owner: HADES | Runtime role: Oracle | Status: active
Source: live audit of `src/`, tests, crate docs, and workspace consumers
Last reviewed: 2026-07-22

Items use `[ ]` pending, `[~]` in progress, `[x]` complete, and `[!]` blocked.
Complete an item only after its acceptance evidence is recorded in the final section.

## Mission and boundaries

`arda-mandos` should be Arda's explainable, evidence-aware decision-support oracle. It may
score, qualify, reject, or escalate a proposal, but it must not manufacture certainty or
silently turn heuristic text matching into autonomous authority.

The target architecture has five explicit stages:

1. Normalize and validate a typed query.
2. Resolve cited evidence into a bounded reasoning context.
3. Run independently inspectable governance gates.
4. Apply a versioned outcome policy, including veto and escalation rules.
5. Persist an auditable verdict before exposing it to consumers.

Out of scope for the first implementation cycle:

- LLM-generated chain-of-thought storage. Persist concise decision reasons and evidence
  references, not hidden reasoning traces.
- Network-wide consensus or autonomous policy changes.
- Splitting transport into another crate before the service contract stabilizes.
- Treating Oracle output as clinical, legal, financial, or safety certification.

## Verified baseline

- [x] Read all crate source modules, crate-local tests, README, and BREAKDOWN.
- [x] Inspect direct consumers in `arda-aule` and `arda-orome`.
- [x] `cargo test -p arda-mandos --all-features` passes: 15 tests, 0 failures.
- [x] `cargo test -p arda-mandos --no-default-features` passes: 15 tests, 0 failures.
- [x] Gate `join_error` with the HTTP feature; no-default-features is warning-free.
- [x] Crate-local strict Clippy passes with `--no-deps`; the full dependency-closure command
  remains separately blocked by pre-existing `arda-core` warnings.

## P0 — Correctness and decision safety

### P0.1 Establish invariant tests before changing policy

- [x] Add table-driven tests covering every gate threshold and every outcome.
- [x] Add boundary tests for scores at, immediately below, and immediately above each
  threshold.
- [x] Add regression tests showing that additional relevant evidence never lowers Bacon's
  score solely because more context entries were supplied.
- [x] Add regression tests for explicit contradiction and high-risk veto behavior.
- [x] Add property tests or equivalent loops asserting all exposed scores are finite and in
  `[0.0, 1.0]`.

Acceptance:

- Tests fail against the known evidence-weight and majority-pass defects before the fixes.
- The test names state the policy invariant rather than the implementation detail.

### P0.2 Replace accidental gate arithmetic with a versioned policy

Current risks:

- `evaluate_bacon()` subtracts `context.len() * 0.15`, so supplying more evidence lowers
  confidence.
- `determine_outcome()` allows any two passing gates to override a failed gate; the current
  contradiction test therefore expects an overall `Pass` while Aurelius fails.
- `score_gate()` duplicates separate keyword heuristics and is not part of `OracleEngine`.
- Matching is inconsistent and partly case-sensitive.

Checklist:

- [x] Introduce `OraclePolicy` with a stable `policy_id`, semantic version, gate thresholds,
  veto rules, conditional band, and evidence caps.
- [x] Make the default policy explicit; do not bury thresholds in evaluator methods.
- [x] Correct Bacon evidence scoring so additional supplied evidence is positive or neutral
  and the contribution is bounded.
- [ ] Replace evidence-count scoring with evidence quality/provenance scoring once P0.5's
  `EvidenceRef` contract exists.
- [x] Define mandatory vetoes for logical contradiction and explicitly classified dangerous
  operations; document which gates may veto.
- [x] Define deterministic `Pass`, `Conditional`, and `Fail` semantics.
- [ ] Decide whether escalation is a separate outcome or a consumer action on `Fail`.
- [x] Populate `Verdict.conditions` for conditional outcomes or replace the optional string
  with typed conditions.
- [ ] Normalize text once and make all lexical matching case-insensitive.
- [ ] Unify `TruthScorer`/`score_gate()` with the engine policy, or deprecate the duplicate
  API after consumer search confirms it is unused.
- [x] Include `policy_id` and `policy_version` in every verdict and status snapshot.

Acceptance:

- A contradiction cannot produce `Pass` under the default policy.
- Adding relevant evidence cannot reduce Bacon's score.
- Every non-pass outcome has a machine-readable reason, and every conditional outcome has
  at least one actionable condition.
- Existing `arda-aule` outcome mapping compiles and has updated policy tests.

### P0.3 Define and validate the query contract

- [ ] Add `query_type: QueryType` to `OracleQuery`, or remove the currently unused enum.
- [ ] Reject empty/whitespace-only IDs, tasks, and requesters.
- [ ] Bound task length, evidence item count, and individual evidence size at every transport.
- [ ] Preserve caller timestamps when supplied and record a separate `evaluated_at` timestamp.
- [ ] Define duplicate-query/idempotency behavior; never silently append conflicting verdicts
  under the same query ID.
- [ ] Add optional correlation/causation metadata for audit linkage without embedding it in
  free-form task text.
- [ ] Add `#[serde(rename_all = "snake_case")]` or an explicitly documented wire format for
  public enums before the contract spreads further.

Acceptance:

- IPC, HTTP, direct service calls, CLI fallback, and Discord use the same validation path.
- Invalid requests fail before gate evaluation or persistence.
- Contract serialization round-trips are covered by golden fixtures.

## P0 — Evidence and PageIndex integrity

### P0.4 Repair PageIndex correctness

Current risks:

- `PageTree::navigate()` searches a new empty `PageIndex`, so it always returns no matches.
- `index_document()` uses `or_insert`, preventing a document from being refreshed.
- `OracleEngine::evaluate_bacon()` searches only the first HashMap document, which is
  nondeterministic and incomplete.
- `build_path()` collects unrelated earlier headings instead of the actual ancestor stack.
- Random UUID node IDs make re-indexed evidence references unstable.

Checklist:

- [ ] Make `PageTree::navigate()` search its own index and nodes.
- [ ] Replace-or-version an existing document intentionally; return an indexing report.
- [ ] Search all eligible documents, or require explicit document IDs on the query.
- [ ] Implement stack-based TOC ancestry and test sibling/nested heading paths.
- [ ] Derive stable node IDs from document ID + canonical heading path + location.
- [ ] Normalize punctuation and Unicode; remove empty terms and deduplicate node hits.
- [ ] Define relevance normalization in `[0.0, 1.0]` and deterministic tie-breaking.
- [ ] Remove unused `PageNode`/`content_preview`, or use a single canonical node model.
- [ ] Either wire the `pageindex` Cargo feature to module/dependencies or remove the inert
  feature.

Acceptance:

- Navigation, re-indexing, multi-document search, ancestry, stable IDs, and deterministic
  ordering each have focused tests.
- Verdict evidence contains stable source references, not only text such as "retrieved N".

### P0.5 Make evidence provenance first-class

- [ ] Replace `Vec<String>` evidence with a typed `EvidenceRef` model containing source ID,
  kind, locator, observed timestamp, digest, and optional excerpt/claim.
- [ ] Distinguish supplied, retrieved, inferred, and unavailable evidence.
- [ ] Record which evidence affected each gate and which evidence was rejected.
- [ ] Add freshness, independence, and source-quality signals without pretending they prove
  truth.
- [ ] Make missing or conflicting evidence visible as uncertainty, not merely a lower opaque
  number.
- [ ] Ensure exports redact sensitive excerpts while retaining hashes and provenance.

Acceptance:

- A verdict can be audited back to immutable source references.
- Missing, stale, conflicting, and corroborating evidence have separate tests.

## P1 — Explainability and reasoning context

### P1.1 Implement `ReasoningContext`

- [ ] Replace the empty placeholder with a bounded tree/graph of claims, evidence, objections,
  assumptions, and dependencies.
- [ ] Assign stable IDs and parent/edge types.
- [ ] Track depth, node count, and byte limits to prevent unbounded traversal.
- [ ] Provide deterministic traversal and summary APIs.
- [ ] Detect cycles and dangling evidence references.
- [ ] Keep concise public rationales separate from private model traces.

Acceptance:

- Unit tests cover branching, objections, cycles, limits, and deterministic traversal.
- `OracleEngine` consumes this model rather than parallel free-form vectors.

### P1.2 Improve gate outputs

- [ ] Replace string gate names with a typed `GateKind`.
- [ ] Separate concerns, supporting evidence, counter-evidence, and recommended remediation.
- [ ] Add score components so the final number is reproducible from serialized inputs.
- [ ] Report confidence/uncertainty independently from approve/reject outcome.
- [ ] Add an explicit `Escalate` disposition if escalation is semantically different from a
  failed proposal.
- [ ] Ensure `resonance_score` is derived from stated factors rather than mostly fixed values.

Acceptance:

- Re-evaluating the same normalized query, evidence set, and policy yields the same verdict
  payload except evaluation metadata.
- Operators can tell what evidence or policy change would alter a conditional verdict.

### P1.3 Clarify governance integration

- [ ] Decide whether Mandos's local Aurelius/Bacon/Sun Tzu gates or
  `arda-governance::triad_validate()` is authoritative; avoid two unexplained triads in one
  verdict.
- [ ] Replace the synthetic completed `Task` in `build_governance_task()` with an explicit
  governance input contract.
- [ ] Remove or wire the unused `Ledger` field and `with_ledger()` path.
- [ ] Define the Love Equation as advisory context and document whether it can ever veto.
- [ ] Move reusable policy interfaces to the governance layer only after the Mandos contract
  is proven by tests and consumers.

Acceptance:

- There is one documented authority for each decision dimension.
- No fabricated task lifecycle timestamps or joule measurements are needed to obtain a
  governance verdict.

## P1 — Persistence, audit, and recovery

### P1.4 Make persisted state authoritative and restart-safe

Current risks:

- Restarted services expose persisted verdicts in `evidence_plane`, while engine
  `history_total` resets to zero.
- Malformed JSONL lines are silently ignored.
- `recent_verdicts()` reads the entire unbounded ledger for every request.
- Snapshot writes are not atomic and no migration guard validates schema versions.

Checklist:

- [ ] Introduce a versioned `VerdictRecord` envelope with sequence number and record digest.
- [ ] Hydrate engine/status counters from valid persisted records at startup, or make the
  persisted store the single status authority.
- [ ] Detect duplicate IDs, malformed lines, truncation, sequence gaps, and unsupported schema
  versions; expose degraded status instead of silently dropping records.
- [ ] Write snapshots atomically via temp file + flush/sync + rename.
- [ ] Define append durability and locking for concurrent evaluations.
- [ ] Add bounded/indexed recent-history reads and a retention/rotation policy.
- [ ] Add export and verify commands before any migration support mutates records.
- [ ] Add migration fixtures for current and future schemas.

Acceptance:

- Status totals are identical before and after restart.
- Injected corruption is reported without losing earlier valid records.
- Interrupted snapshot writes preserve the last valid snapshot.

### P1.5 Make Plutus side effects observable

- [ ] Replace untracked fire-and-forget telemetry with a bounded delivery policy.
- [ ] Record signal state (`pending`, `delivered`, `failed`) or explicitly classify Plutus
  emission as best-effort telemetry.
- [ ] Expose delivery failures/counters in status without failing a valid Oracle verdict unless
  policy requires it.
- [ ] Remove process-global environment mutation from parallel tests; inject Plutus paths or
  clients.
- [ ] Add shutdown draining or cancellation behavior for background work.

Acceptance:

- Tests do not race through `ARDA_PLUTUS_HOME`.
- Operators can distinguish persisted verdict success from telemetry delivery success.

## P1 — Runtime and transport hardening

### P1.6 Share one transport-neutral command contract

- [ ] Define typed request/response envelopes and structured error codes once.
- [ ] Route IPC and HTTP through the same validation and command dispatcher.
- [ ] Return correct HTTP status codes for validation, saturation, internal, and not-found
  errors instead of always returning JSON with HTTP 200.
- [ ] Bound IPC line length and HTTP body size; add request timeouts.
- [ ] Cap `/verdicts` limits and make zero-limit behavior explicit.
- [ ] Add readiness/liveness endpoints distinct from the domain status endpoint.
- [ ] Decide local authentication/authorization requirements before non-loopback binding.

Acceptance:

- Contract tests run the same cases through direct, IPC, and HTTP paths.
- Oversized, malformed, saturated, and unknown-command cases return stable structured errors.

### P1.7 Supervise daemon lifecycle safely

- [ ] If IPC or HTTP exits, cancel the sibling task and return the first actionable error;
  avoid waiting forever in `join!`.
- [ ] Before deleting an existing Unix socket, detect an active listener and refuse to steal it.
- [ ] Remove the socket on graceful shutdown when owned by this process.
- [x] Gate `join_error()` with `#[cfg(feature = "http")]` so no-default builds are warning-free.
- [ ] Validate bind addresses and runtime paths before spawning either server.
- [ ] Add graceful shutdown and startup-collision integration tests.

Acceptance:

- Failure of either listener terminates the daemon predictably.
- Starting a second daemon does not disrupt the first.
- `cargo test -p arda-mandos --no-default-features` emits no crate-local warnings.

### P1.8 Turn SSE into a real event surface

- [ ] Publish verdict events at evaluation time rather than polling status every five seconds.
- [ ] Add monotonic event IDs, event schema version, resume semantics, and lag handling.
- [ ] Send explicit degraded/heartbeat events without repeatedly rewriting status snapshots.
- [ ] Bound subscriber count and channel capacity.

Acceptance:

- Tests cover event order, reconnect/resume, slow consumers, and shutdown.

## P2 — API quality, consumers, and operator experience

### P2.1 Harden public helpers

- [ ] Fix `OracleNotifier::format_query()` to truncate on character boundaries rather than byte
  indices, preventing panic on multibyte UTF-8.
- [ ] Accept typed `VerdictOutcome` in notifier APIs instead of free-form strings.
- [x] Implement `Default` for `DefaultTruthScorer` and document its heuristic limitations if it
  remains public.
- [ ] Re-export the intentionally public PageIndex types (`PageTree`, `TocEntry`,
  `SearchResult`) or narrow the API deliberately.
- [ ] Add `#[must_use]` where dropping a verdict/scoring result is likely accidental.

Acceptance:

- Unicode notifier tests, serialization tests, and public API examples pass.

### P2.2 Align direct consumers

- [ ] Migrate `arda-aule` from per-call ephemeral `OracleEngine::new()` to the authoritative
  service/policy path where persistence and policy versioning are required.
- [ ] Decide whether Discord `/query` is an advisory ephemeral query or a persisted Oracle
  request; label the response accordingly.
- [ ] Surface conditions, uncertainty, and key concerns in CLI/HUD/Discord responses, not only
  outcome + resonance.
- [ ] Repair stale Oracle/Annunimas naming and sigil inconsistencies in user-visible strings.
- [ ] Add consumer contract tests before changing enum wire names or verdict fields.

Acceptance:

- All consumers show policy version and advisory/authoritative mode.
- No consumer silently interprets `Conditional` as unconditional approval.

### P2.3 Add operator workflows

- [ ] Add `oracle export` for JSON/JSONL with schema metadata and redaction controls.
- [ ] Add `oracle verify` for ledger integrity and migration readiness.
- [ ] Add filters by query ID, requester, outcome, time range, policy version, and gate.
- [ ] Add a read-only HUD summary: totals, uncertainty distribution, top concerns, degraded
  state, and recent verdicts.
- [ ] Add metrics for latency, outcomes, gate scores, evidence resolution, corruption, queue
  saturation, and side-effect delivery.

Acceptance:

- Operators can inspect and verify Oracle state without hand-reading JSON files.
- Metrics avoid high-cardinality raw query/requester labels.

## P2 — Documentation and quality gates

- [ ] Rename crate docs from "Annunimas ORACLE Module" to Arda/Mandos and explain Oracle's
  advisory authority boundary.
- [ ] Add module docs for context, reasoning, scoring, PageIndex, service, notifier, and
  transports.
- [ ] Add runnable doc examples for `OracleEngine`, `OracleService`, and `PageIndex`.
- [ ] Document every environment variable, default path, bind, feature, and persistence file.
- [ ] Document the verdict and event schemas, compatibility policy, and migration procedure.
- [ ] Keep `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, and directory `INDEX.md` synchronized.
- [ ] Add CI for all-features, no-default-features, formatting, docs, and crate-local Clippy.
- [ ] Resolve the upstream `arda-core` strict-Clippy blockers or scope lint evidence so Mandos
  lint regressions can still be detected.

Acceptance:

- `cargo fmt -p arda-mandos -- --check` passes.
- `cargo test -p arda-mandos --all-features` passes.
- `cargo test -p arda-mandos --no-default-features` passes without crate-local warnings.
- `cargo doc -p arda-mandos --all-features --no-deps` passes with public API examples.
- Strict Clippy passes for Mandos and its dependency closure, or remaining upstream blockers are
  explicitly tracked with command output.

## Recommended execution order

1. P0.1 invariant tests.
2. P0.2 versioned decision policy and veto semantics.
3. P0.3 typed/validated query contract.
4. P0.4 PageIndex correctness, followed by P0.5 evidence provenance.
5. P1.3 governance authority cleanup.
6. P1.4 persistence authority and restart recovery.
7. P1.5 side-effect observability.
8. P1.6/P1.7 transport contract and lifecycle hardening.
9. P1.1/P1.2 richer reasoning context and explainability.
10. P2 consumer/operator/documentation work; split transport only if measured growth warrants it.

## Implementation evidence log

Add one line per completed slice; do not replace historical entries.

| Date | Checklist item | Evidence |
|------|----------------|----------|
| 2026-07-22 | Baseline audit | `cargo test -p arda-mandos --all-features`: 7 passed; `--no-default-features`: 7 passed with one crate-local dead-code warning; strict Clippy blocked in `arda-core` |
| 2026-07-22 | P0.1/P0.2 first policy slice | RED confirmed contradiction, dangerous-operation, evidence monotonicity, and missing-condition failures; GREEN: 15 tests pass under all/no-default features; crate-local strict Clippy, `arda-aule --features full-cli`, and `arda-orome` checks pass |
