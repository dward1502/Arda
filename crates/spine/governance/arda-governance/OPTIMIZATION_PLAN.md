---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Governance Optimization & Feature Plan

Drafted 2026-05-07 alongside the Charon plan. Same shape: P0 = do next, P1 = soon, P2 = nice to have. Each item names files to touch and a verification signal.

Scope: `crates/spine/governance/arda-governance/` (historical size/module counts in this plan are audit snapshots, not the current inventory). Pure deterministic decision logic — no LLM calls; Bacon-Lite is the principal evidence writer.

---

## A. Correctness / wiring gaps

### A1. `resonance::triad_purity` is a hardcoded 0.7. **P0**
`resonance.rs:152-154` returns the placeholder default for every call. Resonance is supposed to fold the actual triad pass rate but doesn't — so 20% of the resonance score is a constant. Wire it to either (a) the live `triad_validate(task)` result, or (b) a passed-in `TriadResult` so callers that already validated don't double-validate.

- Touch: `resonance.rs::calculate_resonance`, signature change for `triad_purity(task: &Task)` or `triad_purity(triad: Option<&TriadResult>)`.
- Signal: a task with `Triad.passed=false` produces a measurably lower resonance than one with `passed=true`. Add a unit test that flips the verdict and asserts the delta.

### A2. `game_theory::select_agent` filters to `name.starts_with("athena")`. **P0**
`game_theory.rs:42` hardcodes the candidate filter to athena. Every other agent is invisible to the selector. Either pass `task_type` through to a real capability filter or keep an explicit `eligible_agents: &[&str]` parameter on `select_agent`.

- Touch: `game_theory.rs::select_agent`.
- Signal: a `GameTheory` populated with athena/hermes/hades scores returns at least one non-athena pick across N=100 calls when called for a non-athena-eligible task type.

### A3. `solar::solar_multiplier` is computed but never consumed. **P1**
`solar.rs::solar_multiplier` produces a 0.5–1.5 scalar based on NOAA Kp/Dst, but no other module imports it. Either fold it into `resonance::calculate_resonance` (multiplicative final term) or remove the module.

- Touch: `resonance.rs` (consume) or `lib.rs` (drop the re-export and crate).
- Signal: setting `kp_index >= 5.0` in a fixture shifts resonance score downward by the expected fraction.

### A4. `bacon_lite` writes synchronously inside hot paths. **P1**
`record_bacon_lite` is called on every Charon route (success and failure), every Hades action, every Athena ingest decision. Each call opens two files (machine.jsonl + human.md), writes, closes. No fsync coalescing. Charon already moved its own state writes to an async writer (B3 done); bacon-lite should do the same — either with its own writer task or by handing the event to the caller's writer.

- Touch: `bacon_lite.rs::record_bacon_lite`, `append_machine_log`, `append_human_log`.
- Signal: route p99 latency stops correlating with bacon-lite log size; under burst load, no fsync stalls in iostat.

---

## B. Scoring quality

### B1. Triad scorers are keyword-substring-based. **P1**
`score_aurelius`, `score_bacon`, `score_sun_tzu` all do `desc.contains("because")` / `desc.contains("urgent")` etc. Cheap and deterministic, but trivially gameable and brittle to translation/rephrasing. Two paths:

- **B1a (cheap, P1):** add a structured-context bypass — if `task.context` carries explicit fields (`evidence_url`, `urgency_justified: bool`, `is_action: bool`), trust those over substring heuristics. Keeps the heuristic floor for free-form tasks but stops penalizing well-structured callers.
- **B1b (expensive, P2):** swap heuristic scoring for an LLM-based scorer behind a feature flag. Async signature, but Charon already runs async everywhere upstream of triad calls. Risk: makes the gate non-deterministic; would need careful caching by task hash.

- Touch: `triad.rs::score_*`, `Task::context` shape.
- Signal: a structured task with `evidence_url` set scores ≥0.7 on Bacon even when the description contains no `because`/`source`/`evidence`.

### B2. `phi_harmonic` ratios fail soft when fields are zero. **P1**
`resonance.rs:48-78` returns 50.0 ("neutral") when planning_duration / actual_joule_cost / answered_clarifications are zero. That's most short-running tasks. Result: phi_harmonic is ~50 by default and the 10% weight doesn't differentiate anything. Either zero-weight phi_harmonic when no signal exists (let the other components carry full weight) or replace the neutrals with sentinel values that don't bias the composite.

- Touch: `resonance.rs::phi_harmonic`, `calculate_resonance`.
- Signal: a task with no timing/joule/clarification data produces the same score as a task with neutral=50 today, but a task with real signal moves measurably more.

### B3. `love_equation` `clamp(raw * 100.0, 0.0, 1.0)` produces near-zero scores in practice. **P1**
`love_equation.rs:44-45`: `raw = (impact·reach) / (energy·time)` with floors `energy ≥ 1.0` and `time ≥ 1.0`. For a typical complete task: `(0.95 · 0.65) / (1.0 · 30.0) ≈ 0.02`, then `* 100 = 2.05`, clamped to `1.0`. So almost any non-instant task scores at the ceiling. The `* 100` is suspicious — looks like it was meant to be the % cap, not a multiplier. Audit and fix.

- Touch: `love_equation.rs::love_equation_score`.
- Signal: scores spread across the [0, 1] range for a synthetic task population covering 1s–5min execution times. Today they're bimodal at 0 and 1.

### B4. Game-theory weights treat raw ranges inconsistently. **P2**
`game_theory.rs:53-57`: `avg_resonance` is 0–100, `avg_love_equation` is 0–1 (then ×100), `joule_honesty` is 0–1 (×100), `triad_pass_rate` is 0–1 (×100). The ×100s "normalize" but obscure intent. Move all four to a common 0–1 scale at score time; do the weighting in one place.

- Touch: `game_theory.rs::select_agent`, `game_theory_score`.

---

## C. Observability

### C1. No metrics surface. **P0**
Every other major crate emits Prometheus metrics; governance has none. Add the smallest useful set:

- `governance_triad_validations_total{verdict}` — verdict ∈ pass/conditional/fail
- `governance_triad_gate_outcomes_total{gate, outcome}` — gate ∈ aurelius/bacon/sun_tzu, outcome ∈ pass/conditional/fail
- `governance_bacon_lite_total{verdict}` — pass/fail
- `governance_resonance_score` — histogram, buckets 0/25/50/75/90/100
- `governance_love_equation_score` — histogram, buckets 0/0.25/0.5/0.75/1.0
- `governance_joulework_honesty_ratio` — histogram, 0/0.5/0.75/0.9/1.0

Since governance is a library not a service, the metrics need to live somewhere callers can scrape. Two options:
1. **In-process counter store** (similar to `charon::metrics`): library-owned, exposed via a `governance_metrics()` accessor that callers can render into their own /metrics.
2. **Caller-driven**: library returns counters in the result types; callers (Charon, Hades, Athena) record into their own metric stores. Less coupling but more wiring.

- Touch: new `metrics.rs`, `triad.rs`, `bacon_lite.rs`, `resonance.rs` to instrument.
- Signal: Charon's `/metrics` (or a new gov-specific endpoint) shows triad pass rates broken out by gate after the change.

### C2. No bacon-lite ledger reader / aggregator. **P1**
`record_bacon_lite` writes to `data/governance/bacon_lite.jsonl` but nothing reads it back for aggregate analysis. Add a `bacon_lite_summary(window: Duration) -> SummaryStats` reader that reports per-(crate_name, action) pass rate, mean confidence, gate-fail distribution. This is the natural input to a "governance dashboard" CLI command and to A1/A4 follow-ups.

- Touch: new `bacon_lite::summary` fn, `arda-cli` reader command.

### C3. Triad results are not versioned. **P2**
If we change a scorer threshold or add a gate, old ledger entries become incomparable to new ones. Add a `scorer_version: u32` field to `TriadResult` and bump it on every threshold/heuristic change so historical analysis can filter by version.

---

## D. Resilience

### D1. `solar::fetch_solar_geomag` builds a fresh `reqwest::Client` per call. **P2**
Same lesson as Charon B4 — pool the client. But solar isn't on a hot path (and per A3 isn't currently consumed at all), so this is genuinely P2.

### D2. NOAA endpoint hardcodes / no fallback. **P2**
`solar.rs:21,33` hardcodes `https://services.swpc.noaa.gov/products/...`. If NOAA reshuffles those URLs the function returns Err and any caller relying on solar data falls back to neutral. Move URLs to env-var overrides; consider a 1-minute cached value so transient NOAA outages don't poison the system.

---

## E. New features

### E1. Async-first scorer trait. **P2**
Once B1b lands (LLM-based scorers), the existing sync `triad_validate` becomes a wrapper around an async trait. Define the trait now even if the only impl is sync, so callers that already await elsewhere don't add a sync barrier.

### E2. Per-realm gate weight overrides. **P2**
Hades cares about Sun Tzu (strategy) more than Aurelius (logic-of-description) — a sweep action is a deploy decision. Athena cares about Bacon (evidence) more than Sun Tzu — ingestion is empirics-first. Today every caller uses the default 2-of-3 lenient mode. Add a `TriadConfig::weighted_pass_required(weights: [f64; 3])` mode where a weighted sum of gate scores must clear a threshold. Callers opt in.

### E3. Triad veto-reason structure. **P2**
`veto_reason: Option<String>` is a `|`-joined string today. Make it a typed enum: `Veto::FailedGates(Vec<GateName>)` / `Veto::InsufficientPasses { passed, required }`. Callers can match instead of substring.

---

## Suggested execution order

1. **Wiring fixes first:** A1 (triad_purity → real triad), A2 (game theory candidate filter), C1 (metrics).
2. **Hot-path:** A4 (async bacon-lite writer), then audit B3 (love_equation scaling — likely one-line bug fix with big behavioral payoff).
3. **Scoring quality:** B2 phi_harmonic neutral handling, B1a structured-context bypass.
4. **Cosmetic / future:** A3 solar wiring (or removal), C2 ledger summary, then E2/E3.

The P0 trio (A1, A2, C1) is roughly one focused session.

### Status snapshot

| Item | Status |
| ---- | ------ |
| Everything | open |
