# Warden Research First-Party Application Plan

> **For Hermes:** This plan owns research product contracts, operator APIs, briefs, change detection, and HUD experience. Do not duplicate backend suggestion, dispatch, Varda approval, Vairë knowledge, or Aulë proposal receipts; those belong to the governed-learning plan.

**Status:** Active product plan; reconciled on 2026-07-31
**Target:** Optional Stage 5 private beta; never a Workbench release-candidate blocker
**Canonical backend authority:** [Warden → Varda → Aulë governed learning loop](../archive/2026-07-27-warden-varda-ceo-learning-loop.md)
**Deployment dependency:** [Pi5 deployment/fleet/recovery](../archive/2026-07-23-pi5-outpost-integration-plan.md)

## Product promise

Give Arda a bounded research question or watchlist and receive a cited, freshness-aware, contradiction-aware brief that separates search previews, fetched evidence, evaluation, approved knowledge, and proposed next actions.

## Authority invariants

- Warden observes; Varda evaluates; Vairë stores approved continuity; Aulë may propose.
- Search snippets are untrusted previews and never satisfy a factual claim.
- Product scheduling never bypasses the backend's research-intake gate.
- A brief is advisory evidence. It cannot mutate policy, tasks, code, memory authority, or external systems.
- Every fetch has an explicit budget, expiry, source policy, and provenance chain.

## Stage 4 product baseline

Stage 4 completed the explicit-question minimum:

- `POST /v1/research/brief` exists in `crates/engine/src/harness/research.rs`.
- The path validates a loopback mutation envelope, calls Warden, performs bounded canonical public-source fetches, invokes Varda ingest/deep analysis, writes a stable run-scoped brief, and appends advisory `evidence_linked` state.
- The live record `docs/evidence/stage-4-private-beta/research-chain-live-stage4-research-20260731T181502Z.json` contains two citations, a disclosed failed fetch, stable receipt linkage, and `execution_authorized: false`.
- Focused tests cover stable IDs, bounded excerpts, and private/local target rejection.

The original plan's statement that no durable Varda handoff existed is stale for this explicit Workbench path. The broader governed backend remains incomplete exactly as recorded by the canonical governed-learning plan.

## Ownership boundary after reconciliation

| Concern | Exclusive owner |
|---|---|
| suggestion, dispatch, observation, acknowledgement, Varda approval, delta, knowledge-memory, Aulë proposal receipts | governed-learning plan |
| question/watchlist product contracts and lifecycle | this plan |
| research API, brief projection, change detection, and HUD | this plan |
| Warden AArch64 deployment, fleet identity, SSH, reboot, and shared recovery | Pi5 plan |
| presence, renderer, CITADEL bridge/kiosk, and presentation | RELIC/CITADEL plan |

## Open product tasks — exclusive ownership

### WR-1 — Research project and watchlist contract

**Depends on:** governed-learning GL-2 contract identifiers and authority classes

**Files**

- Create: `outposts/arda-outpost-protocol/src/watchlist.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Test: `outposts/arda-outpost-protocol/tests/watchlist_contract.rs`

**Open work**

- [ ] Define question/watchlist ID, owner, rationale, tags, cadence, expiry, source policy, evidence requirements, contradiction policy, budgets, notification policy, and enabled/paused/retired state.
- [ ] Reference backend suggestion IDs rather than adding a product-owned dispatch queue.
- [ ] Add version, round-trip, malformed, pause/resume, expiry, and unknown-field fixtures.

### WR-2 — Operator research API

**Depends on:** WR-1 and governed-learning GL-2 through GL-4 for recurring execution

**Current compatibility route:** `POST /v1/research/brief`

**Open work**

- [ ] Add question create/read and watchlist create/read/pause/resume/retire endpoints on the existing harness boundary.
- [ ] Add brief list/read endpoints without changing the stable Stage 4 explicit-question route until a versioned migration exists.
- [ ] Preserve loopback/local authentication posture and explicit mutation envelopes.
- [ ] Make read-only mode report the intended backend suggestion without appending it.
- [ ] Prove scheduling cannot bypass research-intake governance or create a second queue.

### WR-3 — Cited brief projection and change detection

**Depends on:** governed-learning GL-4 evaluation/approval receipts

**Current state:** a run-scoped `arda.workbench.research-brief.v1` compatibility brief exists, but its lexical contradiction classification and bounded excerpt summary are narrower than the product promise.

**Open work**

- [ ] Define a durable product brief with question/scope, executive summary, claim/evidence table, supporting/opposing sources, quality/freshness, contradictions, uncertainty, missing evidence, next research/proposal, and complete receipt references.
- [ ] Require every factual claim to point to fetched evidence; accept “no reliable evidence found” as a complete outcome.
- [ ] Record normalized source identity plus content/evaluation/expiry digests.
- [ ] Emit a new brief only when evidence, evaluation, or expiry materially changes; otherwise write a bounded no-change receipt.
- [ ] Preserve disclosed partial failures instead of dropping the entire useful result.

### WR-4 — HUD research workspace

**Depends on:** WR-2 and WR-3

**Files**

- Create a research module and focused components under `apps/arda-hud/src/components/`.
- Create a typed client/projection under `apps/arda-hud/src/lib/`.
- Add focused Vitest coverage before wiring the module into the application shell.

**Open work**

- [ ] Compose explicit questions and bounded watchlists.
- [ ] Distinguish preview, fetched source, evaluation, approved knowledge, and proposal states.
- [ ] Make pause immediately available and show next cadence, budget, stale state, and backend hold reason.
- [ ] Open citation provenance and display rejected/failed source reasons.
- [ ] Provide keyboard-complete, reduced-motion, high-contrast, and screen-reader-labelled flows.

### WR-5 — Product beta reliability

**Depends on:** WR-1 through WR-4; Pi deployment is required only when the beta uses the physical Warden node

**Open work**

- [ ] Bound retries, cooldowns, source-domain rates, result/fetch/token budgets, and retained preview volume.
- [ ] Add poisoned-content/prompt-injection fixtures and make evidence boundaries visible in the brief.
- [ ] Prove offline replay, pause during outage, central fallback policy, and no duplicate brief after restart.
- [ ] Ship disabled watchlist templates for dependency/security notices, model/runtime advances, interoperability, product signals, selected science domains, and competitor capabilities.
- [ ] Keep templates opt-in and findings proposal-only.

## Stage 5 beta dependencies

The optional recurring-watchlist beta may begin only after:

1. governed-learning GL-1 through GL-4 pass their receipt/replay/authority gates;
2. WR-1 establishes product lifecycle and budget/pause contracts;
3. WR-2 routes watchlists through the canonical backend chain;
4. WR-3 proves change detection and contradiction-aware cited briefs;
5. WR-5 proves restart/no-duplicate and prompt-injection handling;
6. PI5-1 passes if the supported beta promises physical Warden deployment.

WR-4 may follow as the beta presentation layer, but no Warden Research task blocks the Stage 5 Workbench release candidate.

## Verification

```bash
cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1
cargo test --manifest-path outposts/arda-outpost-scout/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-engine --all-features -- --test-threads=1
cargo test -p arda-varda --all-features -- --test-threads=1
pnpm --dir apps/arda-hud test
pnpm --dir apps/arda-hud lint
pnpm --dir apps/arda-hud build
```

## Beta acceptance

- [x] One explicit Stage 4 question completes Warden discovery → canonical Varda fetch/evaluation → cited advisory Workbench brief.
- [ ] One recurring watchlist uses the canonical durable backend receipt chain.
- [ ] Budget, pause, source policy, change detection, contradiction handling, and no-change receipts pass.
- [ ] Restart and repeated cadence do not duplicate dispatch, fetch, evaluation, knowledge, or brief records.
- [ ] Operator can understand why a source/claim failed or was rejected and can pause immediately.
- [ ] No research output directly creates executable work.

External-person evaluation is optional supplementary confidence while no separate evaluator or clean machine is available; automated fixtures plus operator acceptance are the active beta gates.