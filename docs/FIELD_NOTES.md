---
soterion:
  sigil: "SCROLL"
  glyph: "🧭"
  code_point: "U+1F9ED"
  role: "field_notes"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-07-17"
---

# Field Notes on Arda

What follows is a grounded read from inside the system — based on actual files, real compile results, and direct edits made during this session. No marketing, no projection.

## What Arda actually is right now

Arda is a working desktop application, not a blueprint. It has:
- Two Tauri apps (`arda-launcher`, `arda-hud`) with real Rust backends and React frontends
- Seven published-ish Rust crates under `crates/spine/`: `arda-core`, `arda-governance`, `arda-economics`, `arda-engine`, `arda-mandos`, `arda-vaire`, `manwe`
- A live manwe inference gateway at `127.0.0.1:7171`
- A HUD with 30+ modules, scene layers, review gates, CEO council workflows, and live Charon snapshots

This is not vapor. `cargo check` is green. Tests pass. The launcher renders an atmospheric world-tree while the backend builds `GuidedSession` objects with real operator onboarding logic. The gap between "idea" and "runnable app" is closed.

## The builder

You build like someone who has shipped systems before: you care more about ownership boundaries and cleanup than about adding new features. The repeated pattern in this session was cataloging before acting, organizing before expanding. That’s not hesitancy — it’s discipline born from having seen sprawl kill maintainability in Annunimas.

Your edits are surgical. When something broke (`arda_plutus` import in `arda-vaire` tests), you named the exact fix without drama. When asked to do breakdowns, you wanted actual file-level detail, not summaries — and you called out superficial reads when you saw them. That feedback loop is unusually tight.

The Tolkien overlay is not decoration. It’s a cognitive framework that makes ownership and hierarchy legible: Valar names, sigils, tiers, realms. Whether or not it helps external adoption, it clearly helps *you* think about the system coherently. That matters.

## The migration: Annunimas → Arda

This is the real story of the codebase. Annunimas was the private sovereign system; Arda is the public, runnable, extractable version. The migration is half-done and that’s the interesting part.

What’s clean:
- Crates are renamed and wired (`arda-engine`, `arda-mandos`, etc.)
- Configs are being consolidated under `config/` with domain folders
- `arda-vaire` is a faithful port of `annunimas-mnemosyne` with all the hard stuff intact: significance scoring, hash chaining, Obsidian sync, dual-write

What’s still messy:
- `~/Annunimas/` is still a live parallel tree with duplicate configs, scripts, and crates
- `core/state/` is a flat 200-file namespace with no enforced tiering
- Stale artifacts are everywhere: `tick_output_*.txt`, `metrics/history/`, `_archive/` batches, legacy `annunimas.*` configs
- `core/state/plans/` is failed automation-loop residue that nobody has cleaned up

The Annunimas transfer checklist exists. So does `arda-vaire`. The work isn’t unknown — it’s just unfinished.

## What’s real vs. what’s aspirational

Real and working:
- Governance primitives in code: `triad_validate`, `bacon_lite_validate`, `LoveEquation`, receipts, review gates
- Memory tiering model in design, with a concrete implementation plan
- Local-first inference via manwe + provider catalog
- PTY spawn, source reveal, scoped filesystem access in the HUD backend
- Hash-chained episodic memory with significance classification

Aspirational but not yet built:
- End-to-end demo showing the full flow from HUD → engine → mandos → council → receipt
- Bluefin/OSTree flashable image
- Redis/Dragonfly hot cache
- “Stewards of Arda” course/product surface
- Hardware embodiment / robotics integration

The danger isn’t that the vision is too big. It’s that the current state looks like the vision to casual observers, when really it’s a solid core with messy edges. The presentation needs to match the actual build status.

## Memory as the unifying abstraction

The most important architectural insight from this session: `core/state/` being flat is the single biggest organizational liability. Every consumer reinvents path resolution, loading, and compaction. `arda-vaire` can fix this — not by introducing a database, but by being the mandatory Rust interface that maps every state file to a tier and a domain.

The Vairë implementation plan got more detailed as we talked because the underlying reality is simpler than the docs make it sound: keep JSON files where they are, but enforce ownership through a trait and a path manifest. That’s achievable without breaking anything.

## What I’d tell someone asking about Arda

“It’s a sovereign agentic OS under active construction, built by someone who’s already done this once and is now doing it cleaner. The governance layer is real and tested. The memory model is thoughtful. The sprawl is the risk, not the architecture. When they say Arda-first runnable app, they mean it — the hard part isn’t the vision, it’s the janitorial rigor to keep the codebase from becoming Annunimas again.”

## Open tensions

1. **Public brand vs. private system.** Arda is the public umbrella; Annunimas remains the internal White City. This works if the code rename keeps pace with the docs rename. Right now it’s partial.
2. **Memory ownership.** `arda-vaire` should become the mandatory access layer for all runtime state, but that requires consumer-by-consumer migration. No single PR will do it.
3. **The plans directory problem.** `core/state/plans/` was an attempt at shared task/plan automation that failed. The cleanup is straightforward but requires deciding what, if anything, should survive from those JSON files.
4. **Metrics drift.** `core/metrics/by_crate/*` mirrors runtime state in stale ways. Either metrics need to become consumers of Vairë, or they need to be moved entirely under `ops/`.
5. **End-to-end wiring.** The HUD has all the UI for council, review gates, and approval workflows. The backend has the logic. The wiring between them is the missing piece, not the pieces themselves.

## Closing observation

The system is more mature than its directory structure suggests. The crates are solid. The governance is non-trivial. The memory model is thoughtful. What’s missing is the janitorial pass that makes the architecture legible to anyone other than the builder. That’s not a failure — it’s the natural state of a system that’s been built forward rather than documented backward. The docs and breakdowns we made in this session are the start of fixing that.
