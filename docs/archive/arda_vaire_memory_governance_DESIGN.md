# DESIGN: arda-vaire memory domain governance

**Status:** implemented and verified 2026-08-05. Retention, correction/supersession, governed compression, scope-policy mediation, provenance quarantine, and revoke cascades are operational through `MnemosyneService`. Companion persona work is complete under `docs/archive/arda_vaire_avatar_memory_PLAN.md`.

**Framing:** arda-vaire already does two of the five operations any memory system needs — store and retrieve — well. This doc specifies the other three (update, compress, forget) plus a scope-enforcement layer, using patterns from current agentic-memory and memory-security research rather than inventing new ones. One store. No new database. No new sync pipeline.

---

## 0. The five-operation checklist

| Operation | Current state | This doc |
|---|---|---|
| Store | Solid — episodic write via `service/store.rs` | unchanged |
| Retrieve | Scoped lexical and canonical recall | mediated by the policy pipeline (§3) |
| Update | Explicit correction/supersession chain | implemented by §2 |
| Compress | Explicit governed batch operation | implemented by §2 |
| Forget | Retention decay plus receipt-backed revoke cascade | implemented by §§1 and 4 |

Rationale for treating this as a checklist rather than a vibe: append-only stores without an update path leave old and new facts coexisting with no signal for which is current, and stores without a forget path accumulate stale entries that quietly degrade every future retrieval. Both are the failure modes reuse-first design is supposed to prevent — better to name them now than discover them as a debugging session six months in.

---

## 1. Retention scoring — one formula, three sets of constants

**File:** `crates/spine/memory/arda-vaire/src/service/retention.rs`

Single shared decay utility, reused by system-scope forgetting, business-scope retention, personal-scope staleness flagging, and the persona `mood_summary` decay from the avatar plan. Same math, different constants per caller — do not fork this into per-domain copies.

```rust
pub struct RetentionScore {
    pub recency: f32,      // exponential decay, half-life configurable per scope
    pub importance: f32,   // existing significance-gate score, reused as-is
    pub retrieval_freq: f32, // access count, decayed same as recency
    pub composite: f32,    // weighted combination, see below
}

pub struct RetentionConfig {
    pub half_life_hours: f32,
    pub importance_weight: f32,
    pub recency_weight: f32,
    pub retrieval_weight: f32,
    pub floor: f32,          // composite score below which a record is forget-eligible
}
```

`composite = importance_weight * importance + recency_weight * recency + retrieval_weight * retrieval_freq`, weights sum to 1.0. `recency` and `retrieval_freq` both use the same exponential decay shape already specified for `persona.mood_summary` in the avatar plan (`weight = exp(-λ * age_hours)`, `λ = ln(2) / half_life_hours`) — this is the shared utility referenced above.

**Per-domain config (defaults, tune later against real usage, not against theory):**

| Scope | half_life_hours | importance_weight | floor | Notes |
|---|---|---|---|---|
| business | 720 (30d) | 0.6 | 0.15 | importance-dominated; a low-recency but high-importance decision record should survive |
| personal | 336 (14d), operator-authored records bypass decay entirely (recency locked to 1.0) | 0.5 | 0.10 | never auto-decays below floor for confirmed facts; unconfirmed/inferred candidates decay normally |
| system | 24 (1d) raw, 720 (30d) for promoted fault signatures | 0.3 | 0.25 | raw noise should fall below floor fast; promoted signatures get the business-scope half-life once promoted |

**Forget mechanics:** a record whose `composite` falls below `floor` on a given consolidation pass is transitioned `active -> decayed` (existing `MemoryRecord.state` value — no new state needed). Decayed records are excluded from recall by default but not physically deleted; physical deletion is a separate, explicit operation (§4), because decay and erasure are different guarantees and conflating them is exactly the mistake the "infinite retention is an architectural bug" framing warns against in the other direction — silent auto-delete of decayed-but-not-forgotten records is its own failure mode (a business record you'd want back later).

**Acceptance criteria:**
- [x] `retention.rs` has zero domain-specific branching — it takes a `RetentionConfig` and a record, returns a score. Domain logic lives entirely in the config table, not in the function.
- [x] Unit test: operator-authored personal record with `recency = 1.0` stays above floor indefinitely regardless of importance/retrieval inputs.
- [x] Unit test: system-scope raw record with importance 0.1 and no retrieval crosses below floor within ~48h.
- [x] Persona `mood_summary` decay (from avatar plan) is refactored to call this shared utility rather than keeping its own copy of the exponential.

The maintenance sweep runs at the start of consolidation, persists `Active -> Decayed` transitions in the canonical contract store, and default recall excludes decayed records without deleting their files.

---

## 2. Update and compress as first-class operations

**Update (correction/supersession):**

The operational boundary is `MnemosyneService::correct_memory(record_id, replacement_content, reason, consumer_ctx)`. This matches Vairë's existing string-typed informant boundary instead of introducing a parallel event enum. On write:

- The old record's state is *not* silently overwritten. It transitions to `state = Revoked` with a `revoked_by` pointer to the new record's id.
- The new record is written normally (`state = Active`) and carries a `supersedes` field in `extensions`.
- Recall, by default, follows the supersession chain and returns only the current record — but the chain is walkable for audit ("what did we used to believe, and when did that change").

This gives you the "operator changed banks" / "business assumption turned out wrong" case a real, queryable answer instead of two coexisting records with no signal for which is current.

**Compress:**

Implemented as a named governed operation in `service/governance.rs` rather than a side effect of promotion:

- `MnemosyneService::compress_episodic_batch(record_ids, summary, consumer_ctx)` — resolves a persisted batch of low-individual-significance, high-count episodic records (e.g. a week of routine system heartbeats, or routine personal-ops captures) and emits one summary record with `extensions.compressed_from = [ids]` and `extensions.compression_ratio`.
- Trigger condition: batch size ≥ 20 records in the same scope within a 7-day window, all individually below the importance threshold for standalone promotion.
- This is the mechanism that keeps retrieval fast as the store grows — without it, retrieval quality degrades because raw logs don't scale, which is the exact failure mode compression exists to prevent.

**Acceptance criteria:**
- [x] Correction chain is walkable in both directions (current -> history, and history -> current) via a single query, no manual joins.
- [x] Compression never runs on business or personal scope without the batch first passing the same importance floor check used for promotion — routine ≠ safe to compress blindly, only routine *and* individually low-significance is.
- [x] Compressed summary records carry enough `compressed_from` provenance that a revoke cascade (§4) can still reach the original raw records if needed.

---

## 3. Scope-policy pipeline (replaces the single boolean check)

**File:** `crates/spine/memory/arda-vaire/src/service/scope_policy.rs` (new)

Not a single `recall_scope_policy(bool)` gate. A mediating pipeline every read and write passes through, with four dispositions:

```rust
pub enum PolicyDisposition {
    Allow,
    Redact(Vec<String>),   // field paths stripped before the caller sees the record
    Quarantine,             // written/returned but flagged, excluded from promotion until reviewed
    Block,
}

pub fn evaluate(record: &MemoryRecord, consumer_ctx: &ConsumerContext) -> PolicyDisposition
```

**Why four states instead of two:** a boolean allow/deny either over-shares (business-scope consumer gets full personal-health record) or over-blocks (business-scope consumer gets nothing, even though a redacted version — "operator was unavailable" without the health detail — would be legitimately useful). Redact is the state that makes cross-domain summaries possible without leaking domain-restricted content.

**Concrete rules for this slice:**

| Consumer scope | Record scope | Disposition |
|---|---|---|
| personal | personal | Allow |
| business/system | personal | Redact — strip any field tagged `sensitivity=health` or `sensitivity=identity`, pass through only `evidence_class=confirmed` non-sensitive fields |
| any | business | Allow (no cross-domain restriction proposed for this slice — business is not privacy-sensitive the way personal is) |
| any | system | Allow, except raw pre-consolidation fault records with embedded credentials/tokens, which are `Block` at the store boundary, not just at recall — this should never be written un-redacted in the first place |
| unauthenticated/no consumer_ctx | personal | Block |

**Quarantine is reserved for provenance failures, not domain crossing** — see §5. Domain-crossing uses Allow/Redact/Block only.

**Where this plugs in:** every call path through `service/retrieval.rs` and every write path through `service/store.rs` calls `scope_policy::evaluate` before returning/committing. This is a chokepoint change, not a new subsystem — same pattern as a request-scoped middleware, applied once.

**Acceptance criteria:**
- [x] A business-scope consultation of Arandur's daily brief returns a redacted personal record (e.g. "operator had a scheduling conflict") without the underlying health-tagged field ever crossing the boundary — test asserts the field is absent from the returned struct, not just hidden in rendering.
- [x] Recall with no `consumer_ctx` on a personal-scope record returns `Block`, not an empty-but-technically-successful result — caller must be able to distinguish "no context provided" from "no matching records."
- [x] Policy table lives in one file (`scope_policy.rs`), not scattered across call sites — auditability requirement, not just style.

---

## 4. Revoke and cascade

Your promotion receipts already retain source IDs — this section defines what "forget the source" does to what was derived from it, which is currently undefined.

**File:** `service/governance.rs`

```rust
pub fn revoke(record_id: RecordId, cascade: CascadePolicy) -> RevokeReceipt

pub enum CascadePolicy {
    SourceOnly,           // mark this record revoked, leave derived records untouched
    MarkDerivedStale,      // derived records get a `derivation_stale: true` flag, not deleted
    RegenerateDerived,      // trigger re-derivation of anything downstream (expensive, use sparingly)
}
```

- Default for personal scope: `MarkDerivedStale`. A revoked personal record doesn't retroactively vanish from a business-improvement summary it fed, but the summary is flagged so a human or agent knows it may need re-derivation.
- Default for business scope: `SourceOnly` unless explicitly escalated — business history is generally meant to persist as an accurate record of what was believed at the time, per the existing "explicit surgical forget only" posture.
- `RegenerateDerived` exists for the rare real case (personal data revoked for a substantive reason, not just correction) but is never the default — it's expensive and should be an explicit operator action, not something a routine revoke silently triggers.
- Every revoke, regardless of cascade policy, produces a `RevokeReceipt` with the same hash-chain provenance discipline as promotion receipts — the revoke itself is an auditable event, not a silent mutation. This is the same origin-bound-authority property your promotion receipts already have; revoke should not be a hole in that guarantee.

**Acceptance criteria:**
- [x] Revoking a raw personal record that fed a promoted business-improvement summary leaves the summary retrievable but `derivation_stale: true` under `MarkDerivedStale`.
- [x] `RevokeReceipt` is hash-chained and the complete ledger is independently validated before closeout — a revoke can't be quietly un-revoked without invalidating the chain.
- [x] No cascade policy silently deletes a promoted record — deletion (as opposed to state transition) is out of scope for this slice entirely; decayed/revoked records stay physically present unless a separate, explicitly-invoked purge operation is run.

---

## 5. Provenance validation at write (poisoning defense, minimal version)

Full memory-poisoning defense is out of scope for this slice, but one cheap guard is worth adding now because it's a one-line hook into a system you already have:

- Every encode carries a `source` field (which subsystem/agent wrote it — CHARON routing, HERMES entry, personal-ops capture, etc.). At write time, `scope_policy::evaluate` checks that the declared `source` is consistent with the declared `memory_scope` (e.g. a `personal` record should not be originating from an external-facing agent entry point with no operator-authorization context attached).
- Records that fail this check are not blocked outright — they're written with `PolicyDisposition::Quarantine`, excluded from promotion eligibility until an explicit review clears them. This gives you a cheap trip-wire against the class of attack where a record's provenance and its claimed scope disagree, without building the full non-malleable-authority machinery right now.
- This is the one place quarantine is used in this slice — reserved for provenance mismatches, not for domain-crossing recall (that's Redact/Block per §3).

**Acceptance criteria:**
- [x] A record with `memory_scope = personal` and `source` = an agent entry point with no operator-context is written as `Quarantined`, not `Active`, and does not appear in default recall until cleared.
- [x] Quarantine clearing is an explicit operator-authorized service action, not automatic on the next consolidation pass.

---

## 6. Explicit non-goals for this doc

- No differential privacy, no encryption-at-rest changes — out of scope, revisit if arda-vaire ever serves more than one operator.
- No full non-malleable-authority/machine-checked-guarantee system — §5 is a minimal trip-wire, not that.
- No automatic physical purge/deletion — decay and revoke both stop at state transition in this slice.
- No cross-agent federated memory — this is all single-store, single-operator.
- No new database, no new sync job, no new IPC bridge — same constraint as the avatar plan.

---

## 7. Sequencing

1. §1 shared retention-scoring utility + per-domain config table (unblocks everything else, and lets the avatar plan's `persona.mood_summary` refactor onto it).
2. §3 scope-policy pipeline with Allow/Redact/Block (Quarantine deferred to step 5, since it needs §5's provenance check to have something to trigger on).
3. §2 update/correction path.
4. §2 compress as a named step.
5. §5 provenance validation + Quarantine disposition wired in.
6. §4 revoke/cascade — last, because it's the operation most likely to surface gaps in how §1–§5 tag and track derivation, so it benefits from those being stable first.

The sequence was implemented as bounded dependency slices in one worktree: shared retention first, then policy mediation, lifecycle operations, provenance quarantine, and revoke cascades. No new database, scheduler, sync job, or IPC bridge was introduced.

## 8. Closeout evidence (2026-08-05)

- `cargo test -p arda-vaire --all-features` — pass: 63 unit tests, 13 integration tests, and 1 recovery test; 1 operator-scale soak ignored by declaration.
- `cargo test -p arda-vaire --no-default-features` — pass: 61 unit tests, 13 integration tests, and 1 recovery test; 1 operator-scale soak ignored by declaration.
- `cargo test -p arda-core quarantined_state_has_a_stable_wire_value` — pass.
- `cargo check --workspace --all-targets` — pass, including every downstream consumer of the shared `MemoryState` contract.
- `cargo clippy -p arda-vaire --all-features --all-targets -- -D warnings` — pass.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-vaire --all-features --no-deps` — pass.
- Governance regression coverage includes correction-chain traversal and default-current recall, compression threshold/scope/provenance, consolidation-driven retention, redaction and missing-context distinction, ingestion quarantine and explicit clearance, receipt idempotence/hash validation, and stale marking for both semantic and persona projections.
