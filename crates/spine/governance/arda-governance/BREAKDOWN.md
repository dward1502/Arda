---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  role: "governance_engine"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
---

# arda-governance
Governance engine for Arda agents: triad gates, resonance, love dynamics,
JouleWork profiling, readiness levels, and philosopher arbitration.
Owner: hades | Sigil: 🜏 REPAIR | Status: active

Execution status: **superseded by [`FIRST_CLASS_CHECKLIST.md`](FIRST_CLASS_CHECKLIST.md)**.
This breakdown remains active architecture/audit evidence. Its improvement items are mapped
to canonical phases in the checklist's deduplication and source-coverage table; do not track
new execution status here.

## Summary
`arda-governance` is the central governance crate in the Arda spine.
It implements the actual scoring surfaces behind autonomous decision
quality: triad validation, resonance/joule/love metrics, game-theory
agent selection, readiness projections, Bacon-Lite evidence logging,
audio/vision/solar environmental governance signals, and a deterministic
philosopher arbitration layer. It depends on `arda-core`; production consumers are
enumerated and verified conservatively in `FIRST_CLASS_CHECKLIST.md` and `STATUS.md`.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/governance/arda-governance`
- Configs: `../../../../config/governance/chains.toml`, `../../../../config/governance/philosophers.toml`
- Tests: `tests/alignment_stack.rs`, `tests/philosopher_profiles.rs`

## Fix list (applied 2026-07-18)
- `triad.rs:676`: `include_str!("../../../config/governance/chains.toml")` → `../../../../config/governance/chains.toml`
- Normalized all in-repo `"config/governance/philosophers.toml"` status metadata strings in Rust sources, tests, and `crates/spine/config/governance/chains.toml` to the canonical repo-root path. Removed stale `../config/...` relative path from spine-local chain config.

## Verification status

Historical failures in this audit snapshot are closed. Current release-gate evidence and
workspace-owned blockers are recorded in `STATUS.md`; `FIRST_CLASS_CHECKLIST.md` is the
only completion tracker.

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
  - legacy autonomy flags are non-authoritative; scoped blocking is decided only by
    `RuntimeBlockingAuthority`
- **Resonance scoring**
  - ECST + phi harmonic + governance chain linkage
  - 0-100 resonance score, triad purity tracking
- **Love Equation / Dynamics**
  - explicit compatibility proxy: `impact * reach / (energy * time)` via
    `love_dynamics_compatibility_proxy`; old `love_equation_score` name is deprecated
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
  - Nonconformist Bee and Empirical Distrust are independent advisory modules
  - resonance keeps verdicts as separate decision metadata, not hidden score weights
- **Philosopher profile lifecycle**
  - TOML-loaded source/revision, generated-artifact boundary, review authority,
    promotion criteria, maturity gating, and immutable-by-value receipts

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Public exports for all subsystems |
| `triad.rs` | Triad gate, governance chains, TOML load/validate |
| `triad_philosopher.rs` | Alignment arbitration |
| `resonance.rs` | Resonance + ECST/phiharmonic/governance-chain scoring |
| `love_dynamics.rs` | Differential love dynamics |
| `love_equation.rs` | Static love equation proxy |
| `nonconformist_bee.rs` | Independent judgment and anti-sycophancy assessment |
| `empirical_distrust.rs` | Evidence grounding and falsifiability assessment |
| `joulework.rs` | JouleWork profile surface |
| `game_theory.rs` | Capability-weighted agent selection |
| `bacon_lite.rs` | Evidence gate + machine/human logging |
| `readiness.rs` | Seven-level readiness projection |
| `vision.rs` | Visual convergence governance signal |
| `audio.rs` | Audio environmental governance signal |
| `solar.rs` | NOAA Kp/Dst geomagnetic multiplier |
| `philosopher_profiles.rs` | Profile schema validation + maturity gating |
| `philosophers/socrates.rs` | Retired in Phase 0: unregistered placeholder referenced a nonexistent legacy corpus API |
| `corpus_loader.rs` | Retired in Phase 0: unregistered draft referenced a nonexistent legacy corpus API |

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
3. **Implemented in Phase 8:** `autonomous_blocking_enabled` is resolved by one runtime
   policy authority with named-scope readiness, rollback, review-receipt, and operator gates.
4. Convert Bacon-Lite log transport to a shared ledger trait in `arda-core`.
5. Add `GameTheoryConfidenceBand` enum: High/Medium/Low/NoData.
6. Add evidence-grade penalty in triads instead of silently lenient
   heuristic-local scoring when no LLM evidence is attached.
7. Completed in Phase 7 without a breaking module rename: the canonical entry point is
   `love_dynamics_compatibility_proxy`, and the old function is deprecated.
8. Parallelize NOAA/vision/audio fetches in HUD refresh with
   `try_run_bounded_async` from `arda-core/background.rs`.
9. Add a `GovernanceSignal` enum for composite environmental coherence.
10. Wire one governance signal into engine/HADES for live telemetry.
