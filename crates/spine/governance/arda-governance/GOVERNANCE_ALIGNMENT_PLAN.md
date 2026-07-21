---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_plan"
  owner: "ORACLE"
  status: "implemented"
  last_reviewed: "2026-06-06"
---

> 🜏 Soterion: 📜 governance_plan | owner: ORACLE | status: implemented | reviewed: 2026-06-06

# Arda Governance Alignment Plan

Scope: `crates/arda-governance/`

Status: Love Dynamics, Triad Philosopher arbitration, resonance metadata, and Prometheus autopilot triad-quorum surface wiring are implemented as of 2026-05-21.

## Intent

Move Arda governance from a single usefulness-efficiency proxy toward a layered alignment stack:

1. Love Dynamics: canonical Brian Roemmele framing `dE/dt = beta * (C - D) * E`, tracking whether empathy/cooperative alignment is growing or decaying over time.
2. JouleWork: resource honesty, proportionality, and cost discipline. High cost is not automatically bad; hidden waste and unjustified cost are the risk.
3. Nonconformist Bee: independence and anti-sycophancy pressure. This keeps cooperation from collapsing into obedience.
4. Empirical Distrust: evidence discipline, provenance, verification, and falsifiability. This keeps confidence grounded.
5. Triad Philosopher: reflective arbitration across the signals. This layer interprets conflicts, flags rationalization, and explains whether the system should proceed, hold, revise, or reject.
6. Resonance: operational composition of timing, status, triad, JouleWork, phi, and optional audio/vision context.

## Current state observed

- `love_equation.rs` still preserves the existing `impact * reach / (energy * time)` public compatibility score.
- `love_dynamics.rs` implements Brian Roemmele's dynamic alignment form `dE/dt = beta * (C - D) * E` with growing/stable/decaying trend classification.
- `joulework.rs` tracks estimate/actual variance, honesty ratio, and an efficiency boolean.
- `triad.rs` has Aurelius/Bacon/Sun Tzu gates and a contract-level `PhilosopherVerdict` mapper.
- `triad_philosopher.rs` now provides a separate deterministic arbitration layer over empirical grounding, independence, sycophancy risk, Love Dynamics trend, JouleWork honesty/efficiency, and defection pressure.
- `resonance.rs` keeps its existing score weighting intact while attaching Love Dynamics metadata and an optional Triad Philosopher verdict for callers that want richer governance evidence.
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

## Next recommended slices

1. Replace or deprecate `love_equation_score` as a proxy in favor of a compatibility wrapper around Love Dynamics.
2. Add structured evidence/cooperation/defection extraction from richer `Task` metadata or result payload schemas instead of relying on current heuristic key/text signals.
3. Extend CLI/operator report surfaces to display the compact philosopher evidence strings where governance decisions are rendered.
4. Add Nonconformist Bee and Empirical Distrust as first-class modules instead of embedded signal fields.
5. Decide whether resonance values should eventually be reweighted by philosopher verdicts; this slice deliberately preserved existing numeric scoring.
