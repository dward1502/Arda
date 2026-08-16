---
soterion:
  sigil: "SCROLL"
  role: "workstation_audit"
  owner: "WARDEN"
  status: "active"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Governance and Guardhouse workstation audit | owner: WARDEN | status: active | reviewed: 2026-08-16

# Lower Workstation 01 — Governance and Guardhouse

## Audit Status

- Evidence review: complete for current structure and source wiring as observed on 2026-08-16.
- Runtime context: native Tauri development process and Vite `:1421` were active during inspection.
- Source changes: none.
- Design changes: none.
- Final redesign: deferred until all five lower surfaces are audited and cross-referenced.

## Operator-Approved Intended Role

This workstation should be the place where the operator can:

- quickly view current governance posture;
- adjust guarded governance controls through explicit authoritative mutation paths;
- inspect security posture, edge/Guardhouse state, and policy authority;
- review governed action packets requiring human judgment;
- navigate large approval families, including Arandur packets, through a compact master-detail interaction rather than stacked cards;
- understand reason, consequence, evidence, provenance, freshness, and mutation authority before approving or rejecting anything.

A likely interaction pattern for dense approval queues is a left-side packet list with the selected packet's full detail and controls on the right. This is recorded as operator intent, not yet an implementation decision.

## Physical and Runtime Path

```text
boardroom.lower.left_wrap
  -> view_desk_l
  -> governance_guardhouse
  -> Governance And Guardhouse Workstation
  -> governance_controls + section_focus
```

Evidence:

- physical contract: `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts:3-5`;
- persisted slot: `core/state/arda_boardroom_slots.json:189-249`;
- default assignment: `apps/arda-hud/src/lib/boardroomSlotSettings.ts:142-151`;
- profile resolution: `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts:21-24`;
- workstation assembly gives manifest modules precedence: `apps/arda-hud/src/App.tsx:1919-1969`.

## Current Workstation Shell

The in-scene container is `SceneWorkstation` in `components/arda/core/SceneWorkstation.tsx`.

Current structure:

1. draggable title bar;
2. generic action icons;
3. horizontal text-tab row;
4. generic `TERM READY` status strip;
5. scrollable module body;
6. nested generic cards, lists, badges, forms, and embedded workstations.

This is functionally a conventional web administration window. It does not yet express a domain-specific governance instrument or a modern master-detail review workflow.

## Current Tab Authority Conflict

The actual manifest profile supplies:

1. `governance_controls`;
2. `section_focus`.

However, `settingsLayout.ts:28-31` defines Governance as:

1. `governance_controls`;
2. `operating_surface`.

The running workstation prefers the manifest's `module_ids` at `App.tsx:1960`; therefore the `settingsLayout` definition does not control this opening path. This is a concrete duplicated authority and must not be resolved until the remaining workstations reveal whether either table has broader ownership.

## Tab 1 — Governance Controls

Implementation: the `governance_controls` registry node is assembled inline in `App.tsx:1109-1300` rather than as one focused domain component.

### Current visible sections

1. Weights
2. Thresholds
3. Human Augmentation summary
4. Recent approvals
5. Issue Approval form
6. Autonomy Readiness
7. Next Unlocks
8. embedded Arandur Approval Workstation
9. embedded Review Gate Workstation

The result is one long vertical collection of partly related workflows.

### Governance weights and thresholds

```text
core/state/runtime_settings.json
  -> bundle.runtimeSettings
  -> getGovernanceSummary()
  -> governance.weights / governance.thresholds
  -> App.tsx Governance Controls
```

Current source facts:

- file exists;
- projection generated `2026-07-14T05:03:01.570475601Z`;
- `runtime_settings.governance` is `{}`;
- rendered weight list is empty;
- rendered threshold list is empty;
- no weight/threshold mutation controls are connected.

The `Adjustable weights` label therefore does not describe an actual adjustment capability.

### Governance runtime signals

```text
core/state/governance_runtime.json
  -> bundle.governanceRuntime
  -> getGovernanceRuntimeSignals()
```

Current projection generated `2026-07-14T05:03:01.761785457Z` and contains:

- autonomy observation score `0.85`;
- autonomy gap `0.0`;
- approved human augmentations `7`;
- pending human augmentations `0`;
- threshold outcomes for autonomy, JouleWork, Love Equation, provider health, and queue health;
- active-ruleset, autonomy-runtime, and human-augmentation contract summaries.

`getGovernanceRuntimeSignals()` derives Autonomy, Gap, JW, LE, Bacon, and Triad display values in `lib/ardaSurfaces.ts:362-382`. Those values are loaded but rendered elsewhere in the broad application surface around `App.tsx:1518`, not in this workstation's Governance Controls section.

### Human augmentation policy and approvals

```text
core/state/human_augmentation_runtime.json
  -> getHumanAugmentationRuntime()
  -> summary, policy, approvals
  -> Governance Controls + embedded review components
```

Current projection generated `2026-06-14T19:05:11.130001356Z` and contains:

- 7 approved records;
- 0 pending records;
- 2-of-3 triad policy;
- autonomous decision classes;
- human-required decision classes;
- triad-quorum decision classes;
- sovereign-override decision classes.

The `Issue Approval` form has a real mutation path, but it records a manually composed approval record. It does not modify governance policy, weights, thresholds, permission profiles, or Guardhouse controls. Its hard-coded initial values make it resemble a test harness rather than a proposal-driven operator action.

### Autonomy readiness

```text
core/state/autonomy_readiness.json
  -> getAutonomyReadinessSummary()
  -> posture, checkpoint, evidence, next unlocks
```

Current projection says:

- read-only autonomy: closed/verified;
- mutating autonomy: gated;
- supervised timer: enabled/active;
- overall posture: `read_only_closed_mutation_locked`;
- 4 evidence entries;
- 1 next unlock.

Its `updated_at_utc` is `2026-05-24T05:19:18.595575712Z`. The workstation renders it alongside newer data without sufficiently strong age hierarchy.

### Arandur Approval Workstation

Expected inputs include:

- `data/arandur/mission_queue_write_requests.jsonl`;
- `data/arandur/recommendations.jsonl`;
- `data/arandur/mission_approval_requests.jsonl`;
- `core/projects/tasks/queue.jsonl`;
- persona/runtime context.

Observed state on 2026-08-16:

- all three Arandur JSONL files are absent;
- canonical task queue exists but has zero records;
- no queue-write request or active-task list is available.

The embedded component has real Tauri actions for review, cancel, and retry paths, but currently has no proposal packets to operate on.

### Review Gate Workstation

`getReviewGateItems()` combines:

1. Arandur queue-write requests;
2. Arandur recommendations;
3. Arandur mission approvals;
4. HADES lifecycle review packets;
5. ATHENA policy-readiness packets.

Observed state:

- Arandur packet sources: absent;
- HADES lifecycle review queue: absent;
- ATHENA policy readiness: 2 records;
- both ATHENA records: `reference_only`;
- ATHENA records timestamped `2026-07-31`.

The only nonempty review family is therefore buried at the bottom of the long Governance Controls tab rather than serving as a clear review queue.

## Tab 2 — Section Focus

Implementation: `components/arda/modules/SectionFocusModule.tsx`.

Visible information:

- source-map status;
- owner;
- panel count;
- primary-source count;
- raw panel identifiers;
- raw source paths;
- freshness/provenance badges when matching records exist.

It does not interpret Guardhouse content and is therefore a source inventory/debug view, not a Governance and Guardhouse operational view.

## Declared Guardhouse Domains

The source map declares three intended panels:

1. `security_posture`;
2. `edge_guardhouse`;
3. `policy_authority`.

These are appropriate top-level information domains for this workstation, but no dedicated content adapter currently turns them into usable operator sections.

## Declared Source Inventory and Actual Presence

Source-map last Git change: `2026-08-09`.

| Declared source | Declared role | Observed state 2026-08-16 | Current use in workstation |
|---|---|---|---|
| `core/state/warden_guardhouse.json` | primary | Missing | Listed as a path; no content |
| `core/state/warden_policy_authority.json` | primary | Missing | Listed as a path; no content |
| `core/state/warden_edge_contract.json` | primary | Present, about 98 KB | Provenance/path only; content not adapted |
| `core/state/warden_nightly_doctrine.json` | primary | Present, about 24 KB | Provenance/path only; content not adapted |
| `data/prometheus/gate_matrix_last.json` | supplemental | Missing | No usable content |
| `data/prometheus/gate_metrics_last.json` | supplemental | Missing | No usable content |

The source map labels the section `ready` despite two missing primary files and both missing supplemental files. This is an explicit status-truth contradiction.

## Data Refresh and Stream Semantics

Governance data is predominantly a durable file projection, not a live stream.

`useArdaBundle()` loads the complete bundle on startup. Periodic full reload is opt-in and defaults to disabled (`refreshIntervalMs = null`) in `components/arda/hooks/useArdaBundle.ts:11-19,90-96`.

The genuine Tauri event channel `arda://hud-pulse` runs through `useArdaRuntimePulse()`, but it feeds the general Operating Surface and is disabled while the app remains in boardroom mode. It does not provide live governance events to this floating workstation.

Consequences:

- repository file changes do not automatically become visible;
- a successful mutation may explicitly refresh the bundle;
- otherwise the operator can be viewing an old projection without a true event stream;
- configured `refresh_ms` values in slot metadata do not prove that governance data is actually streamed at that cadence.

## Mutation Inventory

Currently connected governed actions include:

- record/approve human augmentation;
- review Arandur recommendation/queue-write packets when present;
- cancel approved queue tasks;
- retry approved governed tasks.

Currently absent from this workstation:

- edit active governance weights;
- edit thresholds;
- select or activate a ruleset;
- edit permission profiles;
- modify critical-decision routing policy;
- modify triad/quorum policy;
- modify Guardhouse edge policy;
- inspect mutation consequences through a dry-run before applying a governance-setting change.

No control should be added until its authoritative backend, validation, audit receipt, rollback behavior, and stale-write protection are identified.

## Current Structural Problems

1. The workstation is a generic web window rather than a domain-specific governance environment.
2. Two competing tab definitions exist.
3. Governance Controls is one oversized vertical module containing several independent workflows.
4. The title promises controls, but weights and thresholds are read-only and currently empty.
5. Useful governance runtime signals are loaded but displayed elsewhere.
6. The Guardhouse tab displays metadata and paths rather than Guardhouse information.
7. Missing sources coexist with a `ready` section status.
8. Static projections are presented without a strong distinction from live state.
9. Approval actions are disconnected from a clear selectable packet queue.
10. Arandur and Review Gate components duplicate or nest related approval concepts.
11. Generic `TERM READY` chrome implies a terminal state that this workstation does not own.
12. Deep raw identifiers and fingerprints dominate before human-readable reason and consequence.

## Provisional Information Ownership

This is provisional until all lower workstations are cross-referenced.

### Governance posture

- active ruleset and authority;
- current policy mode;
- thresholds and weights;
- autonomy/mutation posture;
- quorum/sovereign-override requirements;
- freshness and authority of each projection.

### Security posture

- current security level;
- open findings and severity;
- affected asset or boundary;
- recent change or regression;
- immediate operator action, if any.

### Edge Guardhouse

- accepted, denied, challenged, and anomalous edge actions;
- active contracts and violations;
- blocked operations;
- recent high-signal events;
- Warden doctrine or nightly changes requiring review.

### Policy authority

- which source is authoritative;
- active policy/ruleset;
- proposed changes;
- effective scope;
- consequence and rollback;
- signed mutation receipt.

### Governed review queue

- one queue across packet families;
- source/family filters rather than multiple nested review applications;
- age, severity, and waiting-time hierarchy;
- master-detail packet inspection;
- reason, consequence, evidence, provenance, and checklist;
- approve, reject, defer, dry-run, or request-more-evidence controls only where supported.

## Questions Deferred Until Cross-Reference

1. Should task cancellation/retry live here, in Work/Planning, or be surfaced in both with one owning mutation path?
2. Is autonomy readiness governance posture, evidence/audit material, or both?
3. Should ATHENA policy readiness be in the governance queue or a dedicated evidence/knowledge workstation?
4. Does provider rerouting belong to policy governance or Routing and Communications?
5. Which Guardhouse events overlap with Fleet health, provider security, or system incident response?
6. Should the generic `section_focus` module survive as a debug/settings view rather than an operator tab?
7. Should Arandur approvals and Review Gate be consolidated into one packet model and one master-detail component?

## Required Verification Before Implementation Claims

Future implementation must prove:

- every visible value has a named source and timestamp;
- missing sources render as missing, never ready;
- stale snapshots are visually distinct from live events;
- each setting mutation reaches one authoritative backend path;
- dry-run, validation, receipt, and rollback behavior are explicit where required;
- approval lists deduplicate the same underlying packet;
- in-scene and native-window presentations share the same content contract;
- a focused test covers source -> derivation -> workstation section -> mutation command;
- native Tauri observation confirms the actual interaction, not only a browser fixture.

## Evidence Age Ledger

| Evidence | Last change/generated time | Use in this audit |
|---|---:|---|
| Product doctrine | Git change 2026-08-12 | Product and truthfulness authority |
| Boardroom contract | Git change 2026-08-09 | Verified spatial boundary |
| Workstation contract | Git change 2026-07-16 | Retained only where verified against current implementation |
| First-level terminal contract | Git change 2026-08-03 | Executed physical/profile evidence |
| Persisted boardroom slot document | document update 2026-07-31 | Current persisted assignment evidence; individual epoch timestamps are not freshness proof |
| Source map | Git change 2026-08-09 | Declared source inventory, not proof that sources exist |
| Governance runtime | generated 2026-07-14 | Static projection, not current live stream |
| Runtime settings | generated 2026-07-14 | Static projection; governance payload empty |
| Human augmentation runtime | generated 2026-06-14 | Static approval/policy projection |
| Autonomy readiness | updated 2026-05-24 | Older static evidence requiring prominent age handling |
| ATHENA readiness records | timestamped 2026-07-31 | Existing reference-only review packets |
