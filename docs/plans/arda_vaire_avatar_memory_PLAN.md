# PLAN: arda_vaire_avatar_memory

**Scope:** Items 1+2 only — personality identity layer inside arda-vaire, and the HUD persona state consumer.
**Explicitly out of scope:** personal-ops completion, business improvement projection, curiosity query loop. Those remain queued behind this slice.

**Contract impact:** None. Everything below lives inside `MemoryRecord.extensions` under the `persona.*` namespace. No `agent-state-contract.md` version bump required for this slice. The v0.2 bump is deferred to the CuriosityQuery slice.

---

## 0. Naming and ownership convention (do this first, before any code)

Extension keys are namespaced by owning subsystem, dot-separated, lowercase:

```
persona.traits            -> PersonaTrait[]
persona.mood              -> MoodSample[] (rolling window, not full history)
persona.mood_summary      -> derived scalar/vector, cached, recomputed on read or on consolidate
persona.value_evidence    -> ValueEvidence[]
persona.schema_version    -> integer, starts at 1
```

Rule going forward: any future subsystem adding to `extensions` claims a top-level namespace key here in this file before writing code. `persona.*` is owned exclusively by this slice — curiosity, business-improvement, and personal-ops projections get their own namespaces (`curiosity.*`, `business.*`, `personal_ops.*`) when their turn comes. No shared/ambiguous keys.

`persona.schema_version` exists so the eventual v0.2 contract bump has a documented migration point instead of needing to reverse-engineer what shape was written when.

---

## 1. Data shapes (Rust, arda-vaire)

**File:** `crates/arda-vaire/src/persona/types.rs` (new module)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PersonaTrait {
    pub id: String,                 // stable slug, e.g. "decisive", "dry-humor"
    pub label: String,              // display string
    pub evidence_count: u32,        // number of contributing episodic records
    pub confidence: f32,            // 0.0-1.0, derived — see §3 promotion rule
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_reinforced_by: Option<String>, // MemoryRecord id
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoodSample {
    pub timestamp: DateTime<Utc>,
    pub valence: f32,               // -1.0..1.0
    pub source_record: String,      // MemoryRecord id this was derived from
    pub outcome_class: OutcomeClass, // Success | Warning | Error | Deploying
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoodSummary {
    pub as_of: DateTime<Utc>,
    pub weighted_valence: f32,      // decay-weighted average, see §4
    pub sample_count: u32,
    pub window_hours: u32,          // window used for this computation
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ValueEvidence {
    pub value_id: String,           // e.g. "sovereignty", "directness"
    pub evidence_count: u32,
    pub source_records: Vec<String>,
}
```

These are the typed views written into `MemoryRecord.extensions["persona.traits"]` etc. as serialized JSON — `extensions` stays `HashMap<String, serde_json::Value>` per the existing contract; this module just owns serialize/deserialize and validation for the `persona.*` keys.

---

## 2. Identity derivation

**File:** `crates/arda-vaire/src/service/persona_derive.rs` (new)

Function: `derive_identity_summary(actor: &str, since: Option<DateTime<Utc>>) -> PersonaProjection`

- Reads eligible episodic `MemoryRecord`s where `record.actor == "arandur"` (tag-filtered, reuses existing `retrieval.rs` query path — no new index).
- Only records with `state == Active` or `state == Promoted` are eligible. `Decayed`/`Revoked` records are excluded from derivation but not deleted (audit trail stays intact).
- Emits a new synthesized `MemoryRecord` of kind `semantic`, tagged `derivation=persona_identity`, whose `extensions` carries the `persona.*` payload.
- This is called from the existing `consolidate` job — not a new scheduler, not a new cron. Hook into the point in `service/promotion.rs` (or wherever consolidate currently runs) where consolidation already iterates actor-tagged records.

**Acceptance criteria for this section:**
- [ ] `derive_identity_summary` runs as part of existing consolidate pass with zero new background jobs.
- [ ] Running it twice on the same input set is idempotent (same trait confidence output, not append-only duplication).
- [ ] Decayed/revoked records provably excluded (test with a revoked record that would otherwise flip a trait).

---

## 3. Trait promotion rule (the thing I flagged as vague)

A trait is **not** created or reinforced by a single interaction. Concrete rule:

- A candidate trait requires **≥ 3 independent evidence records** within a rolling **30-day window** before it is written to `persona.traits` at all.
- `confidence` = `min(1.0, evidence_count / 10.0)`, recomputed each derivation pass. This means a trait asymptotically approaches confidence 1.0 around 10 corroborating records and never spikes from one event.
- A trait not reinforced for **60 days** is not deleted but flagged `stale: true` in the projection output (HUD should render stale traits visually de-emphasized, not hide them — audit trail stays visible).
- Single-event signals (a strong reaction, an unusual choice) land in `persona.mood` (see §4), never directly in `persona.traits`. Mood and trait are structurally different buckets precisely so one bad interaction can't move an identity claim.

**Acceptance criteria:**
- [ ] Unit test: single evidence record does not appear in `persona.traits` output.
- [ ] Unit test: 3 records within window does appear, confidence = 0.3.
- [ ] Unit test: trait untouched for 61 days gets `stale: true` on next derivation pass, is not removed from the array.

---

## 4. Mood decay function

`persona.mood_summary.weighted_valence` = exponential recency weighting, not a simple average:

```
weight(sample) = exp(-λ * age_hours)
λ = ln(2) / 24   // half-life = 24 hours
weighted_valence = Σ(valence_i * weight_i) / Σ(weight_i)
```

- Window: last 200 mood samples or 14 days, whichever is smaller (bounded read, no unbounded scan).
- Recomputed on-read in `persona_derive.rs`, cached into `persona.mood_summary` at consolidation time so the HUD isn't recomputing decay math on every render.

**Acceptance criteria:**
- [ ] A mood sample from 5 days ago has materially less influence than one from 5 hours ago (test asserts weight ratio ≈ `exp(-λ*120)`).
- [ ] Empty mood window returns `None`/neutral default, not divide-by-zero.

---

## 5. Obsidian projection

**File:** existing `sync_obsidian` pathway (no new sync job) — add a template for persona.

- Output: `human/personality/arandur/<date>.md`
- Content: current traits (with confidence + stale flag), current mood summary, and a short "recent evidence" list (last 5 source records, linked by id).
- Regenerated on each consolidation pass alongside existing Obsidian projections — same write cadence, same job.

**Acceptance criteria:**
- [ ] File is regenerated (not appended) each consolidation cycle.
- [ ] Stale traits render but visually marked in the Markdown (e.g. `~~trait~~` or a `(stale)` suffix).

---

## 6. HUD consumer

**File:** `apps/arda-hud/src/lib/statefulPersona.ts` (renamed/evolved from `avatarPersona.ts`)

- Replace static `frankyrache | rache | bartmoss` selection with a read of the latest `persona.*` projection (via whatever IPC/query path the HUD already uses to read arda-vaire — reuse, don't add a new bridge).
- Brand-voice templates don't disappear — they become the *rendering skin* applied on top of derived trait/mood state, not the source of truth for what the state is.
- New minimal UI: a "Personality" subpanel under `ArandurApprovalWorkstation.tsx` showing current traits (confidence as a bar or dot-fill, stale traits de-emphasized) and current mood as a single indicator (not a full history chart — that's a v2 concern).

**Acceptance criteria:**
- [ ] `statefulPersona.ts` has zero hardcoded persona state; all trait/mood values trace to a `persona.*` read.
- [ ] Subpanel renders with zero data gracefully (new install, no evidence yet) — shows an empty/neutral state, not an error.
- [ ] No new sync pipeline, no new store, no new IPC channel — confirmed by diff review before merge.

---

## 7. Explicit non-goals for this slice

- No CuriosityQuery type, no contract bump.
- No personal-ops wiring — Arandur's daily context source stays out of scope until that phase completes.
- No business-improvement projection.
- No full mood *history* chart in HUD — single current-state indicator only.
- No second memory store, no second avatar sync pipeline, no static persona files beyond the existing brand-voice skins (unchanged in role, just no longer the source of truth).

---

## 8. Sequencing

1. §0 namespace convention committed to this file (already done above) — treat as locked before writing code.
2. §1 types module.
3. §2 derivation + hook into existing consolidate pass.
4. §3 + §4 promotion/decay rules with unit tests — these gate everything downstream, don't skip to HUD before these pass.
5. §5 Obsidian template.
6. §6 HUD consumer.

Each step should be a separate commit/PR so the promotion-rule tests (§3/§4) are reviewable in isolation from the HUD rendering change.
