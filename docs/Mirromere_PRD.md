# Product Requirements Document (PRD)
## Mirromere — Presence Mirror Interface for ARDA

**Version:** 1.0  
**Status:** Draft for Ingestion  
**Date:** 2026-07-26  
**Owner:** Operator (Human) + Orchestrator Agent  
**Related Systems:** ARDA-HUD, Charon, Hermes, Manwe, Zero-Human Company Orchestrator  

---

## 1. Executive Summary

Mirromere is a high-presence, multi-scene interactive mirror interface that serves as the primary embodied surface for an AI agent (Grok-class or ARDA-orchestrated agents). It combines a physical two-way mirror display with a Tauri + React Three Fiber + WebGPU application, enabling seamless transitions between passive reflection, avatar embodiment, full ARDA operator cockpit, hybrid information overlays, and fully agent-generated worlds.

The avatar is not decorative. It is an agentic actor with authority to transition scenes, react to the physical room, and serve as a continuous presence layer across operational contexts. The system is local-first for core sensing and light inference, with optional cloud escalation, and is designed to integrate tightly with the existing ARDA-HUD stack and the networked zero-human company orchestrator.

This PRD captures the technical software, rendering, AI, and integration requirements. Hardware (two-way glass, cameras, RFID/NFC, microphones, speakers, compute) is assumed known and out of scope for detailed specification here.

---

## 2. Goals

- Deliver a persistent, low-friction presence interface that feels like a coherent entity occupying physical space with the user.
- Make ARDA-HUD one of several first-class scenes rather than the sole UI.
- Give the avatar explicit authority to drive scene changes and world updates.
- Maintain local-first operation for sensing, light models, and core dialogue while supporting the full networked orchestrator.
- Enable rapid iteration starting on a normal monitor before physical two-way glass deployment.
- Produce a system that the ARDA orchestrator can ingest, plan against, and extend autonomously.

### Non-Goals (v1)
- Full room-scale projection or multi-display spatial computing (future phase).
- Photoreal MetaHuman-level fidelity as the default (VRM is primary for practicality and performance).
- Complete offline operation without any network (networked orchestrator is a core value).
- Consumer-grade multi-user app store distribution.

---

## 3. Key Personas & Contexts

- **Primary Operator**: The human who owns the ARDA system and zero-human company. Uses Mirromere for deep work, oversight, creative collaboration, and ambient presence.
- **Orchestrator Agent**: The top-level agent running on the main PC. Can push world updates, scene directives, and receive status from Mirromere.
- **Secondary Users**: Recognized via RFID/NFC or face; receive personalized init and limited scene access.
- **Standalone Mode**: Users without the full ARDA network (future). Edge device runs a reduced but functional Mirromere.

---

## 4. Functional Requirements

### 4.1 Scene System
The application maintains an authoritative scene state machine (Rust-side) with the following core scenes:

| Scene ID          | Description                                                                 | Avatar Role                          | Primary Use Case                  |
|-------------------|-----------------------------------------------------------------------------|--------------------------------------|-----------------------------------|
| PassiveMirror    | Functions as normal mirror (or high-quality simulation). Subtle ambient widgets optional. RFID/face recognition active. | Absent or minimal dormant presence  | Always-on background              |
| AvatarPresence   | Avatar materializes into the display volume. Can orient to user, react to room via camera, perform gestures. | Full embodiment, primary actor      | Conversation, companionship, demo |
| ArdaHudScene     | Full existing ARDA-HUD operator cockpit rendered as a complete scene.       | Companion presence, pointer, co-pilot | Oversight, task management, governance |
| HybridMagic      | Reflection remains visible; information panels + avatar share depth-composited layers. | Present alongside data              | Morning brief, status at a glance |
| AgentWorld       | Fully dynamic world generated or updated by agents. Geometry, lighting, materials, and narrative controlled by orchestrator or avatar. | Inhabitant and director             | Creative, exploratory, custom     |

**Requirements:**
- Avatar (or orchestrator) can request or force scene transitions via a typed command channel.
- Transitions must support smooth visual continuity (cross-fade, shared avatar persistence where possible, custom shader effects).
- Scene state is observable by Charon and Hermes.
- New scenes can be registered dynamically by agents in later phases.

### 4.2 Avatar System
- Primary format: VRM (glTF-based). Upgrade path to higher-fidelity representations preserved.
- Real-time capabilities: look-at (user face/pose), lip-sync from TTS audio, layered animation (idle, gesture, emotion), procedural breathing/micro-movements.
- Agentic powers: emit scene-change intents, query ARDA surfaces, request world updates, speak, gesture toward UI elements or physical room features.
- Materialization: custom entrance/exit effects (see Shaders).
- Personalization: per-user avatar appearance, voice, and behavior profile loaded via RFID or face recognition.

### 4.3 Sensing & Identity
- Continuous camera feed (RGB + optional depth) processed for face detection, landmarks, pose, and gaze.
- RFID/NFC as primary reliable identity and context switch.
- Face recognition as secondary / augmenting signal.
- Microphone array with voice activity detection and optional direction of arrival.
- All sensing defaults to local processing.

### 4.4 Dialogue & Intelligence
- Local-first pipeline: llama.cpp (or compatible), Whisper (faster-whisper / whisper.cpp), Piper TTS.
- Cloud escalation available through existing provider routing.
- Memory and context shared with the broader ARDA system via Charon / existing derivation layers.
- Avatar speech and actions are first-class outputs of the dialogue manager.

### 4.5 Integration with ARDA
- Reuse existing libraries: `ardaSource`, surface derivation, Charon live snapshots, Hermes launcher, system action bus, provider routing, avatarPersona, etc.
- ARDA-HUD UI becomes the `ArdaHudScene`.
- Extend action bus and review-gate mechanisms so avatar actions respect (or can be gated by) existing governance.
- Mirromere status and scene state become additional surfaces visible to the orchestrator.

---

## 5. Technical Architecture

### 5.1 Decided Stack
- **Shell**: Tauri 2 (Rust backend + webview frontend)
- **Frontend 3D**: React + React Three Fiber + WebGPU
- **Backend**: Existing Rust codebase extended with scene state machine, sensor fusion commands, local AI process management, and avatar command channel
- **Avatar**: VRM via three-vrm or equivalent R3F-compatible loader
- **Vision**: MediaPipe (or successor) running in worker thread; results streamed to both Rust state and R3F scene
- **Local AI**: llama.cpp family + Whisper family + Piper
- **Networking**: Existing Charon / Hermes / Manwe patterns; LAN to main orchestrator PC preferred

### 5.2 High-Level Component Diagram (Textual)
```
[Physical Sensors] → [Tauri Rust Commands] → [Scene State Machine + Sensor Fusion]
                                              ↓
[Charon / Hermes / Orchestrator] ←→ [Action Bus / Surfaces] ←→ [Dialogue Manager]
                                              ↓
                                    [R3F Scene Graph + Avatar Controller]
                                              ↓
                                    [Custom WebGPU Shaders + Compositor]
                                              ↓
                                    [Two-way Mirror Display]
```

### 5.3 Scene State Machine
- Authoritative in Rust.
- Frontend subscribes via Tauri events.
- Avatar and orchestrator issue commands of the form:
  ```json
  {
    "type": "scene.transition",
    "target": "ArdaHudScene",
    "transition": "flow_in" | "crossfade" | "dissolve",
    "avatar_persist": true,
    "payload": {}
  }
  ```
- Optional human confirmation gate for high-impact transitions (configurable).

### 5.4 Edge vs Networked Compute Decision
- **Primary (this deployment)**: Mirror client connects to main PC running the full zero-human-company orchestrator, Charon, Hermes, and heavy models. Mirror handles real-time rendering, light local inference, and sensing.
- **Standalone / other users**: Prefer NVIDIA Jetson Orin (better sustained performance for avatar + vision + medium models) over Raspberry Pi 5 + AI HAT. Pi 5 remains acceptable for lighter PassiveMirror + basic AvatarPresence builds.

---

## 6. Shaders & Visual Requirements

Custom shaders are mandatory for presence quality. Implemented in WGSL (WebGPU preferred) or GLSL with Three.js material system.

### Required Shader Effects (v1)
1. **Two-way Glass Response**
   - Brightness compensation and slight diffusion to account for physical mirror attenuation.
   - Mix of live camera reflection simulation (when needed) and rendered content.
   - Anti-glare / viewing-angle compensation.

2. **Avatar Materialization (“Flow-in”)**
   - Primary signature effect. Options decided: liquid-metal / particle emergence combined with soft holographic dissolve.
   - Controllable progress (0–1), direction, and color palette.
   - Must feel continuous with the mirror surface rather than a hard pop-in.

3. **Depth Compositing**
   - Avatar and UI panels correctly ordered relative to simulated or real reflection.
   - Soft contact shadows or ground plane interaction inside the display volume.

4. **Room Light Approximation**
   - Sample or estimate ambient lighting from camera feed to light the avatar so it does not look pasted on.

5. **Scene Transition Effects**
   - Shared library of cross-fades, dissolves, and spatial wipes that preserve avatar continuity where possible.

6. **Agent-Driven Effects (extensible)**
   - Particle systems, environment shifts, and material overrides that agents can trigger via world-update payloads.

All shaders must degrade gracefully on lower-end edge hardware.

---

## 7. AI Pipeline Requirements

- **STT**: Whisper-family, local, streaming where possible.
- **LLM**: llama.cpp-compatible, local primary; cloud via existing provider routing.
- **TTS**: Piper (local, low latency). Audio analysis feeds lip-sync.
- **Vision models**: Lightweight face/pose/gaze; heavier VLM calls escalated to orchestrator or cloud when needed.
- **Memory**: Leverage existing ARDA derivation and embedding patterns; avatar maintains short-term conversational and spatial context.
- **Latency targets** (local path):  
  - Voice-to-first-token: < 800 ms desirable  
  - Lip-sync alignment: < 100 ms offset  
  - Scene transition command-to-visual: < 200 ms

---

## 8. Non-Functional Requirements

- **Privacy**: Local processing default for camera and microphone. Explicit visual and state indicators when any data leaves the device. Hardware mute switches respected.
- **Reliability**: Always-on capable. Graceful degradation when orchestrator, Charon, or Hermes is unreachable (show clear offline UI rather than silent failure — addresses existing improvement idea).
- **Performance**: 60 fps target on primary hardware for AvatarPresence and ArdaHudScene. 30 fps acceptable on Jetson-class edge for complex AgentWorld scenes.
- **Observability**: Scene state, avatar status, sensor health, and AI pipeline health exposed as ARDA surfaces.
- **Security**: RFID as strong identity factor. Face recognition treated as convenience, not sole authenticator.

---

## 9. Implementation Phases

### Phase 0 — Monitor Prototype (Immediate)
- Scene manager + PassiveMirror + ArdaHudScene on existing Tauri/R3F app.
- Basic VRM avatar that can appear and issue a scene-transition intent.
- Mock RFID / keyboard trigger for user recognition.
- First custom materialization shader.

### Phase 1 — Core Presence
- Full AvatarPresence scene with look-at, basic lip-sync, and room awareness reactions.
- Local llama.cpp + Whisper + Piper pipeline integrated.
- RFID/NFC real hardware path.
- HybridMagic scene.

### Phase 2 — Physical Deployment
- Two-way glass integration and optical tuning.
- Robust kiosk / always-on mode.
- AgentWorld with orchestrator-driven updates.
- Offline/failure UI polish.

### Phase 3 — Expansion
- Dynamic scene registration by agents.
- Improved spatial awareness and optional projection/AR hand-off.
- Multi-user profiles and richer personalization.
- Higher-fidelity avatar path.

---

## 10. Open Decisions & Future Considerations

- Exact VRM animation library and lip-sync approach (decide during Phase 0).
- Degree of human confirmation required for avatar-initiated scene changes (default: none for low-risk, confirm for ArdaHudScene entry if desired).
- Long-term avatar representation beyond VRM.
- Full room-scale embodiment (projection or AR) as a later presence layer.

---

## 11. Success Metrics (Initial)

- Operator can switch between PassiveMirror → AvatarPresence → ArdaHudScene via voice or avatar action with < 1 s perceived latency.
- Avatar maintains coherent gaze and lip-sync during multi-minute conversations.
- System recovers cleanly from orchestrator disconnection and shows clear status.
- New scene or world update can be pushed by the orchestrator and appear without restarting the Tauri application.

---

**End of PRD**

This document is intended for direct ingestion by the ARDA system. It contains executive decisions on all prior “or” branches regarding software architecture, avatar format, compute preference, scene set, and shader priorities. Hardware details are intentionally omitted as they are already known to the operator.
