Arda Living Presence — Unified Plan
(Identity + Visual Embodiment)

**Status:** Complete and archived 2026-08-03. Tracks A-C are implemented and
verified. The optional cosmic/starfield humanoid and native active-state visual
comparison remain future refinements, not unfinished scope in this plan.

This document merges the two existing plans into a single, agent-executable roadmap.
It treats the persona memory layer (arda-vaire) as the soul and the progressive particle presence (boardroom R3F) as the body. Mood and traits are allowed to influence the visual system, but the two systems remain cleanly namespaced and can be built in parallel.

0. Vision (the fun part)
The central hologram is not a static skin.
It is a living presence that starts as a soft particle orb, can materialize into a humanoid form, and — when mood or curiosity is high — can dissolve into pure starlight particles and reassemble.
Personality is not hardcoded. It is grown from evidence.
Traits only appear after repeated, independent memory records. Mood decays. The visual system listens to both and expresses them as density, turbulence, color temperature, and dissolve tendency.
The cosmic starfield form we explored is the aspirational long-term look for the primary presence (especially for Arandur). It is not required for Milestone 1.

1. Ownership & Namespaces (locked)

Personality identityarda-vairepersona.* inside MemoryRecord.extensionsExclusiveVisual presence stateboardroom / HUDAgentPresenceState + presenceVisualStateExistingBridge / influencethin mapping layerderived uniforms onlyNo new storeObsidian projectionarda-vairehuman/personality/arandur/Same cadence as other projections

ConcernOwnerNamespace / LocationNotesPersonality identityarda-vairepersona.* inside MemoryRecord.extensionsExclusiveVisual presence stateboardroom / HUDAgentPresenceState + presenceVisualStateExistingBridge / influencethin mapping layerderived uniforms onlyNo new storeObsidian projectionarda-vairehuman/personality/arandur/Same cadence as other projections
Any future subsystem must claim a top-level namespace before writing code. persona.* is taken.

2. Identity Layer (from arda_vaire plan — kept almost intact)
File: crates/arda-vaire/src/persona/types.rs
RustPersonaTrait { id, label, evidence_count, confidence, first_seen, last_seen, last_reinforced_by, stale? }
MoodSample  { timestamp, valence: f32 (-1..1), source_record, outcome_class }
MoodSummary { as_of, weighted_valence, sample_count, window_hours }
ValueEvidence { value_id, evidence_count, source_records }
Promotion rule (non-negotiable):

≥ 3 independent evidence records inside a 30-day window before a trait is written at all.
confidence = min(1.0, evidence_count / 10.0)
60 days without reinforcement → stale: true (still visible, de-emphasized).
Single events only affect mood, never traits.

Mood decay:
textweight = exp(-λ * age_hours)   where λ = ln(2)/24   // 24 h half-life
Window: last 200 samples or 14 days (whichever smaller). Recomputed on read / at consolidation, cached into persona.mood_summary.
Derivation:

Hooked into the existing consolidate / promotion path. No new cron.
Only Active / Promoted records. Idempotent. Emits a semantic MemoryRecord tagged derivation=persona_identity.

Obsidian:
human/personality/arandur/<date>.md regenerated each consolidate cycle (traits + confidence + stale markers + recent evidence links).

3. Visual Presence Layer (from presence plan — kept intact)
Keep everything you already have:

presenceState.ts as visual single source of truth
AvatarEmitterBase as the permanent physical stage (hologram_anchor)
presenceVisualState() and presenceSupportMarkers()

New component hierarchy:
textboardroom.avatar.emitter
└── AvatarEmitterBase                    // keep
    └── AvatarPresenceLayer              // new orchestrator
          ├── ParticleOrb                // Milestone 1
          ├── SupportAgentMarkers
          └── ParticleHumanoid           // Milestone 3 (later)
Milestones (visual):

ParticleOrb + appear/dismiss driven purely by existing phase (highest priority, ships independently).
Support markers + basic reactivity stubs.
Progressive humanoid (MeshSurfaceSampler + morph + noise-along-normals). Cosmic starfield form is a valid target mesh.
Voice / proximity / action events continue to flow through the existing derivation path.

Performance rules stay exactly as written (InstancedMesh, hard pause when idle, adaptive count, selective bloom, zero alloc in useFrame).

4. The Bridge (new — the important improvement)
Persona state is allowed to influence the visual system, never replace it.
Mapping (proposed defaults — tune later):

mood_summary.weighted_valence highhigher density, warmer color temperature, calmer noisedensity, colorTemp, noiseAmp ↓mood_summary.weighted_valence lowlower density, cooler cyan, higher turbulence, tendency to dissolvedensity ↓, noiseAmp ↑, dissolveBiasHigh-confidence traits presentsubtle secondary particle streams or slower morphoptional traitAccentStale traitsno visual change (HUD only)—Alert / high urgency (from presence)overrides mood for color (pink) and pulse rateexisting logic wins

Persona signalVisual effectUniform / behaviourmood_summary.weighted_valence highhigher density, warmer color temperature, calmer noisedensity, colorTemp, noiseAmp ↓mood_summary.weighted_valence lowlower density, cooler cyan, higher turbulence, tendency to dissolvedensity ↓, noiseAmp ↑, dissolveBiasHigh-confidence traits presentsubtle secondary particle streams or slower morphoptional traitAccentStale traitsno visual change (HUD only)—Alert / high urgency (from presence)overrides mood for color (pink) and pulse rateexisting logic wins
Implementation:

Extend presenceVisualState() (or a thin pure function next to it) to accept an optional PersonaProjection.
The R3F layer reads the latest persona projection the same way the HUD does (existing IPC / query path — no new bridge).
If no persona data exists yet, visual system falls back to pure phase/urgency behaviour (zero regression).

This is the piece that makes the avatar feel like it has an internal life.

5. Unified Sequencing (agent-friendly)
Track A — Visual (can start immediately)

AvatarPresenceLayer + ParticleOrb + phase-driven materialize/dematerialize.
Wire presenceVisualState fully.
Support markers.
(Later) humanoid + surface sampling.

Track B — Identity (can run in parallel)

- [x] `persona/types.rs` + `persona.schema_version = 1`.
- [x] Derivation + promotion/decay rules + unit tests.
- [x] Hook into existing consolidate with atomic, idempotent projection replacement.
- [x] Local Obsidian-compatible Markdown projection (no account coupling).
- [x] HUD consumer (`statefulPersona.ts`).

Track C — Bridge (after both have minimal viable versions)

Optional persona input into presenceVisualState.
Mood → density / dissolve mapping.
Visual tests that assert different particle behaviour under different valence.

Each step stays a separate commit/PR. Promotion-rule tests must pass before HUD or bridge work is merged.

6. Testing Strategy (combined)
Identity (Rust):

Single evidence → no trait.
3 evidence → trait appears at confidence 0.3.
61-day silence → stale flag.
Mood weight ratio for 5 h vs 5 d samples.
Idempotent derivation.

Presence (TS):

All existing pure functions in presenceState.ts (already planned).
Phase change → appear/dismiss progress.
Visual props under alert vs idle.

Bridge:

Same presenceState + different mood valence → different density / noiseAmp values.
Missing persona data → no crash, pure phase behaviour.


7. Explicit Non-Goals (this slice)

No CuriosityQuery, no contract version bump.
No personal-ops or business-improvement wiring.
No full mood history chart.
No second memory store or second avatar pipeline.
No hardcoded persona skins as source of truth (they become rendering skins only).
Direct Cortana / celebrity likeness models remain forbidden.


8. Agent Build Checklist (start here)
This week (Visual Track A):

- [x] Create `AvatarPresenceLayer.tsx` inside the permanent `AvatarEmitterBase` stage.
- [x] Add a `ParticleOrb` driven by existing `presenceVisualState` and phase.
- [x] Materialize on active phases and dematerialize on `idle` / `resolved`.
- [x] Hard-pause particle frame mutation after dismissal reaches zero.
- [x] Render bounded support-agent markers from `presenceSupportMarkers`.
- [x] Cover all new pure animation helpers with unit tests.

Track A Milestone 1 closed 2026-08-03:

- `particlePresence.ts` owns deterministic phase-to-density/motion targets and bounded transition stepping.
- `ParticleOrb.tsx` retains immutable base positions, performs no per-frame allocations, and restores each animated position from its base instead of accumulating drift.
- The presence layer consumes pulse, opacity, emissive, scanline, light, and support-marker values from the existing visual derivation; alert precedence remains unchanged.
- The live boardroom mounts the layer under `boardroom.avatar.emitter`; it is no longer debug-only or duplicated at scene origin.
- Evidence: 14/14 focused presence tests, 372/372 full HUD tests, `pnpm run build`, `pnpm run lint` with 0 errors, and degraded-web idle-emitter visual inspection.

Identity Track B:

- [x] `persona/types.rs` + `schema_version = 1`
- [x] Promotion + decay unit tests first
- [x] `derive_identity_summary` hooked into existing consolidate
- [x] Local Obsidian-compatible Markdown projection
- [x] `statefulPersona.ts` canonical projection consumer
- [x] Live Arandur Personality subpanel with neutral missing-data state

Track C — personality-to-visual bridge:

- [x] Extend `presenceVisualState` with optional persona-derived uniforms and neutral defaults.
- [x] Map positive valence toward warmer color, higher density, and calmer motion.
- [x] Map negative valence toward cooler color, lower density, turbulence, and dissolve tendency.
- [x] Ignore stale trait accents and keep unavailable persona data phase-pure.
- [x] Preserve alert precedence for pink color and urgent pulse behavior.
- [x] Load the latest canonical persona projection in the live R3F presence layer through the existing read boundary.

Track C source bridge closed 2026-08-03:

- `presenceVisualState(state, persona?)` remains pure and owns bounded density, noise, dissolve, color-temperature, and non-stale high-confidence trait signals.
- `ParticleOrb` consumes those signals without adding another avatar store or changing the phase state machine.
- `AvatarPresenceLayer` reads `core/state/identity/<actor>.json` through `loadStatefulPersona`; missing or unavailable data resolves to the established neutral visual values.
- Alert state still overrides mood for color and pulse, while persona may continue to influence density and motion.
- Evidence: 21/21 focused bridge/persona tests, 376/376 full HUD tests, `pnpm run build` (2,582 modules), `pnpm run lint` with 0 errors, `git diff --check`, and current Tauri/Vite runtime modules serving the new loader/color/dissolve paths.
- Native active-state visual comparison remains an optional follow-up confidence
  gate; the source bridge does not claim it from unit/build evidence alone.

Later:

- Cosmic / starfield humanoid mesh as optional representation.


Final note for the agent
The visual system can ship and look alive before any personality data exists.
The personality system can accumulate evidence before the particle system knows how to listen to it.
The bridge is what turns both into a single living presence.
When you are ready for the first PR, start with the ParticleOrb under AvatarPresenceLayer. Everything else is designed to attach cleanly afterwards.
This is the combined plan.