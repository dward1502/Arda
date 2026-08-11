# Warden → Varda → Aulë Governed Learning Loop

> **For Hermes:** This is the canonical Warden/Varda backend authority. Keep research product/API/HUD tasks in the Warden Research plan, Pi deployment/fleet/recovery in the Pi5 plan, and all RELIC/CITADEL presence/presentation work in the RELIC/CITADEL plan.

**Status:** Complete; archived after GL-1 through GL-6 acceptance and workspace verification on 2026-08-02
**Scope:** Durable, replayable research dispatch, evidence evaluation, approval, governed memory, and proposal receipts
**Stage posture:** Required only for the optional Stage 5 Warden Research beta; not a Workbench release-candidate blocker
**Product dependency:** [Warden Research](2026-07-29-warden-research-application-plan.md)

## Goal and authority

```text
Aulë research suggestion
  → bounded Warden dispatch
  → immutable Warden observation receipt
  → Varda canonical fetch, evaluation, and approval decision
  → approved KnowledgeDelta
  → Vairë governed knowledge receipt
  → Aulë proposal disposition
  → existing CEO governance/readiness gates
```

- Warden is a retrieval and observation authority, never a knowledge, task, or execution authority.
- Varda owns canonical fetch, evidence evaluation, contradiction handling, and knowledge eligibility.
- Vairë owns durable memory, not truth adjudication.
- Aulë may propose work; existing review, HADES/ORACLE, promotion, and execution gates remain authoritative.
- `arda.athena.*` schemas and `data/athena/*` remain compatibility names until a separate migration is approved.
- Search snippets are previews. Only canonical fetched content with provenance may enter evaluation.

## Stage 4 evidence and its boundary

Stage 4 completed one explicit-question compatibility slice:

- `crates/engine/src/harness/research.rs` exposes `POST /v1/research/brief`.
- It calls the Warden scout, fetches bounded public HTTP(S) results, records Varda crawl/deep-analysis evidence, writes a stable Workbench brief, and appends an `evidence_linked` run event.
- `docs/evidence/stage-4-private-beta/research-chain-live-stage4-research-20260731T181502Z.json` proves two cited sources, one failed fetch, `reference_only` policy readiness, and `execution_authorized: false`.
- The Stage 4 plan was operator-accepted with external-person evaluation explicitly optional because no separate evaluator or clean machine is available.

This is real product evidence, but it is not the durable governed-learning loop: the harness path has no suggestion/dispatch/acknowledgement ledgers, no Varda approval receipt, no approved-delta handoff, no governed Vairë knowledge receipt, and no Aulë delta consumer.

## Live-source audit

### Implemented foundations

- `outposts/arda-outpost-protocol` defines observation, authority, in-memory queue, and runtime-presence contracts.
- `outposts/arda-outpost-scout` enforces bounded source policy/expiry, emits advisory observations, and writes append-only Warden-scoped Vairë receipts.
- `arda-varda` provides canonical ingestion, crawl capture, deep analysis, opposition/policy machinery, and the compatibility `KnowledgeDelta` type/emitter.
- `arda-vaire` provides append-only episodic storage, recall, and generic consolidation.
- Workbench can consume one explicit Warden question as advisory run evidence without changing graph authority.

### Confirmed gaps and stale claims removed

- `outposts/arda-outpost-protocol` now has a durable append-only suggestion ingress with idempotency and a restart-safe cursor; complete-chain receipts remain separately persisted.
- Warden now exposes advisory `/suggestions` ingress and cursor-driven `/dispatch`; static topics remain a bootstrap producer of the same typed suggestion contract, covered by `static_topics_produce_the_typed_durable_suggestion_contract`.
- `external_lane.rs` now validates persisted Warden chains, imports the next canonical result by cursor through the Crawl4AI adapter, and exposes that operation through Varda's `/external_lane/import` service route.
- `human` filesystem scanning is now opt-in via the `human-import` feature; the default Varda runtime is receipt-driven.
- Vairë's default HTTP/IPC transports no longer expose Obsidian sync or project Obsidian entries into the live event stream; the direct sync method remains optional import tooling.
- `arda-aule` resolves `config/governance/autonomy_operating_loop.toml` as the canonical autonomy config, rejects simultaneous legacy/canonical files, and reports the selected path in preflight output.
- `KnowledgeTriageConfig::for_root` uses explicit repository-local source roots and no longer derives a parent-based `Eregion` root.
- `review_required` is an explicit non-executable review class with fail-closed delegation behavior.
- Readiness code produces a typed preflight projection, reports incomplete lanes and missing required stages, and preserves `task_promotion_allowed=false`.
- Earlier static health timestamps and installed-binary observations are not retained as current claims; runtime deployment truth must be reverified when implementation reaches activation.

### Integration evidence added after the foundation slice

- Protocol suggestion-ledger restart/idempotency fixture passes.
- Scout runtime suggestion ingress deduplicates persisted records; the existing search path now records and advances the same cursor.
- Aulë exposes `emit_research_suggestion` and its approved-delta consumption ledger is idempotent with complete source/Warden/Varda/approval provenance.
- Varda exposes cursor-driven canonical-result import; it calls the Crawl4AI markdown endpoint, validates canonical content and URL provenance hashes, and advances only after evaluation receipt persistence. The HTTP service exposes this as `POST /external_lane/import`.
- Repository-owned Varda/Crawl4AI services plus caller/timer units now invoke one bounded import every 15 minutes through that route; the development host has all three services enabled and active.
- Operator procedure is documented in `docs/operator/warden-varda-external-lane.md`, including exact endpoint checks, installation, one-shot smoke, timer activation, and durable-ledger verification.
- Aulë now has an offline restart/replay fixture spanning persisted Warden chain → Varda evaluation → approved delta → Vairë receipt → Aulë consumption, with no duplicate receipt on replay.
- Focused verification: protocol suggestion-ledger fixture passed; Scout package tests passed; Varda full suite passed with 122 tests; the live Crawl4AI importer fixture passed; Aulë full suite passed with 181 tests; and the offline restart/replay fixture passed without duplicate receipts.
- `systemd-analyze verify` passes for the external-lane service and timer; the Varda HTTP transport suite passes with 10 tests.
- Live deployment verified: Varda `/status` and Crawl4AI `/health` returned HTTP 200; two manual caller runs completed with `receipt:null` because no unconsumed Warden observation was pending; the timer is enabled and active.

## Open backend tasks — exclusive ownership

Every unchecked task below is owned only by this plan.

### GL-1 — Repair operational configuration truth

**Files:** `arda-aule` autopilot runner/policy/triage tests and `config/governance/autonomy_operating_loop.toml`

- [x] Resolve the canonical autonomy config path in code and config metadata; reject ambiguous dual files and report the selected path/status.
- [x] Replace parent-derived knowledge roots with explicit repository/config roots and report resolved roots.
- [x] Configure `review_required` as a known, non-executable review class while retaining fail-closed behavior.
- [x] Trace and implement live preflight/readiness producers; distinguish missing, malformed, stale, and explicitly-not-ready evidence.
- [x] Keep task promotion and enforcement disabled while later GL tasks are incomplete.

**Acceptance:** one read-only autopilot cycle reports correct paths and actionable holds without creating placeholder readiness evidence.

### GL-2 — Define the durable research receipt chain

**Files:** `outposts/arda-outpost-protocol/src/research.rs`, protocol exports, fixtures, and append-only Warden storage

- [x] Add versioned suggestion, dispatch, external-observation, and acknowledgement types with parent IDs, idempotency keys, UTC times, expiry, budgets, normalized URLs, content/provenance hashes, and explicit advisory authority.
- [x] Add replay, unknown-version/field, malformed-parent, expiry, and dedup fixtures.
- [x] Add append-only suggestion/dispatch/observation/acknowledgement ledgers plus observed cursors; never use destructive dequeue.
- [x] Preserve static topics only as bootstrap producers of the same suggestion contract.

**Acceptance:** the complete request/result handshake replays offline and a result cannot omit its suggestion and dispatch parents.

### GL-3 — Connect Aulë suggestions to bounded Warden dispatch

**Files:** `arda-aule` autopilot research producer and `arda-outpost-scout` queue-aware CLI/runtime

- [x] Emit typed suggestions only from explicit topics, coverage gaps, stale evidence, contradictions, or failed objectives requiring information.
- [x] Cap, expire, cool down, and deduplicate suggestions; read-only mode reports without appending.
- [x] Consume unresolved suggestions with bounded attempts and write accepted, rejected, expired, failed, and completed dispatch receipts.
- [x] Mark snippets untrusted and prohibit task, policy, semantic-memory, or execution mutation.

**Acceptance:** one queued suggestion produces one replay-safe bounded dispatch and result set across restart.

### GL-4 — Add Varda intake, evaluation, and approval receipts

**Files:** `arda-varda/src/ingest/external_lane.rs`, ingest/crawl/policy modules, and receipt fixtures

- [x] Import Warden observations by cursor; validate schema, authority, parents, expiry, normalized URL, and duplicate key.
- [x] Project accepted/rejected import decisions into `data/athena/external_source_lane_ledger.jsonl` as an index, not an independent truth source.
- [x] Reuse canonical fetch/crawl evidence and preserve redirect, final URL, retrieval metadata, content hash, extraction result, and failures.
- [x] Emit deterministic `approved_safe_local`, `review_required`, `rejected`, or `superseded` receipts from provenance, quality, contradiction, privacy, and risk inputs.
- [x] Replace active `human/`/Obsidian coupling with receipt-driven review while retaining optional import tooling only where independently useful.
- [x] Emit one deterministic compatibility `arda.athena.knowledge_delta.v1` only from an approval receipt; support correction/supersession without overwrite.

**Acceptance:** one Warden result becomes a replayable canonical evaluation; snippet-only, failed, contradictory/high-risk, unapproved, or replayed evidence cannot create a delta.

### GL-5 — Enforce governed Vairë knowledge authority

**Files:** a dedicated Vairë knowledge-ingest service plus consolidation policy/tests

- [x] Accept only approved deltas carrying source, evaluation, and approval references.
- [x] Persist a distinct idempotent knowledge-memory receipt with correction/supersession semantics.
- [x] Keep raw Warden observations non-promotable regardless of repetition/significance.
- [x] Preserve approval references in every derived semantic/procedural record and report observed, eligible, promoted, and blocked counts.

**Acceptance:** Vairë can prove why approved knowledge exists, while repeated raw external observations remain observations.

### GL-6 — Consume approved learning in Aulë without bypassing governance

**Files:** `arda-aule` learning consumer, autopilot runner, policy, receipts, and end-to-end fixtures

- [x] Consume deltas once by observed cursor and classify informational, research-follow-up, safe-local proposal candidate, or governed-review candidate.
- [x] Persist complete provenance to the originating suggestion, Warden dispatch/observation, Varda evaluation/approval, and Vairë receipt.
- [x] Keep proposal generation separate from queue promotion and preserve `task_promotion_allowed=false` through canary evidence.
- [x] Add observe-only → intake → evaluation → safe-local knowledge → proposal activation switches, per-cycle caps, stale-input checks, rollback, and metrics linked to durable receipts.

**Acceptance:** an end-to-end fixture and one selected low-risk canary survive restart/replay with no duplicate dispatch, delta, memory, proposal, queue mutation, or execution.

## Required end-to-end fixtures

- one explicit suggestion and one static-topic suggestion;
- normalized duplicate search results;
- successful, corroborating, opposing, stale/low-quality, and retryable-failure sources;
- malformed parent chain, expired suggestion, and unsupported/private target;
- snippet-only, contradictory/high-risk, rejected, and superseded evidence;
- service restart between dispatch/import and a repeated identical cycle;
- task promotion and execution disabled.

## Activation order

1. **Observe only:** validate existing evidence and report broken links.
2. **Research intake:** allow suggestion append and Warden dispatch only.
3. **Varda evaluation:** enable canonical fetch/evaluation and approval-packet generation; no delta without approval.
4. **Safe-local knowledge:** permit only policy-approved low-risk deltas into Vairë; no task promotion.
5. **Proposal receipts:** allow bounded Aulë dispositions; no queue mutation.
6. **Future bounded promotion:** consider only after sustained integrity, operable review, rollback exercises, and existing CEO readiness gates.

## Stage 5 dependency

- Workbench release-candidate packaging, security, reliability, and support work does **not** depend on GL-1 through GL-6.
- The optional Warden Research recurring-watchlist beta depends on GL-1 through GL-4 and on its own product contracts, pause, change-detection, and brief gates.
- Knowledge promotion or governed improvement proposals additionally depend on GL-5 and GL-6.
- Pi deployment is tracked only in the Pi5 plan. RELIC/CITADEL has no dependency on this learning loop for its presence beta.

## Verification

```bash
cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1
cargo test --manifest-path outposts/arda-outpost-scout/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-varda --all-features -- --test-threads=1
cargo test -p arda-vaire --all-features -- --test-threads=1
cargo test -p arda-aule --all-features -- --test-threads=1
cargo test --workspace --all-features -- --test-threads=1
```

Compile success alone does not close a task: each acceptance statement requires the named receipt, replay, authority, and no-mutation evidence.

## Completion criteria

- [x] One typed suggestion triggers one bounded Warden dispatch with durable deduplicated observations.
- [x] Varda performs canonical fetch/evaluation and records a deterministic approval decision without filesystem review authority.
- [x] Only approved evidence emits one versioned delta.
- [x] Vairë stores approved knowledge with complete provenance and cannot promote raw research observations.
- [x] Aulë consumes the delta once and may create a governed proposal without bypassing readiness or review.
- [x] Observe-only through proposal stages have reproducible restart, replay, rollback, and no-duplicate evidence.