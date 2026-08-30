---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-20

# Phase 3: Mirromere Embodied Assistant

> **Planning-only hold:** Do not implement this phase until the operator confirms that core Arda is useful for the daily workflows Mirromere is meant to embody. Do not delegate implementation by default. Human product acceptance—not tests, labels, screenshots, or agent-authored evidence—controls progression.

## What Mirromere is

Mirromere is the room-facing embodied presence of the Arda/Hermes agent. It begins on the operator's second monitor and may later move behind two-way mirror glass.

The human should experience:

- a persistent avatar or restrained fallback presence occupying the display;
- natural text and voice conversation without seeing Hermes administration UI;
- visible listening, thinking, speaking, interrupted, muted, private, and offline behavior;
- contextual presentation of one useful item, brief, reminder, approval, or application when the conversation calls for it;
- calm passive behavior when no interaction is occurring;
- continuity with the same underlying Arda/Hermes relationship and memory without exposing dashboards, service status, prompts, or internal agent machinery.

Mirromere uses Hermes and Arda underneath. It does not display the Hermes dashboard and is not another operator cockpit.

## What Mirromere is not

None of these satisfy the product:

- radar, wave, particle, or status pictures presented as useful behavior;
- a rotating gallery of lifecycle states;
- the Hermes dashboard embedded or navigated into the Mirromere window;
- accessibility labels, process health, backend contracts, tests, or packaging by themselves;
- the ARDA HUD copied onto a second monitor as the default experience;
- an autonomous authority separate from Arda's memory, policy, task, research, or approval systems.

The failed projection/dashboard work is retained only as a warning in [`../../archive/2026-08-20-failed-mirromere-projection-implementation.md`](../../archive/2026-08-20-failed-mirromere-projection-implementation.md).

## Prerequisite: core Arda must be useful first

Implementation is blocked by the active
[`Arda Whole-System Completion Program`](../ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md).
Core Arda must own a real outcome through context retrieval, decomposition,
authorization, scheduling, provider placement, execution, verification, review,
revision/retry, restart recovery, and accepted closure across connected projects.
Its daily research/improvement cycle must also produce verified work and continue
unfinished outcomes without repeated operator task assignment.

Capture, next-action, context-resume, reminders, cited research, approval, and
restart interactions remain useful surface checks; they are not the whole Core
Arda gate.

Mirromere must consume those working capabilities. It must not create visual stand-ins for capabilities Arda does not yet provide.

## Existing code disposition

The standalone Tauri package, second-display selection, close semantics, and packaging may be retained as infrastructure after review.

Before new product work begins:

- remove direct Hermes-dashboard navigation from Mirromere;
- remove the decorative radar/status renderer from the default product path;
- keep old scenes only as explicitly labeled development fixtures if they remain useful for tests;
- verify the retained shell does not own or duplicate Arda/Hermes authority.

This cleanup is not product completion.

## Monitor-alpha build order

### Task 1: Rust-owned scene and interaction state

Implement an authoritative state machine for:

- `PassiveMirror` — quiet background/reflection mode;
- `AvatarPresence` — embodied conversation;
- `ArdaHudScene` — explicitly requested full operator cockpit;
- `HybridMagic` — avatar plus a small amount of contextual information;
- `PrivacyMuted` and `Offline` — always locally reachable.

`AgentWorld` and dynamic agent-created scenes are later work. The first implementation must not disguise placeholders as working scenes.

Visible result: the human can deliberately move between passive, avatar, HUD, private/muted, and offline modes and can always tell which mode is active.

### Task 2: Basic embodied avatar

Add one VRM avatar with:

- stable idle presence and procedural breathing/micro-motion;
- gaze/look-at driven by explicit local input or a development pointer fixture;
- listening, thinking, speaking, interrupted, and offline states;
- gesture and expression commands from typed dialogue output;
- graceful non-humanoid fallback if the model or renderer fails;
- reduced-motion behavior.

Visible result: the avatar acts as the agent's conversational body rather than as decoration around a status display.

### Task 3: Typed conversation through Arda/Hermes

Connect Mirromere to the existing Arda/Hermes conversational runtime through a narrow client contract. Do not embed the dashboard and do not create a second memory or session system.

A response may contain:

- spoken/display text;
- avatar expression and gesture;
- optional scene-transition request;
- optional bounded content/application presentation;
- references needed for continuity and receipts.

Visible result: the human speaks or types to the avatar and receives a coherent response from the same agent relationship while remaining inside the Mirromere scene.

### Task 4: Local voice pipeline

Add supervised local voice activity detection, streaming Whisper-family speech-to-text, typed dialogue, Piper-family text-to-speech, and viseme timing.

Required behavior:

- obvious local microphone mute and quiet mode;
- interruption/barge-in stops speech promptly;
- failed voice falls back to text without losing captured intent;
- raw audio remains ephemeral by default;
- no camera is required for monitor alpha.

Visible result: a multi-minute conversation works through the avatar with understandable turn-taking and lip-sync.

### Task 5: Practical daily assistance

Only after the corresponding core Arda workflows are operator-usable, expose:

- one next action;
- morning, transition, and evening check-ins;
- Personal Operations capture and reminder acknowledgement/defer;
- “What was I doing?” context recovery;
- a concise source-grounded research result;
- review of a consequential proposed action.

These appear contextually inside `AvatarPresence` or `HybridMagic`; they are not permanent dashboard panels.

Visible result: Mirromere reduces the effort required to start, remember, capture, or continue real work.

### Task 6: Development presence trigger

Use a keyboard or explicit mock RFID trigger to exercise arrival/departure and personalization without activating camera, face recognition, or covert sensing.

Visible result: arrival can move PassiveMirror to a privacy-safe greeting; departure returns it to passive/private state. Real identity and sensor providers belong to Phase 4.

### Task 7: Monitor-alpha operator acceptance

The operator—not an implementation agent—uses the installed second-monitor application for:

1. a sustained text and voice conversation;
2. interruption, mute, privacy, offline, and recovery behavior;
3. one real context-resume or Personal Operations interaction;
4. one explicit transition into and back out of `ArdaHudScene`;
5. ordinary close and relaunch behavior.

## Phase gate

Phase 3 remains open until all of the following are true:

- core Arda satisfies the prerequisite daily workflows;
- Mirromere presents an embodied avatar, not radar or dashboard UI;
- text and local voice conversation work through the avatar;
- practical Arda content appears contextually without becoming a dashboard;
- privacy, mute, offline, interruption, close, and recovery work visibly;
- the operator chooses to keep using it.

Builds, tests, contracts, process state, accessibility inspection, and agent-authored walkthroughs may support development but cannot close this phase.
