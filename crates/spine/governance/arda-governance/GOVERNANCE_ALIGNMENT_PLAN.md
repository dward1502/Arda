---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_plan"
  owner: "ORACLE"
  status: "implemented"
  last_reviewed: "2026-07-25"
---

> 🜏 Soterion: 📜 governance_plan | owner: ORACLE | status: implemented | reviewed: 2026-07-25

# Arda Governance Alignment Plan

Scope: `crates/spine/governance/arda-governance/`

Execution status: **superseded by [`FIRST_CLASS_CHECKLIST.md`](FIRST_CLASS_CHECKLIST.md)**.
This file remains design/rationale evidence. Every alignment slice is represented in the
checklist's source-coverage map; do not add competing execution status here.

Status: Phase 7 complete as of 2026-07-25: Love Dynamics, explicit compatibility proxy,
independent Nonconformist Bee and Empirical Distrust assessments, receipted Triad Philosopher
arbitration, separate resonance metadata, and operator disclosure are implemented.

## Intent

Move Arda governance from a single usefulness-efficiency proxy toward a layered alignment stack:

1. Love Dynamics: canonical Brian Roemmele framing `dE/dt = beta * (C - D) * E`, tracking whether empathy/cooperative alignment is growing or decaying over time.
2. JouleWork: resource honesty, proportionality, and cost discipline. High cost is not automatically bad; hidden waste and unjustified cost are the risk.
3. Nonconformist Bee: independence and anti-sycophancy pressure. This keeps cooperation from collapsing into obedience.
4. Empirical Distrust: evidence discipline, provenance, verification, and falsifiability. This keeps confidence grounded.
5. Triad Philosopher: reflective arbitration across the signals. This layer interprets conflicts, flags rationalization, and explains whether the system should proceed, hold, revise, or reject.
6. Resonance: operational composition of timing, status, triad, JouleWork, phi, and optional audio/vision context.

## Current state observed

- `love_equation.rs` preserves `impact * reach / (energy * time)` only through the explicit
  `love_dynamics_compatibility_proxy`; `love_equation_score` is deprecated.
- `love_dynamics.rs` implements Brian Roemmele's dynamic alignment form `dE/dt = beta * (C - D) * E` with growing/stable/decaying trend classification.
- `joulework.rs` tracks estimate/actual variance, honesty ratio, and an efficiency boolean.
- `triad.rs` has Aurelius/Bacon/Sun Tzu gates and a contract-level `PhilosopherVerdict` mapper.
- `nonconformist_bee.rs` and `empirical_distrust.rs` independently assess anti-sycophancy and
  evidence grounding, and feed `triad_philosopher.rs` arbitration.
- `triad_philosopher.rs` carries a lifecycle receipt disclosing source, revision, maturity,
  authority, review mode/authority, generated-artifact identity, and promotion criteria.
- `resonance.rs` declares `separate_decision_metadata`; golden coverage proves philosopher
  verdicts do not alter its existing numeric weights.
- `arda-prometheus/src/autopilot/oracle_gate.rs` now derives a deterministic Triad Philosopher verdict from ORACLE triad quorum evidence.
- `arda-prometheus/src/autopilot/governance_policy.rs` treats a non-proceed Triad Philosopher verdict as a triad-quorum hold, surfaces the verdict in governance reasons, and records compact evidence strings for runtime reports.
- `OPTIMIZATION_PLAN.md` already flags the old Love Equation scaling problem as B3.

## Implementation slice

This slice adds deterministic primitives and resonance metadata without replacing existing public scoring surfaces.

### Step 1: Add Love Dynamics — complete

`src/love_dynamics.rs` provides:

- `LoveDynamicsInput`
- `LoveDynamicsScore`
- `LoveDynamicsTrend`
- `evaluate_love_dynamics(input)`

Behavior:

- Clamps empathy/cooperation/defection to `[0.0, 1.0]`.
- Clamps beta and delta time to non-negative values.
- Computes `delta_empathy = beta * (cooperation - defection) * empathy * delta_time`.
- Computes `projected_empathy = empathy + delta_empathy`, clamped to `[0.0, 1.0]`.
- Classifies trend as `Growing`, `Stable`, or `Decaying`.

### Step 2: Add Triad Philosopher arbitration — complete

`src/triad_philosopher.rs` provides:

- `AlignmentSignals`
- `PhilosopherAction`
- `TriadPhilosopherVerdict`
- `interpret_alignment(signals)`
- `derive_alignment_signals(task, love, joule, components)`

Behavior:

- Holds when empirical grounding is weak and either sycophancy risk is high or Love Dynamics is decaying.
- Revises when sycophancy/obedience risk is high despite otherwise workable cooperation.
- Allows expensive work when Love Dynamics is growing and evidence is strong, even if JouleWork efficiency is poor.
- Explains the reason in a stable, operator-readable string.

### Step 3: Wire exports and resonance metadata — complete

- `lib.rs` exports the Love Dynamics and Triad Philosopher primitives.
- `resonance.rs` derives Love Dynamics and JouleWork context from the current `Task`, attaches the Triad Philosopher verdict to `ResonanceScore`, and mirrors the action/alignment score into `ResonanceComponents`.
- Existing score weights and `calculate_resonance_basic(task)` call shape remain unchanged.

### Step 4: Tests — complete

`tests/alignment_stack.rs` proves:

- Love Dynamics grows when `C > D`.
- Love Dynamics decays when `D > C`.
- Triad Philosopher blocks low-evidence sycophantic compliance even with good JouleWork.
- Triad Philosopher permits high-cost work when evidence is strong and Love Dynamics is growing.
- Alignment signals are derived from `Task`, JouleWork, Love Dynamics, and resonance metadata.
- Backward-compatible resonance callers receive the optional philosopher verdict without changing call signatures.

### Step 5: Wire Prometheus autopilot governance surface — complete

`arda-prometheus/src/autopilot/oracle_gate.rs` and `governance_policy.rs` now propagate the philosopher verdict into the operational triad quorum path:

- `TriadQuorumEvidence` carries optional `triad_philosopher` evidence while preserving backward-compatible deserialization with `#[serde(default)]`.
- ORACLE gate evidence maps triad gate scores/resonance into deterministic `AlignmentSignals` and stores `interpret_alignment(...)` output.
- Governance policy still requires ORACLE quorum/pass thresholds, and additionally blocks delegation when the philosopher verdict is `hold`, `revise`, or `reject`.
- Governance decision evidence includes `triad_philosopher:<action>:<score>` when present.
- Runtime behavior remains deterministic and does not introduce LLM calls or new external dependencies.

## Non-goals for this slice

- Do not remove or rename `love_equation_score` yet.
- Do not reweight `game_theory.rs` yet.
- Do not reweight resonance or game-theory scores from philosopher verdicts yet.
- Do not add LLM calls; this remains deterministic.

## Verification evidence

- Focused integration test: `source scripts/runtime_build_env.sh && cargo test -p arda-governance --test alignment_stack` passed with 6 tests.
- Focused Prometheus tests:
  - `source scripts/runtime_build_env.sh && cargo test -p arda-prometheus triad_class_blocks_delegation_when_philosopher_does_not_proceed`
  - `source scripts/runtime_build_env.sh && cargo test -p arda-prometheus quorum_evidence_surfaces_triad_philosopher_verdict`
- Source index updated: `src/INDEX.md` lists `love_dynamics.rs` and `triad_philosopher.rs`; `src/README.md` summarizes the active source surfaces.

## Phase 7 closeout

- The Socrates/corpus-loader drafts remain deliberately retired: no source files or registered
  module paths exist, and source indexes record why they were removed.
- `tests/phase7_philosopher_expansion.rs` covers the compatibility boundary, fixed resonance
  weighting, independent assessments, lifecycle receipts, and conflicting arbitration cases.
- Future changes may replace heuristic signal extraction with richer structured task metadata,
  but must preserve the disclosed evidence grade and receipt boundaries.
