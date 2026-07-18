# EMBODIED_INTERFACE Plan Review

## Overview
EMBODIED_INTERFACE is the Annunimas physical presence and embodied display planning surface. It covers the Pi5 / Pepper's Ghost path, CITADEL Companion, RELIC visualization, runtime-truth-bound visual doctrine, and future gated voice/physical interaction boundaries.

The current quick reference is `core/projects/Plans/EMBODIED_INTERFACE.md`. It points to the CITADEL Companion roadmap and runtime state surfaces rather than a separate human plan file.

## Core Runtime Surfaces
The reviewed contract is represented by these primary surfaces:

- `core/projects/Plans/EMBODIED_INTERFACE.md` — quick reference and plan pointer
- `docs/plans/2026-05-27-citadel-companion-embodied-roadmap.md` — current operator-facing CITADEL Companion roadmap
- `core/state/embodied_interface.json` — embodied interface design/runtime mapping
- `core/state/tauri_embodiment.json` — rendering doctrine and stack guidance
- `core/edge/targets.toml` — edge target inventory
- `/var/home/mythos/Eregion/relic-kiosk/README.md` — active RELIC projection visualization context
- `docs/contracts/citadel-voice-physical-interaction-safety-contract.md` — future voice/physical interaction safety boundary

## Current Contract
EMBODIED_INTERFACE currently owns:

1. **Runtime-truth-bound embodiment**: animation and visual state must derive from live runtime state, queue pressure, agent heartbeat, or explicit presence payloads rather than decorative timers.
2. **Physical display doctrine**: black-field projection, centered luminous subject, Pepper's Ghost legibility, and sacred-geometry / platonic-solid visual language.
3. **CITADEL Companion boundary**: a small Pi-safe display client that answers what the embodied surface should show right now, without becoming a reasoning engine, model router, or queue mutation surface.
4. **RELIC experimental lane**: a separate projection-illusion lab for kinetic sacred-geometry visualization while keeping the stable companion runtime protected.
5. **Future capability gates**: microphone, camera, wake-word, spoken output, physical sensors, device commands, external sends, service control, credentials, and queue mutations remain disabled unless a later implementation packet carries human approval, WARDEN review, and durable receipts.

## Observed Runtime / Plan State
The reviewed surfaces show the embodied interface is present as a documented and partially deployed physical visualization lane:

- `core/state/embodied_interface.json` declares schema `annunimas.embodied-interface.v1`, hardware targets for Pi5 guardhouse, Pi5 CITADEL avatar controller, and Pepper's Ghost enclosure, plus realm-to-geometry visual mapping.
- `core/state/tauri_embodiment.json` declares schema `annunimas.tauri-embodiment.v1`, preferred stack guidance around Tauri / Vite React / Three.js / react-three-fiber / Theatre.js, and explicitly requires event-driven runtime-truth binding.
- The CITADEL Companion roadmap is `completed-queue-backed` through visual projection, state normalization, ARDA/CITADEL presence bridging, shared presence schema, and RELIC forward visualization evidence.
- RELIC is documented as the active forward CITADEL Pi visualization as of 2026-05-29, with `relic.service` on port `8091` and `citadel-companion.service` disabled during RELIC deploys.
- Fleet/edge readiness remains operator-sensitive: `node-pi5-citadel-avatar` identity and recovery failures are represented as operator-confirmation or human-gated work rather than autonomous mutation.

## Implementation Status

### Completed / Present
- Quick reference exists at `core/projects/Plans/EMBODIED_INTERFACE.md`.
- Current CITADEL Companion roadmap exists at `docs/plans/2026-05-27-citadel-companion-embodied-roadmap.md`.
- Runtime embodiment contract exists at `core/state/embodied_interface.json`.
- Rendering doctrine exists at `core/state/tauri_embodiment.json`.
- RELIC documentation exists at `/var/home/mythos/Eregion/relic-kiosk/README.md`.
- Voice and physical interaction safety contract exists at `docs/contracts/citadel-voice-physical-interaction-safety-contract.md`.

### Degraded / Blocked
- CITADEL Pi fleet recovery and canonical edge identity binding require operator confirmation before autonomous bootstrap or service mutation.
- The physical device lane is environment-dependent; live Pi/service claims require fresh remote validation before being treated as current operational truth.
- Future voice, microphone, camera, wake-word, sensors, spoken output, and device-command work is explicitly gated and not active runtime capability.

### Follow-up Work
1. **Edge identity confirmation**
   - Confirm canonical identity for `node-pi5-citadel-avatar` before additional embodied bootstrap.
   - Separate SSH/Tailscale recovery failures from ordinary app/runtime defects.

2. **Runtime projection hardening**
   - Keep embodied state driven by compact ARDA/CITADEL projection contracts.
   - Preserve provenance and fail-closed behavior when projection payloads are missing, stale, or malformed.

3. **RELIC / companion boundary maintenance**
   - Keep RELIC as the experimental projection lane until its scene contract and physical behavior are proven stable.
   - Keep `citadel-companion` available as the stable fallback surface.

4. **Future gated interaction design**
   - Treat voice, microphones, cameras, wake-word, sensors, and device commands as separate WARDEN/human-reviewed implementation packets.
   - Require explicit disabled-by-default behavior, approval receipts, and audit logs before any device-side action capability is enabled.

## Verification Commands
Useful focused checks for this plan surface:

```bash
python -m json.tool core/state/embodied_interface.json >/dev/null
python -m json.tool core/state/tauri_embodiment.json >/dev/null
scripts/check_task_queue_append_only.sh
```

Live CITADEL Pi status requires fresh operator-environment checks before claims:

```bash
cd /var/home/mythos/Eregion/relic-kiosk && npm run validate
cd /var/home/mythos/Eregion/relic-kiosk && npm run deploy:citadel
bash scripts/deploy_citadel_companion.sh
```

Run remote deploy or service mutation only when the active task carries explicit operator authority.

## Alignment with Annunimas Principles
- **Evidence-first embodiment:** visual behavior is tied to runtime truth, projection payloads, and receipts rather than decorative motion.
- **Safety-gated physical presence:** physical sensors, microphone/camera, wake-word, service control, credentials, and queue mutation remain human-gated.
- **Operator clarity:** CITADEL is a physical display/presence surface, while ARDA/Hermes/CLI remain the richer inspection and action surfaces.
- **Separation of stable and experimental lanes:** stable companion and RELIC experimental visualization stay distinct.

## Open Questions
1. What is the canonical edge identity for `node-pi5-citadel-avatar` after fleet recovery drift?
2. When should RELIC graduate from experimental projection lane to stable companion default, if ever?
3. Which future voice or physical interaction capability should be scoped first under the safety contract?

## References
- Quick reference: `core/projects/Plans/EMBODIED_INTERFACE.md`
- CITADEL Companion roadmap: `docs/plans/2026-05-27-citadel-companion-embodied-roadmap.md`
- Embodied runtime state: `core/state/embodied_interface.json`
- Tauri embodiment guidance: `core/state/tauri_embodiment.json`
- RELIC README: `/var/home/mythos/Eregion/relic-kiosk/README.md`
- Voice / physical interaction safety: `docs/contracts/citadel-voice-physical-interaction-safety-contract.md`
