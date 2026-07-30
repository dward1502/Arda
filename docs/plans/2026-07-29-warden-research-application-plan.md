# Warden Research First-Party Application Plan

> **For Hermes:** Execute the existing governed-learning plan for receipt-level implementation detail. Use this document as the product/application boundary and avoid creating a parallel research queue.

**Goal:** Turn Warden's bounded scout and Varda's evidence machinery into a useful first-party research application: operators define questions and watchlists, Warden gathers advisory observations, Varda evaluates full sources, and Arda produces cited briefs and governed improvement proposals.

**Architecture:** Warden remains an outpost and observation authority. Varda owns canonical fetch, evidence evaluation, contradiction handling, and knowledge eligibility. Vairë stores approved continuity. Aulë proposes research and product opportunities. The HUD presents the chain. No research result directly mutates policy, tasks, code, or external systems.

**Tech stack:** Existing `arda-outpost-scout`, `arda-outpost-protocol`, `arda-varda`, `arda-vaire`, `arda-aule`, SearXNG-compatible search, Rust/Serde receipts, HUD React surfaces.

**Target stage:** Stage 4 supporting application; mature private beta in Stage 5  
**Canonical implementation dependency:** `docs/plans/2026-07-27-warden-varda-ceo-learning-loop.md`

---

## Verified starting point

- Warden's `RunTopics` command reads `config/outposts/warden/research-topics.json` and a timer runs configured topics.
- `outposts/arda-outpost-scout/src/research.rs` performs SearXNG search and emits advisory observations.
- Warden observations reach Vairë episodic memory but do not yet complete a durable Varda handoff.
- Varda already supports canonicalized ingestion, crawl receipts, deep analysis, opposition harvesting, policy readiness, and knowledge deltas.
- The active governed-learning plan specifies the missing research suggestion, dispatch, external observation, evaluation, approval, memory, and proposal receipt chain.
- The HUD has source trust, freshness, learning loop, knowledge map, action contract, and approval surfaces suitable for projection.

## Product promise

> Give Arda a bounded research question or watchlist. Receive a cited, freshness-aware, contradiction-aware brief that clearly separates search previews, fetched evidence, evaluation, inference, and recommended next actions.

## Authority invariants

- Search snippets are untrusted previews, never canonical evidence.
- Warden can observe and report; it cannot approve knowledge or work.
- Varda can evaluate and recommend; it cannot silently promote policy.
- Vairë stores only the class of record authorized by the approval chain.
- Aulë may propose tasks; existing governance and operator gates decide promotion.
- Every external fetch has budget, expiry, source policy, and provenance.
- Robots, terms, authentication, copyright, and private-source constraints are respected.

## Phase 0 — Complete the governed receipt chain

Do not duplicate Tasks 0.1–4.x from the active learning-loop plan. Implement and close them there:

1. operational truth repair;
2. durable research request/result contracts;
3. Aulë suggestion → Warden dispatch;
4. Warden observation → Varda canonical ingest;
5. Varda evaluation → approved delta;
6. Vairë storage → governed proposal.

**Additional product acceptance**
- One operator question can be traced across every receipt.
- Replaying unchanged input produces no duplicate fetch or knowledge record.
- A failed, blocked, stale, or contradictory result remains useful evidence rather than disappearing.

## Phase 1 — Research workspace API

### Task 1.1: Add research project and watchlist contracts

**Files**
- Create: `outposts/arda-outpost-protocol/src/watchlist.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Test: `outposts/arda-outpost-protocol/tests/watchlist_contract.rs`

**Fields**
- watchlist ID, owner, question, rationale, topic tags;
- source allow/deny policy;
- cadence, expiry, result/fetch/token budget;
- evidence requirements and contradiction policy;
- notification and promotion policy;
- enabled/paused/retired state.

### Task 1.2: Add harness research endpoints

**Files**
- Create: `crates/engine/src/harness/research.rs`
- Modify: `crates/engine/src/harness.rs`
- Test: `crates/engine/tests/harness_research.rs`

**Endpoints**
- `POST /v1/research/questions`
- `GET /v1/research/questions/{id}`
- `POST /v1/research/watchlists`
- `POST /v1/research/watchlists/{id}/pause`
- `GET /v1/research/briefs`
- `GET /v1/research/briefs/{id}`

**Acceptance**
- Read-only mode reports what would run without appending.
- Scheduling never bypasses research-intake governance.

## Phase 2 — Cited brief generation

### Task 2.1: Define a research brief projection

**Files**
- Create: `crates/spine/executors/arda-varda/src/brief.rs`
- Modify: `crates/spine/executors/arda-varda/src/lib.rs`
- Test: `crates/spine/executors/arda-varda/tests/research_brief.rs`

**Brief structure**
- question and scope;
- executive summary;
- claim/evidence table;
- supporting and opposing evidence;
- source quality/freshness;
- unresolved contradictions;
- uncertainty and missing evidence;
- suggested next research or governed proposal;
- complete receipt references.

**Acceptance**
- Every factual claim points to fetched evidence, not a model citation invention.
- “No reliable evidence found” is a valid result.

### Task 2.2: Add change detection

Store normalized source identity and content digest. Emit a new brief only when evidence, evaluation, or expiry materially changes; otherwise write a bounded no-change receipt.

## Phase 3 — HUD research application

### Task 3.1: Build the research workspace

**Files**
- Create: `apps/arda-hud/src/components/arda/modules/ResearchWorkspaceModule.tsx`
- Create: `apps/arda-hud/src/components/arda/modules/ResearchWorkspaceModule.test.tsx`
- Create: `apps/arda-hud/src/components/research/QuestionComposer.tsx`
- Create: `apps/arda-hud/src/components/research/WatchlistPanel.tsx`
- Create: `apps/arda-hud/src/components/research/ResearchBriefView.tsx`
- Create: `apps/arda-hud/src/lib/research.ts`
- Modify: `apps/arda-hud/src/App.tsx`

**Acceptance**
- UI distinguishes preview, fetched source, evaluation, approved knowledge, and proposal.
- Operator can pause a watchlist immediately.
- Brief citations open source details and provenance.

## Phase 4 — Product and technology scouting

Add governed watchlist templates for:

- Arda dependencies and security notices;
- local model/runtime advancements;
- agent and MCP interoperability;
- relevant product-market signals;
- user-selected scientific domains;
- competing product capabilities.

Templates are disabled until selected. Research findings become proposals, not automatic integration work.

## Phase 5 — Reliability and outpost operation

- bounded retries and cooldowns;
- offline queue replay;
- source-domain rate limits;
- poisoned-content/prompt-injection handling;
- Pi5 resource caps;
- signed outpost identity and revocation;
- central fallback when Warden is unavailable;
- retention and compaction for previews versus canonical evidence.

## Verification ladder

```bash
cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1
cargo test --manifest-path outposts/arda-outpost-scout/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-varda --all-features -- --test-threads=1
cargo test -p arda-vaire --all-features -- --test-threads=1
cargo test -p arda-aule --all-features -- --test-threads=1
cd apps/arda-hud && pnpm test && pnpm lint && pnpm build
```

## Release acceptance

- One explicit question and one recurring watchlist complete the entire receipted chain.
- Every brief is source-cited, freshness-aware, and contradiction-aware.
- Restart and repeated schedules do not duplicate requests, fetches, or knowledge.
- Operator can see why a source or claim was rejected.
- No research result directly creates executable work without the existing proposal and approval chain.
