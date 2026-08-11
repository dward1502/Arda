# Archived companion: Arda Avatar Presence Plan

**Status:** Superseded and archived 2026-08-03. Milestones 1-2 were implemented
and verified through the unified living-presence plan. Milestones 3-4 remain
optional future representation/integration work and are not active commitments.

## 1. What You Already Have (Strong Foundation)
Your current system is already well-architected:

presenceState.ts is an excellent single source of truth:
Robust normalization & derivation from both live ArdaPresenceEvents and the JSONL ledger (data/prometheus/arda_presence_events.jsonl).
Clean AgentPresenceState with scenarios (idle | briefing | routing | knowledge | council | alert | recovery), phases (idle | agent_arrival | alert | awaiting_user | action_confirmed | resolved), urgency, multi-agent support, focus, banners, etc.
presenceVisualState() already maps urgency/scenario/phase → pulseRate, ringOpacity, bodyEmissiveIntensity, scanlineOpacity, lightIntensity, supportMarkerScale.
presenceSupportMarkers() already computes orbital positions, colors, labels, and focus flags for secondary agents (arandur, athena, hermes, manwe…).
Agent color palette and short labels are ready.

AvatarEmitterBase is the perfect physical stage:
Geometric platform (torus ring + base cylinder + pulsing core + concentric rings) driven by presenceState.
Color and intensity correctly react to active vs alert vs idle.
useFrame is pure mutation (good R3F hygiene).
Zone definition (boardroom.avatar.emitter) already has binding: 'hologram_anchor' and interaction: 'presence_focus'.


Do not replace this. Treat AgentPresenceState + ledger as the authoritative presence domain and AvatarEmitterBase as the permanent hologram platform. Everything new sits on top of or above this stage.
2. Target Architecture
textboardroom.avatar.emitter (zone)
└── <AvatarEmitterBase ... />                          // existing physical platform (keep)
    └── <AvatarPresenceLayer presenceState={...} />    // NEW – owns representation + juice
          ├── ParticleOrb (or PhysicalOrb fallback)    // Milestone 1
          ├── SupportAgentMarkers                      // from presenceSupportMarkers()
          └── (later) ParticleHumanoid                 // Milestone 3
Key principles:

All visual behavior is driven by the existing AgentPresenceState (and the visual helpers you already wrote).
Phase transitions are the primary trigger for appear/dismiss animations.
A thin imperative surface (AvatarAPI-style) can later force summon/dismiss or inject voice/proximity events, but it still flows through the same state derivation rather than bypassing it.
Progressive representation: start simple, swap the body later without rewriting the state machine or emitter.

3. Recommended Milestones (Actionable Inside Arda)
Milestone 1 – State-Driven Particle Orb + Appear/Dismiss (Highest Priority)
Goal: Turn the central platform into a living presence that materializes and dematerializes with juice, fully driven by existing presence state.

Create AvatarPresenceLayer (or AvatarPresenceBody) as a child of (or sibling positioned above) AvatarEmitterBase.
Implement a first-version ParticleOrb:
Points or InstancedMesh of soft additive particles (start ~12–25k).
Custom ShaderMaterial (or TSL if you are already on WebGPU) with:
Simplex / FBM noise displacement for flow/breathing.
Soft circular particles + additive blending.
Uniforms driven by presenceVisualState() (pulseRate → noise amplitude / flow speed, bodyEmissiveIntensity → brightness, etc.).

Color: primaryAgent color from AGENT_VISUAL_COLORS, overridden to alert pink (#ff4f9d) when urgency high or scenario = alert.

Appear / Dismiss juice (tied to phase):
When phase leaves idle → materialize: particles birth near the platform center, attract/lerp into the orb form with slight overshoot + scale/opacity ramp.
When phase returns to idle → dematerialize: scatter + fade.
Detect phase changes with a ref + previous value; drive with pure useFrame lerp or @react-spring/three / GSAP timeline (whichever you already use in Arda).

Keep motionEnabled gate and add hard pause when phase === 'idle' (or Tauri window blurred) so cost is near zero when dormant.
Adaptive quality (simple FPS monitor or quality tier) controlling particle count.

Deliverable: The orb appears when an agent arrives or an alert happens, feels alive, and disappears cleanly. Fully driven by the existing ledger/events.
Milestone 2 – Support Markers + Richer Reactivity

Render presenceSupportMarkers(state) as small orbital energy points / mini-orbs around the primary form (use the pre-computed angles, radii, colors, phaseOffsets).
They only appear when the primary is active.
Stub audio reactivity: expose a setAudioLevel(level) that modulates particle intensity / noise even if the voice pipeline is not fully wired yet.
Ensure the visual state mapping remains the single place that decides “how intense / turbulent / bright” the presence looks.

Milestone 3 – Progressive Particle Humanoid (Cortana / Joi Look)

Introduce a representation switch ('orb' | 'humanoid') controlled by config or progressive unlock.
Load a stylized low-poly humanoid GLB once (original design, not a direct Ana de Armas likeness).
Use MeshSurfaceSampler (from drei or three/examples) to generate target positions + normals.
Store orb positions and body positions; morph with a progress uniform (0 = pure orb → 1 = full humanoid).
Flowing energy: continuous noise displacement along normals + optional secondary energy filament streams.
Selective bloom only on the avatar layer.
Keep the same appear/dismiss and support-marker systems — they continue to work on top of the morphing form.

Best open reference for this exact aesthetic and technique set: the 2026 cortiz2894/hologram-particles project (WebGPU + TSL, 60k instanced spheres from any GLB, surface sampling, GPU physics, morph, noise along normals).
Milestone 4 – Triggers & Future Integration

Voice, proximity, and action events continue to flow through the existing derivePresenceStateFromArdaPresence / ledger path (or a thin store updater that produces the same AgentPresenceState).
Optional thin imperative API (e.g. avatar.summon(reason), avatar.dismiss(), avatar.setAudioLevel()) that either emits synthetic presence events or directly updates the store.
Later: contextual gestures (lookAt(monitorId), pointTo(...)) that the main boardroom UI can also react to.

4. Performance & VFX Rules (Bake These In From Day 1)

Single draw call preferred (InstancedMesh).
Pre-allocate all buffers/vectors; zero allocations inside useFrame.
Hard pause simulation + reduce to near-zero cost when phase = idle or window not focused.
Adaptive particle count (quality tiers).
Selective post-processing (bloom only on avatar).
Soft particles / depth fade if they intersect the platform geometry.
Prefer WebGPU + TSL when available in the Tauri WebView; keep a clean WebGL fallback.

5. Testing Strategy
Highest value first (pure functions):

Comprehensive unit tests (Vitest/Jest) for every pure function in presenceState.ts:
normalizeAgentPresenceState
derivePresenceStateFromArdaPresence
derivePresenceStateFromEventLedgerRecord / ledger projection
presenceVisualState
presenceSupportMarkers

Fixtures covering: empty ledger, malformed lines, stale vs fresh, multi-agent, urgency ramps, each scenario/phase combination.
Snapshot expected AgentPresenceState and visual props for key scenarios.

Component level:

@react-three/test-renderer or simple prop assertions for AvatarEmitterBase and the new AvatarPresenceLayer (color, intensity, scale changes based on phase/urgency).
Optional Storybook stories or Playwright visual snapshots of the boardroom in idle / agent_arrival / alert states.

Later:

Performance budget checks (GPU time active vs dormant).
Integration tests once Tauri events are feeding the same state path.

6. Small Recommended Extensions to Existing Code

Optionally expand presenceVisualState with a few more fields useful for particles (noiseAmplitude, particleDensityTarget, formProgress, bloomStrength). Keep the function pure.
Consider transient animation flags or a short-lived “materializing / dematerializing” derived state if pure phase jumps feel too abrupt, but try driving everything from phase changes first.
Keep the primary avatar form tied to primaryAgent color; the humanoid silhouette itself can stay consistent (or later become agent-specific if desired).

Summary Roadmap Priority

Now – Milestone 1 (ParticleOrb + appear/dismiss driven by existing phase + visualState). Highest visual impact, lowest risk, fully leverages what you already built.
Milestone 2 – Support markers + reactivity stubs.
Milestone 3 – Morphing particle humanoid.
Milestone 4 – Wire real voice / proximity / action events into the same presence pipeline.

This approach keeps your excellent pure state derivation, multi-agent model, ledger observability, and geometric platform intact while delivering the immersive, progressive, Cortana-style presence we discussed. The avatar will feel native to Arda rather than bolted on.
You can start immediately with Milestone 1 inside the existing boardroom.avatar.emitter zone. When you have the first particle orb + phase-driven materialize working, we can refine the shader, morph path, or test suite further.