# ARDA

ARDA is the rebrand and structural refactor of the Annunimas agentic system.
Annunimas is being parsed apart deliberately; ARDA is the new surface.
This README exists because ARDA's own architecture is still being assembled
and the intent needs to survive the rebuild.

## What ARDA is supposed to be

A local-first agent runtime with an operator-facing desktop surface.
Not a single opaque runtime. A modular stack where each piece can be built,
verified, tested, and kept inspectable.

## What exists now versus what is intended

Current state of this repository is minimal and unstable by design.
The pieces are being ported piece by piece, not renamed and committed as if
finished. Expect cycles of deletion, addition, and renames.

## Subsystems being ported from Annunimas

- fleet and node topology
- network mesh / provider routing
- LLM provider configuration and fallback routing
- memory and recall
- core runtime state and persistence
- operator diagnostics, logging, and observability surfaces

## Active launcher: HUD/arda-launcher

The desktop entry point is `HUD/arda-launcher`.
On first launch it should detect missing subsystems and run setup.
On later launches it should detect the installed state and run onboarding.

Currently the frontend renders the intro visuals and the Tauri backend is a
placeholder. The README for HUD/arda-launcher documents the intended behavior
and current limitations.

## Repository layout

```
Cargo.toml          # workspace root
src/main.rs         # stub binary
crates/
  arda-core/        # initial stub crate
  README.md         # ARDA vision and architecture map
HUD/arda-launcher/ # active Tauri/React desktop launcher
```

## Notes

- This is an in-progress migration.
- Structure and naming are expected to shift as subsystems are validated.
- The goal is a working runtime, not a renamed snapshot.
