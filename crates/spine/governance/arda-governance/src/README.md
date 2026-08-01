---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-07-25

# src

Purpose: Source map for `crates/spine/governance/arda-governance/src`.

## Contents

See `INDEX.md` for deterministic child listing.

## Active governance surfaces

- The explicit `love_dynamics_compatibility_proxy` remains in `love_equation.rs`; the old
  `love_equation_score` name is deprecated and neither surface is canonical Love Dynamics.
- Love Dynamics lives in `love_dynamics.rs` and models `dE/dt = beta * (C - D) * E`.
- JouleWork profiling lives in `joulework.rs` and feeds honesty/efficiency signals.
- Versioned `Task.result` evidence extraction and grading live in `evidence.rs`.
- Triad gate validation lives in `triad.rs`.
- Async scorer contracts, local deterministic scoring, timeout/error receipts, and the
  optional feature-gated LLM scorer live in `scorer.rs`.
- Realm/action rules, atomic reload receipts, and the sole runtime blocking authority live
  in `realm_policy.rs`.
- Nonconformist Bee and Empirical Distrust live in independently testable modules and feed
  the arbitration signals.
- Triad Philosopher arbitration lives in `triad_philosopher.rs`, carries lifecycle receipts,
  and interprets empirical grounding, independence, sycophancy risk, Love Dynamics,
  JouleWork, and defection pressure.
- Resonance composition lives in `resonance.rs`; missing phi inputs are zero-weight and
  philosopher verdicts remain separate decision metadata rather than implicit weights.
- Bounded Bacon-Lite persistence/recovery and metrics live in `bacon_lite.rs` and
  `metrics.rs`; `operator.rs` builds the read-only status projection.
- `normalization.rs` and `versions.rs` centralize bounded score normalization and stable
  policy identifiers; `paths.rs` owns explicit root/path resolution.
- Audio, vision, solar, and environmental modules are quality-tagged advisory inputs only.
