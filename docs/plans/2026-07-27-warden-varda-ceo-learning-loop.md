# Warden → Varda → Aulë Governed Learning Loop

**Status:** Active implementation plan  
**Date:** 2026-07-27  
**Scope:** Complete the missing runtime wiring from research suggestion through governed knowledge learning and CEO proposal generation.  
**Safety posture:** Observe and evaluate automatically; preserve explicit gates for durable knowledge promotion, task creation, and execution.

## Goal

Turn the existing read-only CEO monitor and independently scheduled Warden research into one receipt-driven learning loop:

```text
Aulë/Prometheus research suggestion
  → bounded Warden research dispatch
  → immutable Warden observation receipt
  → Varda canonical-source ingest and digest
  → evidence evaluation and review decision
  → approved KnowledgeDelta
  → Vairë durable learning event
  → Prometheus task proposal
  → existing CEO governance/readiness gates
```

“Automatic” in the initial implementation means research requests may be dispatched and evidence may be evaluated without a human copying files between folders. It does not mean external evidence can silently become policy, mutate the canonical task queue, or authorize execution.

## Naming and authority

- `arda-varda` is the current crate, but substantial public types, storage paths, and schema names still use `Athena`. Do not perform a broad rename while wiring the loop.
- Treat `arda.athena.knowledge_delta.v1` and existing `data/athena/*` paths as compatibility contracts during the first working slice. Record a separate migration decision before introducing `arda.varda.*` schemas or `data/varda/*` paths.
- Warden is a bounded retrieval and observation authority, not a knowledge or policy authority.
- Varda owns source ingestion, evidence analysis, digest generation, and knowledge-delta eligibility.
- Vairë owns durable memory storage, not truth adjudication.
- Prometheus may derive proposals from approved deltas. The CEO loop remains responsible for governance, readiness, task promotion, and execution decisions.

## Verified current state

### Runtime and CEO

- `arda-aule-autopilot-read-only.timer` invokes `arda-cli prometheus autopilot once --read-only` every 30 minutes.
- The current loop reads queue and health state, applies governance checks, reports selections, and writes `data/ceo/autopilot.state.json`; it does not currently produce or consume the research/evidence chain in this plan.
- `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs` expects `config/autonomy_operating_loop.toml`, while the repository contains `config/governance/autonomy_operating_loop.toml`.
- The readiness check expects:
  - `data/prometheus/autonomy_operating_loop_preflight.json`
  - `data/hades/autonomy_cleanup_approval_packets.json`
  - `data/athena/external_source_lane_ledger.jsonl`
  - configured sovereign-adapter evidence
- `config/governance/autonomy_operating_loop.toml` is explicitly `active_draft`; task promotion and enforcement remain disabled.
- `review_required` is emitted by knowledge triage, but `governance_policy.rs` treats an unconfigured action class as unknown and defaults it back to review-required blocking.

### Warden scout

- `outposts/arda-outpost-scout/src/main.rs` supports a static `RunTopics` command using `config/outposts/warden/research-topics.json`.
- `config/systemd/arda-warden-research.timer` runs those configured topics every six hours.
- `outposts/arda-outpost-scout/src/research.rs` performs SearXNG search and converts result snippets into advisory observations.
- `outposts/arda-outpost-scout/src/runtime.rs` persists observations through `ScoutMemoryBridge`.
- `outposts/arda-outpost-scout/src/memory.rs` writes Warden-scoped episodic Vairë events, but no durable external-source handoff exists for Varda or the CEO.
- `outposts/arda-outpost-scout/src/suggestion.rs` concerns repository survey suggestions; it is not a contract for queued internet-research requests.
- `outposts/arda-outpost-protocol` currently defines observation and in-memory queue contracts only. It has no durable research-request, research-result, or acknowledgement schema.

### Varda / legacy ATHENA surfaces

- `crates/spine/executors/arda-varda/src/ingest/*` already provides source classification, canonicalized ingest input, crawl receipts, deep analysis, opposition harvesting, policy readiness, and observability.
- `crates/spine/executors/arda-varda/src/learning.rs` implements `arda.athena.knowledge_delta.v1` emission to `knowledge_deltas.jsonl`.
- `crates/spine/executors/arda-varda/src/ingest/interceptor.rs` can emit policy-ready digest events and write Vairë informant events, but it does not call `emit_delta`.
- No production caller currently completes approved evidence → `KnowledgeDelta`.
- `core/state/athena_active_learning_health.json` is a static MVP projection dated 2026-06-05 and references an uncertainty receipt under `data/athena/` that has no live producer.
- `crates/spine/executors/arda-varda/src/lib.rs` still exports `pub mod human`; this must be audited because the repository no longer uses the former `human/` knowledge-input workflow.

### Memory

- Warden currently persists raw advisory search observations as Vairë episodic events.
- Varda policy-ready ingest also writes Vairë events before any `KnowledgeDelta` handoff.
- `crates/spine/memory/arda-vaire/src/service/promotion.rs` consolidates sufficiently significant repeated episodic records into semantic/procedural records using generic clustering. It does not currently require an approved external-evidence receipt.
- The same module retains an Obsidian sync implementation even though Obsidian and the old human-curated folder are no longer active inputs.
- `core/state/learning_loop_v1.json` has not advanced since 2026-06-12 and is not driven by the current timer.

### Operational defects to resolve first

- `KnowledgeTriageConfig::default_for_workspace()` derives the knowledge root via `workspace_root.parent()`, producing `/var/home/mythos/Eregion/Eregion` from the current workspace.
- The installed `~/.local/bin/arda-cli` was observed behind `target/debug/arda-cli`; service deployment must be explicit and verifiable.
- The workspace build baseline passes with `cargo build --workspace` before this plan is implemented.

## Non-goals

- No immediate autonomous code execution or broad task promotion.
- No direct Warden-to-task-queue mutation.
- No trust in search snippets as canonical evidence.
- No silent conversion of an external observation into semantic/procedural memory.
- No broad ATHENA→Varda rename during runtime activation.
- No reintroduction of a `human/` folder or dependency on an Obsidian vault.
- No retirement of compatibility schemas or paths without a migration plan and fixture coverage.

## Target receipt chain

Every handoff must be append-only, replayable, and idempotent.

1. `arda.prometheus.research_suggestion.v1`
   - Suggested by Prometheus/Aulë from a coverage gap, contradiction, stale source, failed objective, or explicit operator topic.
   - Contains stable ID, query, rationale, scope, risk class, requested evidence criteria, budget, expiry, and parent evidence IDs.
2. `arda.warden.research_dispatch_receipt.v1`
   - Records acceptance/rejection, bounded request parameters, attempt count, and execution timestamps.
3. `arda.warden.external_observation.v1`
   - Records normalized result URL, retrieval/search metadata, snippet as untrusted preview, result rank, content hash when available, and provenance back to the suggestion.
4. `arda.varda.external_source_evaluation.v1`
   - Records canonical fetch/crawl receipt, source identity, recency, provenance, source-quality score, claim set, corroboration/opposition, contradiction flags, and recommendation.
5. `arda.varda.knowledge_approval.v1`
   - Records deterministic gate inputs and one of `approved_safe_local`, `review_required`, `rejected`, or `superseded`.
6. `arda.athena.knowledge_delta.v1`
   - Existing compatibility schema emitted only for approved evidence.
7. `arda.vaire.knowledge_memory_receipt.v1`
   - Confirms durable storage of the delta and preserves all source/evaluation/approval references.
8. Existing Prometheus promotion and CEO receipts
   - May derive a proposal; queue mutation and execution continue through current governance gates.

The external-source lane ledger should be a projection/index over these receipts, not an unrelated manually maintained truth source.

## Phase 0 — Repair operational truth

### Task 0.1: Align autonomy configuration discovery

**Files:**
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs`
- Modify tests: `crates/spine/observability/arda-aule/tests/autopilot_surface.rs`
- Verify config: `config/governance/autonomy_operating_loop.toml`

**Work:**
- Add a failing test proving the canonical repository path is `config/governance/autonomy_operating_loop.toml`.
- Update the loader to use that path. If legacy fallback is retained, expose which path was selected and reject ambiguous dual configuration.
- Keep enforcement and task promotion disabled.

**Acceptance:**
- Autopilot no longer reports the present config as missing.
- State output records the resolved path and `active_draft` status.

### Task 0.2: Fix knowledge-triage source-root derivation

**Files:**
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/knowledge_triage.rs`
- Add/extend tests in the same module.

**Work:**
- Add a failing test for `/var/home/mythos/Eregion/Arda` proving the default source root cannot become `/var/home/mythos/Eregion/Eregion`.
- Derive roots from explicit workspace/config authority rather than an assumed parent-directory layout.
- Emit the resolved root in triage reports.

**Acceptance:**
- Dry-run discovery targets the intended repository/knowledge roots.
- Existing deduplication, classification, and no-mutation behavior remain unchanged.

### Task 0.3: Make `review_required` an explicit governance class

**Files:**
- Modify: `config/governance/autonomy_operating_loop.toml`
- Modify if required: `crates/spine/observability/arda-aule/src/prometheus/autopilot/governance_policy.rs`
- Extend policy tests in that module.

**Work:**
- Define `review_required` as a known, non-executable, human/ORACLE-routed class rather than allowing the unknown-class fallback to hide configuration drift.
- Preserve blocking behavior until an approval receipt exists.

**Acceptance:**
- Reports distinguish “known and awaiting review” from “unknown or unconfigured.”
- No candidate is executable merely because this class becomes known.

### Task 0.4: Generate preflight/readiness projections without weakening gates

**Files:**
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs`
- Modify/create producer code only after tracing current HADES and sovereign-adapter authorities.
- Extend: `crates/spine/observability/arda-aule/tests/autopilot_surface.rs`

**Work:**
- Produce or refresh readiness files from live authorities; do not create empty placeholder files solely to satisfy existence checks.
- Make each readiness check report missing, malformed, stale, or explicitly not ready.
- Treat the external-source lane as `observe_only` until Phases 1–4 are complete.

**Acceptance:**
- `hold` reasons are actionable and evidence-backed.
- Missing evidence never becomes readiness through file existence alone.

### Task 0.5: Deploy and verify the current CLI

**Files:**
- Verify existing systemd unit and install procedure; change only if source and installed paths diverge by design.

**Work:**
- Build the intended release/debug artifact according to the current service installation convention.
- Install atomically to `~/.local/bin/arda-cli`.
- Restart the read-only service/timer and verify the binary hash/version used by systemd.

**Acceptance:**
- The service command resolves to the newly built binary.
- A read-only cycle completes and writes a state receipt from the current source.

## Phase 1 — Define durable research contracts

### Task 1.1: Add research request/result schemas to the outpost protocol

**Files:**
- Create: `outposts/arda-outpost-protocol/src/research.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Add tests alongside the module or under `outposts/arda-outpost-protocol/tests/`.

**Work:**
- Define versioned suggestion, dispatch, observation, and acknowledgement types.
- Include idempotency keys, parent IDs, UTC timestamps, expiry, source scope, fetch budget, result limit, normalized URL, and provenance fields.
- Keep authority classes explicit: suggestion is advisory; Warden result is observation-only.
- Add serde round-trip, unknown-field/version, and dedup-key fixtures.

**Acceptance:**
- Contract fixtures can replay the full request/result handshake without network access.
- A result cannot omit its originating suggestion and dispatch IDs.

### Task 1.2: Add durable append-only queue paths

**Files:**
- Modify: `outposts/arda-outpost-scout/src/memory.rs`
- Modify: `outposts/arda-outpost-scout/src/runtime.rs`
- Modify: `config/systemd/arda-warden-research.service`
- Extend: `outposts/arda-outpost-scout/tests/runtime_api.rs`
- Extend: `outposts/arda-outpost-scout/tests/memory_fixtures.rs`

**Proposed runtime projections:**
- `data/warden/research_suggestions.jsonl`
- `data/warden/research_dispatch_receipts.jsonl`
- `data/warden/external_observations.jsonl`
- `data/warden/research_acknowledgements.jsonl`

**Work:**
- Use append-only records and an observed cursor rather than destructive dequeue.
- Deduplicate suggestions by stable suggestion ID and results by normalized URL plus content/provenance hash.
- Persist accepted, rejected, expired, failed, and completed outcomes.
- Keep configured static topics as one suggestion producer, not a separate untraceable path.

**Acceptance:**
- Restarting Warden replays safely without duplicate dispatch or observation records.
- A failed request remains retryable with bounded attempts and visible failure evidence.

## Phase 2 — Connect Prometheus suggestions to bounded Warden dispatch

### Task 2.1: Produce research suggestions from explicit evidence gaps

**Files:**
- Create: `crates/spine/observability/arda-aule/src/prometheus/autopilot/research.rs`
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/mod.rs`
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs`
- Add tests in the new module and `crates/spine/observability/arda-aule/tests/autopilot_surface.rs`.

**Work:**
- Convert only typed triggers into research suggestions: explicit configured topic, knowledge coverage gap, stale evidence, contradiction, or failed objective requiring information.
- Cap suggestions per cycle, enforce cooldowns, assign expiry, and deduplicate against unresolved prior suggestions.
- In read-only mode, report “would suggest” but do not append. Add a separate `research_intake_allowed` gate for append-only suggestion production.
- Never derive a task directly from a suggestion.

**Acceptance:**
- Identical unchanged evidence does not emit a new suggestion every 30 minutes.
- Read-only mode remains mutation-free.
- Enabled intake changes only the suggestion ledger.

### Task 2.2: Replace static-only scheduling with a queue-aware dispatcher

**Files:**
- Modify: `outposts/arda-outpost-scout/src/main.rs`
- Modify: `outposts/arda-outpost-scout/src/research.rs`
- Extend: `outposts/arda-outpost-scout/tests/runtime_cli.rs`
- Extend: `outposts/arda-outpost-scout/tests/research_fixtures.rs`
- Modify: `config/systemd/arda-warden-research.service`

**Work:**
- Add a bounded command that consumes unresolved durable suggestions, dispatches searches, and writes receipts.
- Convert `research-topics.json` entries into the same suggestion contract or retain them only as bootstrap inputs.
- Enforce request expiry, URL/domain policy, maximum result count, timeout, and retry budget.
- Persist snippets as previews explicitly marked untrusted; do not present them as fetched source content.

**Acceptance:**
- A queued suggestion causes one bounded Warden dispatch and traceable result set.
- Static and dynamic topics share one receipt chain.
- No Vairë semantic/procedural promotion occurs at this stage.

## Phase 3 — Varda external-source intake and autonomous digest

### Task 3.1: Add a Warden observation importer and external-source ledger

**Files:**
- Create: `crates/spine/executors/arda-varda/src/ingest/external_lane.rs`
- Modify: `crates/spine/executors/arda-varda/src/ingest.rs`
- Modify: `crates/spine/executors/arda-varda/src/ingest/layout.rs`
- Add focused tests in the new module.

**Work:**
- Read Warden observation receipts by cursor.
- Validate schema, authority, parent chain, expiry, normalized URL, and duplicate keys.
- Append accepted/rejected import decisions to `data/athena/external_source_lane_ledger.jsonl` as a compatibility projection.
- Do not treat the search snippet as ingested source content.

**Acceptance:**
- Valid Warden observations become pending Varda candidates exactly once.
- Malformed, duplicate, expired, or unsupported candidates remain auditable and cannot enter analysis.

### Task 3.2: Fetch canonical content and preserve crawl evidence

**Files:**
- Modify: `crates/spine/executors/arda-varda/src/ingest/io.rs`
- Modify: `crates/spine/executors/arda-varda/src/ingest/source.rs`
- Reuse/extend: `crates/spine/executors/arda-varda/src/ingest/activity.rs`
- Add deterministic crawl fixtures and tests.

**Work:**
- Route accepted URLs through the existing crawl/capture path.
- Record redirects, final URL, retrieval time, HTTP/content metadata, hash, and extraction outcome.
- Reject unsupported/private/local targets unless explicitly allowed by policy.
- Preserve source content/crawl receipt linkage independently of model-generated summaries.

**Acceptance:**
- Every analyzed external source has a successful canonical crawl receipt and content hash.
- Fetch failure produces a terminal/retryable evaluation outcome, never a fabricated digest.

### Task 3.3: Produce evidence-aware autonomous digests

**Files:**
- Modify: `crates/spine/executors/arda-varda/src/ingest/interceptor.rs`
- Modify: `crates/spine/executors/arda-varda/src/ingest/policy.rs`
- Modify as needed: `crates/spine/executors/arda-varda/src/ingest/deep.rs`
- Extend unit fixtures around policy readiness and opposition harvesting.

**Work:**
- Score provenance, recency, source quality, citation integrity, corroboration, opposition coverage, and contradiction.
- Extract claims with source spans/URLs and preserve opposing evidence rather than flattening it into one summary.
- Generate a digest and approval packet automatically when sufficient evidence exists.
- Route uncertainty, health/safety-sensitive claims, contradictions, low provenance, and strategy/policy implications to review.
- Replace any old `human/`-folder assumption with receipt-driven review status; do not recreate that folder.

**Acceptance:**
- Varda can turn one completed Warden research request into a replayable digest/evaluation packet without manual file placement.
- A digest by itself has no memory-promotion or task-queue authority.

## Phase 4 — Establish the approval and rejection gate

### Task 4.1: Define evidence approval policy

**Files:**
- Add a focused section/file under `config/governance/` after choosing whether this belongs in `autonomy_operating_loop.toml` or a Varda-specific policy.
- Modify: `crates/spine/executors/arda-varda/src/ingest/policy.rs`
- Integrate with existing ORACLE surfaces rather than creating a second governance engine.

**Work:**
- Define deterministic outcomes: `approved_safe_local`, `review_required`, `rejected`, `superseded`.
- Require provenance, canonical fetch receipt, content hash, quality threshold, and no unresolved high-severity flags for safe-local approval.
- Require ORACLE/human review for uncertain, contradictory, strategy-changing, privacy-sensitive, medical/health, destructive, or externally consequential claims.
- Keep rejected evidence recallable in the evidence archive while excluding it from knowledge and policy projections.

**Acceptance:**
- The same immutable inputs always produce the same pre-model gate outcome.
- Approval is represented by a receipt, not inferred from file location or Vairë presence.

### Task 4.2: Replace obsolete human-folder coupling

**Files:**
- Audit/modify: `crates/spine/executors/arda-varda/src/human.rs`
- Audit all call sites before deleting or renaming the module.
- Update active contracts/docs that require `human/` or Obsidian as live authorities.

**Work:**
- Preserve useful review queue and approval concepts behind receipt-based paths/APIs.
- Retire only genuinely dead filesystem assumptions after tests prove no runtime caller depends on them.
- Keep an operator-facing review surface possible without making a local notes folder canonical.

**Acceptance:**
- No active learning path requires `human/` or an Obsidian vault.
- Review-required evidence remains visible and actionable.

## Phase 5 — Emit knowledge deltas and correct memory authority

### Decision gate before implementation

Before Task 5.1, decide the memory model described under “Discussion after plan.” The implementation must distinguish raw observations, evaluated evidence, approved knowledge, and learned operational outcomes.

### Task 5.1: Emit `KnowledgeDelta` only from approved evidence

**Files:**
- Modify: `crates/spine/executors/arda-varda/src/learning.rs`
- Modify: `crates/spine/executors/arda-varda/src/ingest/interceptor.rs`
- Add focused tests proving eligibility and idempotency.

**Work:**
- Call delta emission only when an approval receipt authorizes it.
- Extend provenance references as needed without breaking `arda.athena.knowledge_delta.v1` compatibility.
- Make delta ID deterministic from approved evaluation/version so replay cannot duplicate learning.
- Emit correction/supersession relationships rather than silently overwriting prior knowledge.

**Acceptance:**
- Policy-ready but unapproved evidence produces zero deltas.
- Approved evidence produces exactly one delta across retries.
- Rejected/superseded evidence cannot be mistaken for current approved knowledge.

### Task 5.2: Add a governed Vairë knowledge-ingest path

**Files:**
- Create: `crates/spine/memory/arda-vaire/src/service/knowledge.rs`
- Modify: `crates/spine/memory/arda-vaire/src/service.rs`
- Modify: `crates/spine/memory/arda-vaire/src/lib.rs`
- Extend: `crates/spine/memory/arda-vaire/tests/knowledge_deltas.rs`

**Work:**
- Accept approved deltas with source, evaluation, and approval references.
- Persist a distinct knowledge-memory receipt.
- Enforce idempotency and correction/supersession semantics.
- Preserve observational Warden memory, if retained, as a non-promotable tier with explicit retention and authority labels.

**Acceptance:**
- Vairë can prove why a knowledge record exists and which approval authorized it.
- Raw Warden observations cannot become semantic knowledge merely through repetition/significance.

### Task 5.3: Harden consolidation against unapproved external evidence

**Files:**
- Modify: `crates/spine/memory/arda-vaire/src/service/promotion.rs`
- Extend: `crates/spine/memory/arda-vaire/tests/public_flows.rs`

**Work:**
- Require authority/eligibility metadata before external-research episodic events participate in semantic or procedural consolidation.
- Separate frequency/significance from truth confidence.
- Preserve receipt references in every promoted semantic/procedural record.
- Mark Obsidian sync as optional import tooling or retire it separately; it must not be a prerequisite for memory health.

**Acceptance:**
- Repeated unapproved observations remain observations.
- Approved deltas can be recalled without duplicate promotion artifacts.
- Consolidation reports distinguish observed, eligible, promoted, and blocked records.

## Phase 6 — Feed approved learning into Prometheus proposals

### Task 6.1: Add a delta consumer with an observed cursor

**Files:**
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/learning.rs`
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs`
- Extend autopilot tests.

**Work:**
- Consume approved deltas idempotently.
- Classify each delta as informational-only, research-follow-up, safe-local proposal candidate, or governed review candidate.
- Persist outcome and cursor receipts; do not rely on `core/state/learning_loop_v1.json` as the sole mutable authority.
- Keep proposal generation separate from task promotion.

**Acceptance:**
- A delta is processed once and its disposition is replayable.
- Informational learning does not force creation of a task.
- Proposal candidates preserve complete provenance to Warden suggestion and Varda approval.

### Task 6.2: Connect proposals to existing governance and queue promotion

**Files:**
- Reuse/modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/knowledge_triage.rs`
- Reuse/modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/governance_policy.rs`
- Reuse existing planner, queue writer, ORACLE, HADES, and executor bridges.

**Work:**
- Route safe-local proposal candidates through existing promotion receipts.
- Route review-required and strategy-changing proposals through configured review/ORACLE paths.
- Preserve `task_promotion_allowed=false` until Phase 7 canary criteria are met.

**Acceptance:**
- End-to-end dry run reports one selected delta and proposed disposition without queue mutation.
- Enabling safe-local proposal generation still cannot bypass CEO readiness or action-class policy.

## Phase 7 — Progressive activation

### Stage A: Observe-only integration

- Produce no new suggestions or mutations.
- Read existing Warden/Varda/memory surfaces and report missing/invalid links.
- Verify receipt-chain metrics and dashboard visibility.

### Stage B: Automatic research intake

- Allow bounded suggestion append and Warden dispatch.
- Keep Varda evaluation, delta emission, proposal generation, queue promotion, and execution disabled.

### Stage C: Automatic Varda evaluation

- Enable canonical fetch, digest, scoring, and approval-packet generation.
- Keep delta emission review-gated.

### Stage D: Safe-local knowledge promotion

- Allow only policy-approved low-risk deltas into Vairë.
- Allow Prometheus to classify/propose, but keep task promotion disabled.

### Stage E: Safe-local proposal generation

- Enable proposal receipts for eligible deltas.
- Require explicit canary budget, rollback switch, stale-input checks, and per-cycle cap.

### Stage F: Bounded task promotion and execution

- Consider only after sustained receipt integrity, no duplicate processing, review queue operability, current preflight, ORACLE/HADES readiness, and successful rollback exercises.
- Enable one action class at a time; never use a global autonomy switch as a substitute for class-specific policy.

## Required observability

Add a cycle projection that reports at minimum:

- research suggestions created, deduplicated, expired, and blocked
- Warden dispatches attempted, completed, failed, and retried
- external observations accepted, rejected, and duplicated
- canonical fetch success/failure and source staleness
- Varda evaluations by decision and blocker
- deltas emitted, deduplicated, corrected, and superseded
- Vairë knowledge receipts and blocked consolidation attempts
- Prometheus delta dispositions and proposals
- review queue age and unresolved high-severity contradictions
- queue mutations and executions, expected to remain zero through Stage E

Every count must link to a durable receipt path/cursor; dashboard-only counters are not sufficient.

## End-to-end tests

### Fixture test

Build a deterministic local fixture containing:

- one bounded research suggestion
- duplicate search results with URL normalization differences
- one successful canonical source
- one corroborating source
- one opposing source
- one stale/low-quality source
- one retryable fetch failure

Assert the expected dispatch, deduplication, crawl, evaluation, approval, delta, memory, and proposal receipts without internet access.

### Negative tests

- snippet-only evidence cannot produce a delta
- malformed parent chain is rejected
- duplicate replay does not duplicate dispatch, delta, memory, or proposal records
- expired suggestion is not dispatched
- contradictory/high-risk evidence requires review
- rejected evidence remains recallable but is absent from approved knowledge
- repeated raw Warden observations cannot become semantic memory
- stale approval/preflight cannot authorize promotion
- read-only CEO mode performs no mutation

### Canary test

Run one explicitly selected safe-local research topic through the live chain with:

- task promotion disabled
- execution disabled
- exact receipt IDs captured at every stage
- service restart between dispatch and Varda import to prove replay safety
- a second identical cycle to prove idempotency

## Verification commands

Run focused gates after each phase, then the workspace gate:

```bash
cargo test -p arda-outpost-protocol
cargo test -p arda-outpost-scout
cargo test -p arda-varda
cargo test -p arda-vaire
cargo test -p arda-aule
cargo build --workspace
cargo test --workspace
```

Also verify runtime authority directly:

```bash
systemctl --user status arda-aule-autopilot-read-only.timer --no-pager
systemctl --user status arda-aule-autopilot-read-only.service --no-pager
systemctl --user status arda-warden-research.timer --no-pager
systemctl --user status arda-warden-research.service --no-pager
```

Capture resolved config paths, binary hashes, state timestamps, and receipt counts. Do not mark a phase complete from compile success alone.

## Completion criteria

The plan is complete when:

1. A typed Prometheus suggestion can automatically trigger bounded Warden research.
2. Warden produces durable, deduplicated observation receipts with no knowledge authority.
3. Varda fetches canonical sources, generates an evidence-aware digest, and records an approval decision without a `human/` folder or Obsidian.
4. Only approved evidence emits one versioned `KnowledgeDelta`.
5. Vairë stores the approved learning event with complete provenance and cannot semantically promote raw research observations.
6. Prometheus consumes the delta once and may create a governed proposal.
7. The CEO still applies explicit readiness, review, ORACLE/HADES, task-promotion, and execution gates.
8. Observe-only through safe-local proposal stages have reproducible end-to-end receipts and rollback controls before any bounded execution is considered.

## Discussion after plan: decisions to settle before implementation

1. **Memory tiers:** Should raw Warden observations remain in Vairë episodic memory with a non-promotable authority label and retention window, or should Vairë receive only approved Varda deltas while raw observations stay solely in evidence storage?
2. **Digest versus memory:** Should Varda’s digest be durable evidence only, with the approved delta as the sole knowledge-memory input, or should approved digests and deltas have separate recall roles?
3. **Research suggestion authority:** Which producers may create automatic suggestions initially: static operator topics only, Prometheus coverage gaps, failed objectives, Warden’s own anomaly detection, or a staged subset?
4. **Review surface:** With no `human/` directory or Obsidian, should review be represented first by an append-only approval ledger plus CLI, or should it target an existing HUD/ARDA approval view?
5. **Naming migration:** Keep `data/athena/*` and `arda.athena.*` as durable compatibility names indefinitely, or schedule a versioned Varda migration after the loop works?
6. **External-source retention:** How long should rejected, stale, and raw fetched content remain locally, especially for privacy-sensitive or health-adjacent research?
7. **First canary topic:** Choose a low-risk topic whose expected evidence and contradiction behavior can be independently verified before enabling dynamic CEO-generated research.
