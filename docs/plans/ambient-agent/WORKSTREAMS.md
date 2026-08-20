---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "execution_map"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: 📜 execution_map | owner: PROMETHEUS | status: active | reviewed: 2026-08-20

# Ambient Agent Execution Order and Ownership

## Current hold

Do not dispatch or implement ambient-agent work until the core-Arda prerequisite in [`README.md`](README.md) is accepted by the operator. Planning, audit, and correction are allowed. Application code, sensor activation, hardware integration, and commercialization execution are not.

When implementation eventually resumes, direct implementation is the default. Do not delegate coding unless the operator explicitly asks for delegation.

## Required sequence

1. Make core Arda useful for capture, next action, context recovery, Personal Operations, research, review, and restart recovery.
2. Build and obtain operator acceptance for the monitor-first Mirromere embodied assistant.
3. Add real presence/identity providers one at a time.
4. Wire RELIC only to genuine runtime activity that already exists.
5. Prove physical-action governance with a simulator before any real actuator.
6. Test external product demand only with a slice the operator already finds useful.

Later numbering does not override these gates. Do not run phases in parallel merely because their files do not overlap.

## Authority ownership

| Concern | Existing owner | Ambient consumers may do |
|---|---|---|
| Conversation, tools, sessions, gateway | Hermes | Continue the relationship through a narrow client; never clone or display management UI as the product |
| Durable context and memory policy | Vairë | Request bounded context references; never create a Mirromere memory store |
| Research and evidence | Varda | Present source-grounded results; never invent research truth in a scene |
| Commitments/tasks | Core Arda task authority | Capture and present; never create a parallel task queue |
| Approval and consequential action | Existing Arda review/receipt paths | Present proposals and submit exact human decisions |
| Personal routines | Personal Operations | Capture, acknowledge, defer, correct, and display bounded projections |
| Mirromere scenes/avatar/local media | Standalone Mirromere app | Own rendering, voice interaction, local privacy state, and scene transitions |
| Presence signals | Presence sidecar/outpost contract | Emit expiring local claims; never authorize action |
| Runtime visualization | RELIC bridge | Render read-only fresh events; never infer or mint activity |
| Device execution | Enrolled device adapter | Validate one exact approved intent and emit one terminal receipt |

## Path ownership when work resumes

- `apps/arda-mirromere/**`: Mirromere application only.
- `apps/arda-hud/**`: HUD only; it must not create or own the Mirromere native window.
- `crates/spine/interface/arda-mirromere/**`: shared bounded contracts/client logic, not product UI.
- `outposts/arda-outpost-protocol/**`: serialized schema changes; one reviewed change at a time.
- `outposts/arda-relic-bridge/**`: RELIC read-only transport and mapping.
- `config/systemd/**`: deterministic lifecycle only; services do not prove useful behavior.
- `core/projects/tasks/queue.jsonl`: canonical append-only task authority.
- `core/state/queue_active.json` and `core/state/queue_summary.json`: generated projections; never stage them with feature work.

## Task admission test

A future implementation task is allowed only if it:

- closes a literal behavior or safety item in one active plan;
- depends on a prerequisite the operator has actually accepted;
- names what the human will see or be able to do;
- reuses the existing authority that owns its data/action;
- defines failure, privacy, close, and recovery behavior;
- cannot be satisfied by labels, tests, screenshots, service health, or decorative output alone.

Reject tasks whose result is only a schema, projection, daemon, test fixture, visual metaphor, or proof artifact unless that artifact is a necessary bounded dependency of an already-approved visible behavior.

## Closeout rule

Only the operator can accept usefulness and product behavior. Engineering agents may report code, tests, runtime state, and observed failures, but may not convert those into operator acceptance or archive a product plan without explicit operator approval.
