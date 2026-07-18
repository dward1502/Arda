---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  role: "governance_engine"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-governance
Governance engine for Arda agents: triad gates, resonance, love dynamics,
JouleWork profiling, readiness levels, and philosopher arbitration.
Owner: hades | Sigil: 🜏 REPAIR | Status: active

## Summary
`arda-governance` is the heaviest governance crate in the Arda spine.
It implements the actual scoring surfaces behind autonomous decision
quality: triad validation, resonance/joule/love metrics, game-theory
agent selection, readiness projections, Bacon-Lite evidence logging,
audio/vision/solar environmental governance signals, and a deterministic
philosopher arbitration layer. It depends on `arda-core` and is not yet
wired into `arda-engine` or `apps`.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/governance/arda-governance`
- Configs: `../../config/governance/chains.toml`, `../../config/governance/philosophers.toml`
- Tests: `tests/alignment_stack.rs`, `tests/philosopher_profiles.rs`

## Verification status
- `cargo check -p arda-governance`: OK
- `cargo test -p arda-governance`: 44/45 passing
- Failing: `repository_default_chain_config_matches_g3_contract`
  - Cause: path mismatch `../config/governance/philosophers.toml` vs
    `config/governance/philosophers.toml` in included repo config.
  - Fix: align `chains.toml` `profile_source` with the test include path.
- No consumer imports detected in `crates/engine` or `apps`.

## Agentic-OS abstractions
- **Triad Gate**: deterministic 3-lens validation
  - lenses: aurelius, bacon, sun_tzu
  - configurable chain via TOML with schema validation and versioning
  - review modes: HeuristicLocal, IndependentAgent, HumanReviewed, ConsensusReceipted
  - outcomes: Pass, Conditional, Fail
  - default weights: aurelius 0.60, bacon 0.50, sun_tzu 0.50
- **Governance chain config contract**
  - schema version `arda.governance.chains.v1`
  - validates required passes, required fields, lens thresholds
  - blocks `autonomous_blocking_enabled` until later phase
- **Resonance scoring**
  - ECST + phi harmonic + governance chain linkage
  - 0-100 resonance score, triad purity tracking
- **Love Equation / Dynamics**
  - static proxy: `impact * reach / (energy * time)`
  - dynamic: `dE/dt = beta * (C - D) * E`
  - trends: Growing, Stable, Decaying
- **JouleWork profiling**
  - variance, honesty_ratio, measurement source/confidence
  - efficiency when variance <= 0.25
- **Game-Theory agent selection**
  - `AgentScore` combines resonance, love, joule honesty, triad pass rate
  - `GameTheorySelectionResult` exposes policy label and confidence
  - current labels: capability_weighted_heuristic / policy_backed_fallback
- **Readiness taxonomy**
  - seven levels from DocumentedOnly to AutonomyReadyForScope
  - evidence-backed with independent review receipts
- **Bacon-Lite compliance logging**
  - appends JSONL machine log + markdown human log
  - env-overridable paths
- **Governance signals**
  - Audio, Vision, Solar geomagnetic environmental signals
- **Philosopher arbitration**
  - alignment signals with Proceed / Revise / Hold / Reject
- **Philosopher profile lifecycle**
  - TOML-loaded schema, maturity gating, draft-only bootstrap in G2
- **Corpus loader**
  - regex patterns + weighted veto classes from philosopher dirs

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Public exports for all subsystems |
| `triad.rs` | Triad gate, governance chains, TOML load/validate |
| `triad_philosopher.rs` | Alignment arbitration |
| `resonance.rs` | Resonance + ECST/phiharmonic/governance-chain scoring |
| `love_dynamics.rs` | Differential love dynamics |
| `love_equation.rs` | Static love equation proxy |
| `joulework.rs` | JouleWork profile surface |
| `game_theory.rs` | Capability-weighted agent selection |
| `bacon_lite.rs` | Evidence gate + machine/human logging |
| `readiness.rs` | Seven-level readiness projection |
| `vision.rs` | Visual convergence governance signal |
| `audio.rs` | Audio environmental governance signal |
| `solar.rs` | NOAA Kp/Dst geomagnetic multiplier |
| `philosopher_profiles.rs` | Profile schema validation + maturity gating |
| `philosophers/socrates.rs` | Placeholder specialist philosopher |
| `corpus_loader.rs` | Regex/weight corpus quick-check |

## Consumer wiring
- No direct `arda-engine` / `apps` imports
- Indirectly reachable via `arda-core` loop engine / triad consultant
- Logical next wiring point: expose triad/resonance/love/joule scorings
  through engine or CLI for telemetry

## Ideas for improvement
1. Fix repo path contract: align `chains.toml` `profile_source` with
   repo default `config/governance/philosophers.toml`.
2. Replace path-string coupling with base-dir injection so tests don't
   depend on repo layout.
3. Make `autonomous_blocking_enabled` a runtime policy toggle instead
   of repeated compile-time-ish validation.
4. Convert Bacon-Lite log transport to a shared ledger trait in `arda-core`.
5. Add `GameTheoryConfidenceBand` enum: High/Medium/Low/NoData.
6. Add evidence-grade penalty in triads instead of silently lenient
   heuristic-local scoring when no LLM evidence is attached.
7. Rename `love_equation.rs` to `love_equation_proxy.rs` to make intent
   explicit and reduce confusion with `love_dynamics.rs`.
8. Parallelize NOAA/vision/audio fetches in HUD refresh with
   `try_run_bounded_async` from `arda-core/background.rs`.
9. Add a `GovernanceSignal` enum for composite environmental coherence.
10. Wire one governance signal into engine/HADES for live telemetry.
