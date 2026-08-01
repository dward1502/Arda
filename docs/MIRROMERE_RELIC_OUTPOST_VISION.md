# Mirromere and Relic: Embodied Outpost Vision

**Status:** Active architectural vision; implementation is intentionally incremental and safety-gated  
**Scope:** Arda core, modular Rust outposts, ambient assistance, embodied visualization, and health-support boundaries  
**Supersedes:** No existing contract. This document carries forward the useful intent from the outdated `docs/plans/EMBODIED_INTERFACE.md` while separating verified current state from future design.

## Vision

Arda should become a dependable base system that can support independently deployable **outposts**: modular Rust crates and applications that connect to the main system through versioned contracts while owning their own hardware, tools, local data, and failure behavior.

Two planned outposts express this direction:

- **Mirromere** — a two-way-mirror ambient assistant with a display, local camera, microphone, speaker, environmental sensing, and opt-in biometric integrations. Its purpose is to reduce day-to-day executive burden, support self-observation, and provide a calm interface for personal routines and health information.
- **Relic** — a Pepper's Ghost projection surface that renders live Arda runtime activity as luminous geometric forms. It makes agent presence, collaboration, health, and handoffs spatially legible without pretending that decorative animation is runtime truth.

These are not alternate Arda cores. They are embodied projections and capability-bearing edge systems governed by Arda's identity, evidence, consent, routing, and receipt boundaries.

## Human Context and Product Intent

Mirromere is motivated by the practical needs of living with heart-transplant care and AuDHD accessibility requirements. The desired system is not a novelty dashboard. It should help manage the shape of daily life while preserving agency:

- lower the activation energy for medication, appointment, hydration, sleep, nutrition, and household routines;
- present one clear next action when a larger plan becomes cognitively expensive;
- support predictable morning, transition, and evening check-ins;
- adapt information density, sound, lighting, and interruption behavior to sensory and attentional state;
- collect longitudinal observations without turning every deviation into an alarm;
- summarize patterns for personal review and optional clinician sharing;
- provide an embodied place to check validated vitals and record subjective internal state;
- make privacy, silence, and temporary disconnection immediate physical choices.

The intended relationship is **supportive and reflective**, not supervisory. Mirromere may notice, remind, summarize, and recommend verification. It must not shame, coerce, diagnose, prescribe, or silently escalate its authority.

## Verified Foundation Already Present

The repository and external Relic prototype already contain parts of the embodied-outpost foundation.

### Pi5 outpost role map and live boundary

The Pi5 systems are not disposable display targets. They are named Arda outposts
with complementary roles:

- **Warden (`node-pi5-warden`)** is the always-on guardhouse, lightweight local
  inference lane, bounded internet/repository scout, and advisory council
  evidence node. Scout outputs enter the normal Athena/council evidence path;
  Warden does not independently approve autonomous actions.
- **CITADEL (`node-pi5-citadel-avatar`)** is the embodied avatar and chat
  presentation outpost, the RELIC runtime-presence display, a future ARDA HUD
  companion projection, a bounded scout dispatcher, and an advisory council
  presence. Its council role presents state and contributes receipted evidence;
  it does not gain execution authority merely because it renders a council.

Live verification on 2026-07-23 established:

- `ssh warden` reaches `numenor@100.110.85.37`; key auth, passwordless sudo,
  linger, and `llama-server.service` are operational;
- `ssh citadel` and `ssh raspberrypi` reach `citadel@100.119.130.127`; key auth,
  passwordless sudo, and linger are operational;
- CITADEL runs `relic.service` on port `8091` and
  `citadel-kiosk.service` against `http://127.0.0.1:8091/`;
- no CITADEL scout worker, chatbot bridge, council worker, or ARDA HUD companion
  service is deployed yet. Those are implementation gaps, not abandoned ideas.

The first ARDA HUD integration should be a Pi-safe companion projection fed by
the same bounded state bundle as the desktop HUD, not an attempt to run the full
Tauri operator cockpit on the round display. Interactive chat and scout requests
should traverse Manwe and the existing governance/receipt contracts; internet
tools remain allowlisted central capabilities rather than unrestricted Pi shell
access.

### Arda embodiment contracts

`core/state/embodied_interface.json` currently describes:

- Raspberry Pi 5 guardhouse and CITADEL avatar-controller targets;
- a Pepper's Ghost avatar enclosure;
- realm-to-color, geometry, and motion mappings;
- an explicit requirement that animation derive from runtime state, queue pressure, agent heartbeat, or other evidence rather than decorative timers;
- a fail-soft idle resonance glow when runtime projection data is unavailable.

`core/state/tauri_embodiment.json` currently records:

- Rust/Tauri as the backend direction;
- Vite/React with Three.js and react-three-fiber as the preferred scene stack;
- event-driven runtime binding;
- Pepper's Ghost as a projection shell over the same runtime contract;
- software-rendering and WebGPU-aware deployment considerations.

Both state files still carry legacy `annunimas.*` schema identities. They are useful evidence of prior design and implementation direction, but should be versioned into canonical `arda.*` schemas rather than silently reinterpreted.

### Relic prototype

The external prototype at `/var/home/mythos/Eregion/relic-kiosk` currently documents and implements:

- a black-field Three.js Pepper's Ghost scene;
- sacred-geometry particle identities for Charon, Athena, and Warden;
- up to three active agents and a fused council form;
- local `annunimas.relic.scene.v1` scene-state normalization;
- scripted and static modes;
- configurable fusion, brightness, vertical offset, and scale;
- a static kiosk deployment path targeting port `8091`;
- separation from the stable CITADEL companion lane.

The prototype README records a previously operator-approved Pi deployment baseline. That is historical deployment evidence, not proof of current live service state; the Pi must be checked before any new operational claim.

Relic's first prototype intentionally has no camera, microphone, voice, humanoid rig, or direct Arda runtime bridge. That separation remains a good safety boundary.

### Environmental governance groundwork

`arda-governance` now contains an advisory environmental signal layer:

- typed audio, vision, and solar/geomagnetic signal envelopes;
- timestamps, freshness, confidence, measurement quality, and degraded/unavailable states;
- pooled and bounded NOAA Kp/Dst collection with timeout and cache behavior;
- advisory environmental coherence that cannot approve, reject, or block work;
- in-process telemetry and an optional Varda executor-receipt projection.

This is a useful precursor for Mirromere, but it is not a health model. Environmental data must remain contextual evidence with disclosed uncertainty.

### Known stale references in the older plan

The outdated `docs/plans/EMBODIED_INTERFACE.md` references these paths, which are not currently present in this checkout:

- `docs/plans/2026-05-27-citadel-companion-embodied-roadmap.md`
- `docs/contracts/citadel-voice-physical-interaction-safety-contract.md`

Their claimed boundaries should not be treated as active contracts until equivalent canonical Arda documents are restored or rewritten.

## Outpost Architecture

### Core principle

An outpost is an independently deployable edge capability that communicates with Arda through narrow, versioned messages. It must remain inspectable and safe when Arda, the network, a sensor, or a model is unavailable.

### Arda core owns

- outpost identity and enrollment;
- capability declarations and allowlists;
- policy evaluation and authority boundaries;
- semantic event routing;
- consent, approval, and action receipts;
- provenance and schema-version checks;
- runtime health and observability contracts;
- revocation, quarantine, and degraded-state representation.

### Each outpost owns

- local hardware adapters and device drivers;
- local scene rendering and interaction loops;
- sensor sampling and short-lived raw buffers;
- explicit local data-retention policy;
- hardware-specific calibration;
- offline and degraded behavior;
- outpost-local tools that are not general Arda capabilities;
- a physical privacy and shutdown path.

### Proposed crate/application boundary

These names are proposed, not claims about existing crates:

```text
crates/outposts/arda-outpost-protocol/   # shared manifests, events, receipts, health
crates/outposts/arda-mirromere/          # orchestration and ambient interaction
crates/outposts/arda-mirromere-health/   # validated-device adapters and health provenance
crates/outposts/arda-mirromere-sensing/  # camera, audio, environment, local feature extraction
crates/outposts/arda-relic-bridge/       # read-only runtime-to-scene projection
apps/mirromere/                           # mirror UI and device deployment surface
apps/relic/                               # canonical Arda-owned projection application, if migrated
```

The shared protocol crate should remain small. It should not absorb camera frameworks, clinical-device SDKs, Three.js assets, or outpost-specific inference logic.

## Shared Data Contracts

Every observation should distinguish raw measurement, derived estimate, self-report, default, and unavailable state.

### `OutpostManifest`

Declares:

- outpost ID, device class, and software version;
- supported input/output capabilities;
- whether each capability is read-only, advisory, confirmation-gated, or prohibited;
- local retention policy and remote-projection policy;
- current calibration and schema compatibility;
- physical privacy controls present on the device.

### `SensorObservation`

Carries:

- source and device identity;
- source timestamp and collection timestamp;
- freshness and confidence;
- measurement/derived/defaulted/unavailable classification;
- calibration reference;
- units and valid range;
- local-only/raw-data flags;
- optional derived feature payload.

### `HealthObservation`

Adds:

- clinical, consumer-wellness, experimental, or self-reported evidence class;
- validated-device identity when applicable;
- body site and measurement protocol when relevant;
- whether the value is suitable for display, trend analysis, reminders, or escalation;
- an explicit prohibition against medication or diagnostic authority unless a future independently reviewed clinical integration contract exists.

### `RuntimePresenceEvent`

Provides Relic with sanitized runtime truth:

- agent/realm identity;
- event kind and lifecycle state;
- health, confidence, and freshness;
- task or handoff correlation ID without private prompt content;
- collaboration edges;
- bounded resource-pressure projection;
- provenance receipt reference.

### `AmbientSceneCommand`

Describes what an embodied display may render without granting it general action authority:

- scene ID and purpose;
- visual density and sensory intensity;
- allowed audio behavior;
- duration and expiry;
- source event/decision references;
- fallback scene;
- whether operator acknowledgement is requested.

## Mirromere Functional Model

### Calm daily-life assistance

Mirromere should favor low-friction, low-stimulation interactions:

- one next action rather than an unbounded task list;
- visual-first prompts with optional voice;
- stable placement and predictable scene transitions;
- configurable quiet hours and recovery modes;
- graduated reminders with a hard maximum, not infinite escalation;
- easy deferral that records context without moral judgment;
- routines shaped around personal baselines rather than population averages.

### Reflective check-ins

A check-in may combine:

- validated wearable or medical-device readings;
- manually entered blood pressure or temperature;
- medication confirmation;
- sleep and fatigue report;
- pain, dizziness, breathlessness, mood, or cognitive-load self-report;
- ambient light, sound, temperature, and environmental context;
- optional local camera/audio-derived features.

The output should be a timestamped personal record and a concise trend explanation—not a diagnosis.

### Camera-derived physiology

Facial video can support experimental remote photoplethysmography for pulse and related waveform features under controlled conditions. Camera-only blood-pressure estimation is not a substitute for a validated cuff and should not be represented as a clinical measurement.

For transplant-aware use:

- validated devices are the source of record;
- camera-derived pulse or blood-pressure estimates are explicitly `experimental_derived`;
- estimates require confidence, lighting, motion, skin-region quality, calibration, and freshness metadata;
- an estimate may recommend taking a validated measurement;
- it cannot alter medication, suppress an alert, or overrule a validated device;
- symptoms or transplant-team instructions always take precedence over model output.

### Audio and behavior context

Local audio and camera processing may identify coarse interaction conditions—speech present, noise level, posture change, prolonged inactivity, or unusual routine timing—but should avoid covert psychological profiling. Raw streams should remain ephemeral by default. Long-term records should store the smallest useful derived feature and disclose how it was produced.

Arda must not infer a clinical diagnosis from typing, voice, expression, or routine patterns.

## Relic Runtime-Truth Doctrine

Relic should remain a read-only observability outpost. Its visual language may be expressive, but each state transition must correspond to actual runtime evidence.

A possible mapping is:

- **shape** — stable agent, role, or realm identity;
- **color** — canonical realm or subsystem;
- **brightness** — health or confidence, within a sensory-safe range;
- **motion** — active work backed by a lifecycle event;
- **orbit/proximity** — collaboration or handoff relationship;
- **trail** — provenance-linked transition;
- **scale** — normalized bounded workload or resource pressure;
- **fracture/jitter** — degraded or blocked state;
- **dim unresolved form** — stale, missing, or uncertain telemetry;
- **fusion** — a real multi-agent council/session, never an arbitrary timer.

Relic must not display private prompt text, credentials, hidden chain-of-thought, or fabricated internal states. When telemetry is absent, the scene should visibly become unknown or idle rather than continuing a convincing fiction.

## Privacy, Consent, and Physical Safety

### Local-first defaults

- Camera and microphone are off or locally processed by default.
- Raw video/audio is not persisted unless a time-bounded recording mode is explicitly activated.
- Derived health and behavior observations remain local unless individually allowed for projection or export.
- Cloud inference is opt-in per capability, not one blanket setting.
- Sensitive observations never become ordinary Arda task labels or broadly replicated fleet state.

### Physical controls

Mirromere should have controls that do not depend on software correctness:

- camera shutter;
- microphone hardware mute;
- speaker mute or volume limit;
- visible sensor-active indicator driven by electrical state where possible;
- immediate privacy scene;
- local power/network isolation option;
- accessible manual mode when speech or camera interaction is undesirable.

### Authority ladder

1. **Observe locally** — collect an explicitly enabled signal.
2. **Reflect** — display or summarize it to the operator.
3. **Advise** — suggest a reversible next step.
4. **Ask confirmation** — prepare but do not execute an external action.
5. **Execute a narrow allowlisted action** — only with a durable receipt and revocation path.
6. **Prohibited by default** — diagnosis, medication changes, transplant-care decisions, silent emergency dismissal, unrestricted device control, or broad external disclosure.

Emergency behavior should follow clinician-authored and operator-approved instructions. Mirromere may help present those instructions and contact options, but it is not an emergency medical service.

## Environmental and Heliophysical Context

Solar, geomagnetic, cosmic-ray, acoustic, lighting, weather, and local magnetic observations may be recorded as exploratory environmental context. Their inclusion should support personal longitudinal research, not imply established causality.

Any such analysis should:

- preserve the original source and units;
- separate measured values from derived indices;
- include freshness and confidence;
- account for season, daylight, weather, sleep, medication timing, and routine changes as confounders;
- test lagged relationships rather than selecting correlations after the fact;
- use null controls and multiple-comparison correction;
- remain advisory even when a correlation appears personally meaningful.

Useful future sources may include Kp, Dst, IMF Bz, solar-wind speed/density, F10.7 radio flux, local magnetometer readings, and neutron-monitor counts. Each source should be implemented independently and degrade to unavailable without blocking Mirromere.

## Failure and Degraded Modes

- **Arda unavailable:** local clock, validated-device display, manual check-in, and privacy controls continue.
- **Network unavailable:** no repeated noisy retries; queue only explicitly allowed outbound records.
- **Camera/microphone unavailable:** show unavailable state and continue without synthetic substitutes.
- **Health-device disagreement:** display sources separately and request validated remeasurement; do not average incompatible readings.
- **Stale runtime telemetry:** Relic transitions to visibly stale/idle state.
- **Model unavailable:** deterministic routines and manual controls remain functional.
- **Schema mismatch:** reject the payload with a visible compatibility reason and retain the last known safe scene only within its expiry.
- **Storage pressure:** preserve consent and critical receipts before optional media or derived history.

## Incremental Delivery Path

### Phase 0 — Canonicalize embodied contracts

- Inventory the actual Pi, Relic, CITADEL, and Arda surfaces.
- Replace legacy `annunimas.*` embodied schema identities with versioned `arda.*` successors while preserving migration readers.
- Restore or rewrite the missing physical-interaction safety contract.
- Define an outpost manifest and read-only enrollment state.

**Exit evidence:** schema fixtures, compatibility tests, current hardware inventory, and no implied live-Pi claims without verification.

### Phase 1 — Relic read-only bridge

- Define `RuntimePresenceEvent` and scene-projection contracts.
- Add a Rust read-only bridge that consumes sanitized Arda runtime events.
- Replace scripted activity with evidence-backed events while retaining scripted mode as a labeled demo.
- Represent stale, degraded, and unknown telemetry visually.

**Exit evidence:** deterministic event-to-scene tests, recorded fixture playback, and no mutation capability in the Relic bridge.

### Phase 2 — Mirromere display shell

- Build the mirror-safe high-contrast scene shell.
- Add clock, agenda, medication schedule display, manual routine acknowledgement, and privacy mode.
- Implement sensory profiles, quiet hours, and predictable transitions.
- Keep camera, microphone, health inference, and external actions disabled.

**Exit evidence:** local kiosk operation, keyboard/touch/physical-control accessibility, restart recovery, and offline behavior tests.

### Phase 3 — Validated-device health ledger

- Integrate one validated device or standards-based export at a time.
- Add typed health observations, calibration/provenance, manual correction, and retention policy.
- Produce personal trend summaries without diagnostic language.
- Add explicit export review before clinician sharing.

**Exit evidence:** fixture-backed ingestion, unit validation, stale/duplicate handling, and source-visible trend output.

### Phase 4 — Local ambient sensing

- Add hardware camera shutter, microphone mute, and sensor-state indicators first.
- Introduce ephemeral local audio/light/environment features.
- Add opt-in camera-derived pulse research behind an experimental evidence class.
- Keep camera-derived blood pressure out of clinical displays.

**Exit evidence:** privacy-control tests, raw-buffer expiration, no-network operation, and clearly marked experimental outputs.

### Phase 5 — Day-management assistance

- Add routine planning and one-next-action projection.
- Learn operator-approved preferences without silently converting behavior into diagnoses.
- Add reversible action preparation with explicit confirmation.
- Evaluate reminder burden, false escalation, and sensory interruption.

**Exit evidence:** receipt-backed actions, bounded reminder behavior, preference export/delete controls, and operator-reviewed longitudinal results.

### Phase 6 — Personal environmental research

- Add additional heliophysical and local environmental adapters independently.
- Define preregistered personal hypotheses and confounder fields.
- Run retrospective advisory analysis without feeding correlations into action authority.

**Exit evidence:** reproducible notebooks/reports, null-result preservation, and explicit non-clinical interpretation.

## Non-Goals

- Replacing a transplant team, clinician, validated cuff, or emergency service.
- Inferring diagnosis or intent from ambient behavior.
- Persisting continuous household surveillance.
- Giving Relic access to private reasoning content or mutation tools.
- Letting environmental or biometric correlations become undisclosed governance gates.
- Building every sensor and action into Arda core.
- Requiring network or model availability for basic mirror operation.

## Architectural Decisions to Preserve

1. Arda must be stable before outposts gain meaningful authority.
2. Outposts are modular capability domains, not miscellaneous UI plugins.
3. Raw sensitive data remains at the edge unless explicitly released.
4. Every observation discloses provenance, quality, confidence, and freshness.
5. Every external action has a narrow capability, confirmation policy, and receipt.
6. Relic renders runtime truth; unknown state is allowed and visible.
7. Mirromere assists personal agency; it does not become a medical or behavioral authority.
8. Imaginative embodiment and strict epistemic discipline are compatible requirements.

## Open Questions

1. Which validated health device or export format should be integrated first?
2. Which Mirromere interactions should be touch, gesture, voice, or physical-button driven?
3. What raw-buffer lifetimes are acceptable for camera and microphone processing?
4. Should Mirromere have a physically separate health-data store from general Arda memory?
5. Which runtime event source should become Relic's first canonical live bridge?
6. Should the external `relic-kiosk` remain independent or migrate into an Arda app after the protocol stabilizes?
7. Which clinician-authored thresholds or instructions, if any, should be displayable without granting Arda interpretive authority?
8. How should sensory profiles be selected without requiring the system to infer emotional state?

## References

- `docs/plans/EMBODIED_INTERFACE.md` — prior embodied-interface narrative and historical status
- `core/state/embodied_interface.json` — current legacy embodiment mapping
- `core/state/tauri_embodiment.json` — current legacy rendering doctrine
- `crates/spine/governance/arda-governance/src/environmental.rs` — typed advisory environmental signals
- `crates/spine/governance/arda-governance/src/solar.rs` — bounded NOAA geomagnetic collection
- `/var/home/mythos/Eregion/relic-kiosk/README.md` — external Relic prototype and historical deployment record
