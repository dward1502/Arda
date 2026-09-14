---
soterion:
  sigil: "LANTERN"
  role: "design_reference_research"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 🏮 Sci-fi interface reference research | owner: HERMES | status: active | reviewed: 2026-08-16

# ARDA HUD Lower Workstations — Sci-Fi Interface Reference Research

## Purpose

This record translates external sci-fi interface references into bounded design rules for ARDA. It is not a license to copy artwork, import decorative assets, or replace ARDA's existing information architecture with generic FUI styling.

The research follows the completed lower-surface audit and cross-reference. Repository code and ARDA runtime contracts remain authoritative. External references contribute visual and interaction principles only.

Pinterest search results were reviewed as discovery material, but Pinterest did not provide a stable extractable source. The durable reference set therefore links directly to artist, studio, ArtStation, and Behance project pages.

## Reference set

### 1. Marcel Core Control Panel

[ArtStation project][1]

Observed characteristics:

- a wide screen is physically inset into a beveled housing rather than floating as a flat web page;
- the chassis has edge lights, handles, seams, recesses, and controls that communicate scale and touch;
- the screen uses one dominant radial instrument, several secondary graphs, and dense diagnostic text;
- cyan is the normal-state structural color, while small amber marks carry attention;
- hierarchy is produced by scale, placement, and luminance rather than by repeated cards.

Use for ARDA:

- inset Command Core display and front-plate utility controls;
- visible separation between instrument glass, command buttons, and console body;
- one dominant live signal with a limited number of supporting readings;
- edge illumination as state, not decoration.

Do not copy:

- unreadably small filler text;
- equal visual weight for every diagnostic;
- a single universal dark-cyan treatment for all workstations.

### 2. Evgeny Rodygin Transceiver

[ArtStation project][2]

Observed characteristics:

- the screen follows the physical device's irregular outline;
- one large acoustic visualization and one large circular control form the primary reading;
- a compact keypad/grid occupies a distinct secondary zone;
- a restrained amber-on-black palette makes the device feel specialized rather than generic;
- the hands and chassis make interaction scale and reach obvious.

Use for ARDA:

- task-shaped, asymmetric workstation compositions;
- large targetable regions instead of tiny conventional controls;
- physical control banks placed where a seated operator can plausibly reach them;
- domain-specific accent palettes and instruments.

Do not copy:

- ambiguous unlabeled touch regions for consequential actions;
- decorative rings that do not encode a value or state;
- high-density orange everywhere.

### 3. Territory Studio — Blade Runner 2049

Territory describes producing more than 100 screen assets across 15 sets and designing each interface around specific storybeats and contexts.[3] The studio deliberately explored physical and organic alternatives to conventional digital screens, including optical lenses, projectors, microfiche, and scanned materials.[3]

Observed characteristics from the cockpit material:

- cyan structure, amber sensing, and red fault marks are sparse and meaningful;
- imperfect projection, ghosting, texture, and degradation communicate the age and condition of the machine;
- a large central sensed form is framed by edge scales and a handful of strong numeric readings;
- the interface is understandable at a glance even when the underlying data is technically styled.

Use for ARDA:

- source freshness and runtime degradation may affect texture, stability, or confidence marks;
- stale and unavailable data should visibly alter the instrument instead of silently leaving old values present;
- lower screens should use a central domain signal with peripheral supporting status;
- physical context should determine the interface form.

Do not copy:

- simulated damage that could be mistaken for application failure;
- reduced legibility solely for atmosphere;
- uncontrolled scanline, bloom, or glitch effects that harm performance.

### 4. R▲ Device UI

The project is a simulated glass-projection interface informed by 1980s CRT controls, Soviet aircraft HUDs, and vector displays.[4] The designer previewed work in its intended physical medium and iterated fonts and shapes for readability under different lighting conditions.[4]

Observed characteristics:

- single-line vector language keeps the interface light and instrument-like;
- nested frames disclose detail progressively without turning the entire surface into a window manager;
- large areas of darkness create focus and make a small active region feel intentional;
- small red markers provide orientation and state without becoming a full accent theme;
- layout follows a staged interaction sequence rather than a persistent dashboard grid.

Use for ARDA:

- progressive disclosure from lower instrument to focused workstation to evidence detail;
- vector framing and sparse anchors for tactical displays;
- testing in the actual Three.js/WebKitGTK presentation, not only a browser screenshot;
- restrained active regions surrounded by deliberate negative space.

Do not copy:

- very small edge labels as primary navigation;
- simulated glass interaction where a physical click target is required;
- sparse composition without enough state to support a real decision.

### 5. Andrew Sullivan Sci-Fi UI Explorations

The ArtStation set includes animated and static experiments ranging from sparse central instruments to denser operational displays.[5]

Observed characteristics:

- strong central symmetry can make a single machine process immediately legible;
- thin perimeter lines and tiny corner identifiers create a bounded instrument without card chrome;
- a sparse screen works when it answers one question, such as process state or machine integrity;
- dense variants become harder to scan when many equal-weight widgets compete.

Use for ARDA:

- Fleet topology, Routing flow, and Command Core coherence may each use one dominant geometric field;
- persistent corners can carry identity, freshness, and source state;
- reserve dense diagnostic layouts for focused detail, not lower monitors.

Do not copy:

- ornamental radial motion without system meaning;
- headings or numbers too small for the physical camera distance;
- generic circles reused for every domain.

## Process references

### Build function before finish

Cloud Imperium's UI process begins with gameplay needs and simple flow wireframes before visual styling.[6] It then moves through broad mood-board references, fast concept variants, a selected visual target, and a compact style guide.[6] The same account describes using a mostly pre-rendered background with limited animated regions to add life without excessive runtime cost.[6]

ARDA translation:

1. establish each workstation's decision and action flow;
2. wire existing ARDA source families and truth states;
3. sketch the task-specific layout;
4. approve one visual target per surface class;
5. implement restrained motion only where it communicates state;
6. verify on the native runtime and preserve the frame-rate gate.

### Technical credibility must remain readable

Territory Studio describes believable FUI as a balance between technical minimalism and the audience's ability to quickly follow the relevant point.[7] The studio grounds interfaces in real military or scientific references, then coordinates visual language, color, iconography, visualization, motion, and focal framing around context.[7]

ARDA translation:

- a visualization is valid only when its geometry maps to real fields;
- every screen has one primary reading;
- supporting labels explain state rather than decorate it;
- visual complexity belongs where operational complexity actually exists;
- source truth and action consequences remain explicit.

## ARDA visual grammar

### Shared substrate

All lower surfaces may share:

- console material and inset-screen construction;
- thin vector lines and restrained glow;
- a standard truth-state language;
- fixed minimum type and target sizes at the boardroom camera distance;
- source/freshness markers in a consistent corner location;
- motion budgets and reduced-motion behavior;
- standard focus, hover, press, disabled, blocked, and unavailable states.

They should not share one card template, one radial chart, or one tab stack.

### Truth-state language

| State | Visual treatment | Meaning |
|---|---|---|
| live | stable cyan/mint trace with timestamp cadence | direct current runtime observation |
| snapshot | steady line with a bounded age tick | valid stored observation |
| projected | dashed or offset trace | derived from another source |
| stale | amber persistence trail and explicit age | value exists but exceeds freshness policy |
| unavailable | dark instrument with retained frame and named missing source | adapter or service cannot currently supply data |
| missing | open circuit/broken path mark | expected source artifact is absent |
| loaded but unused | development/evidence-only marker, never shown as live | data exists in memory but has no owning renderer |
| blocked | red guard notch plus reason | action is intentionally prevented |
| receipt pending | pulsing amber witness mark | action dispatched but not yet proven |
| proven | brief mint latch then steady state | terminal receipt verifies completion |

No state may be conveyed by color alone.

### Motion grammar

- continuous motion must represent a changing value, active process, or acquisition sweep;
- idle surfaces settle rather than constantly pulse;
- unavailable data stops or breaks the affected trace;
- alert motion is localized to the responsible instrument;
- animations should be bounded and deterministic under reduced motion;
- avoid full-screen particles, gratuitous parallax, and independent looping widgets.

## Distinct surface directions

### Command Core — tactile intervention console

- dominant coherence/attention instrument;
- existing GO, STOP, ROUTE, and ENTER command bank retained as command-level controls;
- Settings, Terminal, and Hermes Dashboard relocated from the detached bottom row to a physically separate utility bank on the Command Core front plate;
- detached bottom row removed after parity and accessibility verification;
- Service Health remains available through the existing Fleet/Backbone workstation and may contribute to the Command Core health signal, rather than becoming a duplicate utility launcher;
- every consequential action transitions through ready, blocked, dispatched, and receipt-proven states.

### Governance/Guardhouse — decision chamber

- left queue/list for pending approvals, incidents, and review records;
- right selected-record detail with rationale, evidence, policy authority, and append-only decision controls;
- a narrow upper posture rail for autonomy, policy, and Guardhouse state;
- no stacked dashboard cards.

### Fleet/Backbone — topology and maintenance scope

- dominant node/backbone topology or rack-line visualization;
- node selection reveals hardware, models, health, drift, and source detail;
- maintenance/action controls appear only for the selected node and only when backed by an existing adapter;
- provider routing content is removed from this detail owner.

### Routing/Communications — flow field

- lanes and provider/model routes shown as a directed flow field;
- select a lane or route to inspect policy, capacity, health, and change history;
- provider refresh uses the corrected native action contract;
- no fleet hardware, package inventory, or setup duplication.

### Human/Business/Personal — continuity console

- three coordinated but visually distinct horizons: Human, Business, Personal;
- a concise current-focus rail and timeline/constellation view for commitments and changes;
- opportunity and engagement records use list/detail only where density requires it;
- missing underlying client/project files display as missing, not as realized business state.

## Non-goals

- no new chart library;
- no generic component-system rewrite;
- no replacement of Three.js lower apertures with DOM dashboards;
- no new source of truth for tasks, approvals, routing, fleet, or settings;
- no invented metrics to make a surface look active;
- no copied art assets or artist-specific visual replicas;
- no large asset or shader additions before native performance evidence.

## Design gate

A design slice is ready for implementation only when:

1. the owning ARDA source family is named;
2. every displayed value has a truth classification;
3. absent connections have an explicit unavailable/missing state;
4. the primary operator question is written in one sentence;
5. the interaction flow is testable without relying on visual appearance;
6. the physical lower surface and focused workstation responsibilities do not duplicate each other;
7. the slice reuses existing components/adapters unless a measured gap requires a new one.

## Sources

[1] https://www.artstation.com/artwork/blzWXn — Fictional User Interface - Core Control Panel
[2] https://www.artstation.com/artwork/Q6k2d — Transceiver - Sci-Fi FUI
[3] https://territorystudio.com/project/blade-runner-2049 — Blade Runner 2049 - Territory Studio
[4] https://www.behance.net/gallery/131319489/Device-UI — Device UI
[5] https://www.artstation.com/artwork/zPrJPZ — sci-fi ui exploration futuristic concepts
[6] https://magazine.artstation.com/2020/10/the-importance-of-ui-in-star-citizen — The Importance of UI in Star Citizen
[7] https://scifiinterfaces.com/2020/06/23/scifi-interfaces-qa-with-territory-studio — SciFi Interfaces Q&A with Territory Studio
