---
sigil: SCROLL
soterion:
  id: echo-nexus-prompting
  version: 1.0.0
  classification: general-document
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: general
  tags:
    - documentation
    - general
  mnemosyne:
    lineage: echo-nexus-prompting-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

---
sigil: SCROLL
soterion:
  id: annunimas-echo-nexus-prompting
  version: 1.0.0
  classification: governance-prompting
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: governance
  tags:
    - echo-nexus
    - prompting
    - governance
    - communication
  mnemosyne:
    lineage: annunimas-echo-nexus-prompting
    memory_type: governance-reference
---
# Prompting Codex For Echo Nexus

## Purpose
This guide defines the preferred way to prompt Codex for Echo Nexus work.

The goal is not vague feature asking. The goal is controlled system modification with clear constraints, preserved behavior, and verifiable outcomes.

## Why This Matters
Inside Echo Nexus, Codex is most reliable when prompts define:

- the system context
- the exact task
- the constraints
- the integration points
- the expected output behavior

This reduces drift, protects stable surfaces, and improves deterministic implementation.

## Core Principle
Do not say only:

`build X`

Prefer:

`Here is the system. Here is the constraint. Here is the exact behavior I want changed.`

## Echo Nexus Prompt Pattern
Use this structure for most coding tasks:

- `Task`
- `Goal`
- `Change Scope`
- `Files to update`
- `Behavior to preserve`
- `Specific additions`
- `Constraints`
- `Verification`
- `Gate Awareness`

## Recommended Template
```text
Task:

Goal:

Change Scope:
(minimal | moderate | invasive)

Files to update:

Behavior to preserve:

Specific additions:

Constraints:

Verification:

Gate Awareness:
(optional - impact on rho, gamma, delta, tau)
```

## Guidance For Each Field
### Task
State the implementation action in one clear line.

Examples:

- `Add a live decision stream to Vitals`
- `Tighten the holographic beam geometry`
- `Stabilize AUTO theme switching`

### Goal
Describe the intended system outcome, not just the visual change.

Examples:

- `Make transport and voice history feel persistent across refresh`
- `Make the orb reflect instability without changing governance logic`

### Files to Update
Name the files directly when known.

Examples:

- `hud.html`
- `hud_style.css`
- `hud_script.js`
- `cipher_local_api.py`

### Behavior to Preserve
Call out the stable pieces that must not break.

Examples:

- `Do not break Dev Sandbox or Live Mirror`
- `Do not change backend governance behavior`
- `Keep terminal parity intact`

### Specific Additions
List the exact new elements, functions, fields, or UI controls you want.

Examples:

- `Add a Global Timeline panel`
- `Add a theme selector with manual and auto controls`
- `Add a Live -> Sandbox promotion button`

### Constraints
State execution boundaries clearly.

Examples:

- `Frontend only`
- `No backend changes`
- `No file writes from preview mode`
- `No unnecessary libraries`
- `Preserve existing routing and observability`

### Verification
Say how success should be checked.

Examples:

- `Restart 5000 and 5001 and verify health`
- `Refresh HUD and verify the theme changes live`
- `Smoke test read -> search -> patch -> test`

### Change Scope
Declare how broad the modification should be.

Values:

- `minimal` : small patch, local change, lowest drift
- `moderate` : feature-level change across a few files
- `invasive` : large refactor, architecture move, or intentionally broad rewrite

Why it matters:

- reduces over-editing
- protects stable surfaces
- aligns implementation work with drift control

### Gate Awareness
Use this when the requested work affects governed behavior or system interpretation.

Examples:

- `visual only - no impact on rho, gamma, delta, tau`
- `delta display changes only - no gate logic change`
- `affects delta-lite pause behavior`
- `introduces future tau-related sequencing logic`

## Preferred Style For Echo Nexus Work
Inside this habitat, prefer:

- targeted in-place patches
- minimal surface-area change
- preserved stable behavior
- clear verification steps

Do not default to:

- full-file rewrites
- broad refactors without need
- changes that mix visual work with governance changes unless explicitly requested

## Example Prompt
```text
Task:
Add a persistent Global Timeline panel.

Goal:
Unify voice, transport, and governance history into one visible system memory surface.

Change Scope:
moderate

Files to update:
- hud.html
- hud_style.css
- hud_script.js
- cipher_local_api.py

Behavior to preserve:
- Do not break Voice Lab
- Do not break Mesh / transport controls
- Do not change Dev logic

Specific additions:
- Add a timeline panel in Vitals
- Add filter controls for domain and decision
- Add GET /api/events to merge existing ledgers

Constraints:
- Keep persistence model consistent: ledger = truth, HUD = view
- Clear should clear only the visible panel
- No unnecessary new dependencies

Verification:
- Restart ports 5000 and 5001
- Verify /api/events returns merged history
- Refresh HUD and confirm filters work

Gate Awareness:
visualization only - no change to gate thresholds or execution policy
```

## Echo Nexus Master Context Prompt
For a new coding session, this compressed context is useful:

```text
You are working inside Echo Nexus, a governed AI runtime interface.

Execution flow:
input -> gate -> route -> execute -> log -> reflect

Governance:
- rho (rho) = confidence
- gamma (gamma) = alignment
- delta (delta) = drift

Default gate rules:
- PROCEED if rho >= 0.70 and gamma >= 0.70 and delta <= 0.30
- PAUSE if delta > 0.30
- ABORT if delta > 0.40 or gamma < 0.65

Rules:
- no ungated execution
- no silent execution
- all actions must be observable
- all meaningful actions must produce ledger outcomes

Prefer minimal in-place changes.
Preserve stable behavior unless explicitly modified.
Wait for task.
```

## Practical Note
When working with Codex in the live habitat, asking for "full updated file" is often unnecessary. It is usually better to ask for:

- precise edits
- preserved behavior
- direct verification

That keeps iteration faster and reduces unrelated drift.

## Echo-Native Prompt Contract
For Echo Nexus work, the strongest reusable prompt format is:

```text
Task:
Goal:

Change Scope:
(minimal | moderate | invasive)

Files to update:

Behavior to preserve:

Specific additions:

Constraints:

Verification:

Gate Awareness:
(optional - impact on rho, gamma, delta, tau)
```
