---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  role: "governance_engine"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-28"
---

# arda-governance
Governance engine for Arda agents: triad gates, resonance, love dynamics,
JouleWork profiling, readiness levels, and philosopher arbitration.
Owner: hades | Sigil: 🜏 REPAIR | Status: active

This is the canonical implementation map. Current health and verification evidence live in
`STATUS.md`; possible future changes live in `PLAN.md`.

## Summary
`arda-governance` is the central governance crate in the Arda spine.
It implements the actual scoring surfaces behind autonomous decision
quality: triad validation, resonance/joule/love metrics, game-theory
agent selection, readiness projections, Bacon-Lite evidence logging,
audio/vision/solar environmental governance signals, and a deterministic
philosopher arbitration layer. It depends on `arda-core`; its direct workspace consumers are
listed below and its supported consumer surface is re-exported from `src/lib.rs`.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/governance/arda-governance`
- Configs: `../../../../config/governance/chains.toml`, `../../../../config/governance/philosophers.toml`
- Tests: 67 unit tests, 48 all-feature integration tests across nine targets, and three
  doctests (118 total); no-default runs 47 integration cases (117 total).

## Verification status

The crate is stable for its current advisory/governance scope. The all-feature test, strict
Clippy, formatting, and rustdoc gates pass; exact commands and counts are in `STATUS.md`.
No incomplete crate-local implementation item remains. Conservative non-blocking defaults
are intentional safety behavior rather than unfinished work.

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
| `audio.rs` | Audio environmental advisory signal |
| `bacon_lite.rs` | Bounded evidence persistence, recovery, and ledger reads |
| `environmental.rs` | Typed multi-source advisory coherence |
| `evidence.rs` | Versioned structured evidence extraction and grading |
| `empirical_distrust.rs` | Independent evidence-grounding assessment |
| `game_theory.rs` | Capability-weighted agent selection and explicit fallback |
| `joulework.rs` | JouleWork profile and measurement provenance |
| `love_dynamics.rs` | Canonical cooperation/defection dynamics |
| `love_equation.rs` | Deprecated-name-compatible task-value proxy |
| `metrics.rs` | Bounded-label in-process metrics |
| `nonconformist_bee.rs` | Anti-sycophancy and independence assessment |
| `normalization.rs` | Bounded score normalization |
| `operator.rs` | Read-only operator status projection |
| `paths.rs` | Explicit runtime path resolution |
| `philosopher_profiles.rs` | Profile schema, lifecycle, and maturity gates |
| `readiness.rs` | Evidence-backed seven-level readiness projection |
| `realm_policy.rs` | Realm/action policy, atomic reload, and blocking authority |
| `resonance.rs` | Resonance and phi/governance composition |
| `scorer.rs` | Async scorer contracts, receipts, and optional LLM adapter |
| `solar.rs` | NOAA Kp/Dst client, cache, and degraded semantics |
| `triad.rs` | Triad gate, governance chains, TOML load/validate |
| `triad_philosopher.rs` | Alignment arbitration |
| `vision.rs` | Visual convergence governance signal |
| `versions.rs` | Stable policy and semantics identifiers |

## Direct workspace consumers

- `manwe` — adaptive route preview/selection governance and blocking receipts.
- `arda-aule` — operator status and metrics presentation.
- `arda-varda` — governed ingestion/task receipts and environmental evidence.
- `arda-mandos` — policy/outcome authority integration.
- `arda-orome` — dispatch governance hooks.
- `arda-economics` — governance-aware runtime economics.
- `arda-vaire` — governance/memory integration scenarios.
- `arda-engine` — aggregate observability and governance-counter projection.

The workspace root also declares the dependency centrally. Consumers should prefer crate-root
re-exports and treat `tests/fixtures/public_api_v1.json` as the wire-compatibility baseline.

## Change boundaries

- Do not make advisory environmental inputs authoritative without a new reviewed contract.
- Do not enable blocking from legacy config flags; `RuntimeBlockingAuthority` is the sole gate.
- Do not remove/rename public fields, wire names, or crate-root exports without compatibility
  review and fixture updates.
- Do not treat default/estimated JouleWork or missing environmental data as observed evidence.
- Keep future proposals and decisions in `PLAN.md`, not in this implementation map.

## Exact Rust source classification (2026-07-28)

Production/default (25):

- `src/audio.rs`, `src/bacon_lite.rs`, `src/empirical_distrust.rs`, `src/environmental.rs`
- `src/evidence.rs`, `src/game_theory.rs`, `src/joulework.rs`, `src/lib.rs`
- `src/love_dynamics.rs`, `src/love_equation.rs`, `src/metrics.rs`
- `src/nonconformist_bee.rs`, `src/normalization.rs`, `src/operator.rs`, `src/paths.rs`
- `src/philosopher_profiles.rs`, `src/readiness.rs`, `src/realm_policy.rs`, `src/resonance.rs`
- `src/scorer.rs`, `src/solar.rs`, `src/triad.rs`, `src/triad_philosopher.rs`
- `src/versions.rs`, `src/vision.rs`

`src/scorer.rs` is compiled by default; only its LLM adapter sections are guarded by
`llm-scorer`. Therefore no whole production file belongs in the feature-gated category.

Integration test/build script (9/0):

- `tests/alignment_stack.rs`, `tests/governance_observability.rs`
- `tests/path_independence.rs`, `tests/phase7_philosopher_expansion.rs`
- `tests/phase8_realm_policy.rs`, `tests/philosopher_profiles.rs`
- `tests/policy_versioning.rs`, `tests/public_api_compat.rs`
- `tests/structured_evidence.rs`

Production/feature-gated: 0 standalone files. Generated include: 0. Test-only standalone
source: 0. Unwired: 0. The module graph has no latent file-vs-directory root collision.
