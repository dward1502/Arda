---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-05-21

# src

Purpose: Source map for `crates/annunimas-governance/src`.

## Contents

See `INDEX.md` for deterministic child listing.

## Active governance surfaces

- Love compatibility scoring remains in `love_equation.rs` for existing callers.
- Love Dynamics lives in `love_dynamics.rs` and models `dE/dt = beta * (C - D) * E`.
- JouleWork profiling lives in `joulework.rs` and feeds honesty/efficiency signals.
- Triad gate validation lives in `triad.rs`.
- Triad Philosopher arbitration lives in `triad_philosopher.rs` and interprets empirical grounding, independence, sycophancy risk, Love Dynamics, JouleWork, and defection pressure.
- Resonance composition lives in `resonance.rs`; it preserves existing score weights while attaching optional Love Dynamics and Triad Philosopher metadata.
