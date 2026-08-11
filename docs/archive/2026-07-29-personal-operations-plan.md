# Arda Personal Operations Implementation Plan

> **For Hermes:** Phase 0 contract preservation may proceed against the versioned Workbench project/run foundation. Do not begin the Personal Operations service, reminder delivery, or HUD beta until the Workbench run/event boundary is stable. Use `subagent-driven-development`; preserve health and accessibility guardrails.

**Goal:** Build an accessibility-aware personal operations application that captures thoughts immediately, organizes them later, supports schedules and reminders, reconstructs context, and produces calm daily briefs without becoming a clinical or coercive system.

**Architecture:** Personal Operations is a first-party application over the Arda kernel. It reuses task, event, memory, communications, governance, and receipt contracts; it does not create a second queue or memory system. The first UI is a focused HUD module, with Mirromere consuming the same projections later.

**Tech stack:** Rust/Serde contracts, Vairë memory, Oromë delivery, Tauri/React HUD, local SQLite or append-only event store behind a repository trait, CalDAV/iCalendar adapters, local STT as an optional input adapter.

**Target stage:** Stage 5 private alpha; not a Workbench 1.0 blocker  
**Primary doctrine:** **Capture now, organize later**

**Lifecycle status:** IMPLEMENTATION COMPLETE and archived on 2026-08-10.
Phases 0–4 and every automatable release-acceptance behavior are implemented
and verified. Optional wellness support remains unapproved and deferred by the
[privacy review](../security/personal-operations-privacy-review.md). Native
operator observation and the uninterrupted seven-day dogfood verdict remain
open under the [private-alpha operational acceptance record](../operations/personal-operations-private-alpha-acceptance.md);
they do not keep this non-release-blocking implementation plan active.

---

## Verified starting point

- `core/state/personal_runtime.json` projects identity, values, priorities, research domains, and personal documents, but it is a generated snapshot rather than an interactive personal-operations store.
- `core/state/human_inbox_ingestion_plan.json` contains stale legacy paths and authority names; it is evidence to migrate, not a runtime contract to extend blindly.
- `apps/arda-hud/src/components/arda/modules/HumanRealmModule.tsx` displays readable docs and plan summaries.
- `OperatingSurfacePlanModule.tsx` already defines Now, Work, Decisions, Knowledge, Health, Business, Evidence, and Settings lanes.
- Oromë provides communication/delivery receipts; Vairë provides continuity. Personal Operations must consume both rather than inventing reminder delivery truth or duplicate memory.

## Safety and accessibility constraints

- Wellness support is clearly distinguished from clinical measurement or medical advice.
- Medication reminders may replay an operator-authored schedule; they may not change dosage, infer adherence, or recommend treatment.
- Overdue items use neutral language and offer complete, defer, reschedule, split, or dismiss.
- Every voice action has text, transcript, edit, and replay equivalents.
- Quiet mode, sensory intensity, interruption windows, and reminder fatigue limits are first-class policy.
- Camera/presence signals are optional context only and never sole identity or health evidence.
- No contact, calendar, or health data leaves the local system without explicit adapter policy and a receipt.

## Phase 0 — Define personal operations contracts

**Implementation update (2026-07-30):** Task 0.1 is implemented as a bounded
parallel-design tranche. It does not start the Personal Operations service or
make Personal Operations a Stage 4 dependency.

### Task 0.1: Add capture and planning types

**Files**
- [x] Create: `crates/spine/governance/arda-core/src/personal_ops.rs`
- [x] Modify: `crates/spine/governance/arda-core/src/lib.rs`
- [x] Create: `spec/personal-ops/v1/personal-ops.schema.json`
- [x] Test: `crates/spine/governance/arda-core/tests/personal_ops.rs`

**Types**
- `InboxCapture`, `CaptureSource`, `CaptureAttachment`
- `PersonalItem`, `PersonalItemKind`, `PersonalContextLink`
- `ReminderPolicy`, `InterruptionPolicy`, `QuietWindow`
- `DailyBrief`, `ContextResumeCard`, `ReminderReceipt`
- evidence class: operator-authored, imported, inferred, device-measured, self-reported, unavailable

**Acceptance**
- [x] Capture requires only text/audio reference and timestamp; project, priority, and date remain optional.
- [x] Inferred classification is reversible and cannot overwrite operator-authored fields.
- [x] Health-related records carry evidence class and non-clinical disclosure.
- [x] Reminder policy is bounded and attempted delivery is distinct from delivered/acknowledged state.

### Task 0.2: Define the event and projection boundary

**Files**
- Create: `spec/personal-ops/v1/event-contract.md`
- Create: `crates/spine/governance/arda-core/src/personal_ops_projection.rs`
- Test: `crates/spine/governance/arda-core/tests/personal_ops_projection.rs`

**Acceptance**
- [x] Append-only events produce deterministic inbox, today, waiting, scheduled, and completed projections.
- [x] Reclassification and rescheduling preserve history rather than editing prior receipts.

## Phase 1 — Universal capture and context recovery

### Task 1.1: Implement a personal operations service

**Implementation update (2026-08-02):** Task 1.1 and 1.2 are implemented.
The personal-ops service provides append-only capture, classification,
scheduling, completion, and projection endpoints. Idempotent mutations
require loopback. Quiet mode is placeholdered pending Phase 2 presence
wiring.

**Files**
- [x] Create: `crates/engine/src/personal_ops/mod.rs`
- [x] Create: `crates/engine/src/personal_ops/store.rs`
- [x] Create: `crates/engine/src/harness/personal_ops.rs`
- [x] Modify: `crates/engine/src/lib.rs`
- [x] Modify: `crates/engine/src/harness.rs`
- [x] Test: `crates/engine/tests/personal_ops_store.rs`
- [x] Test: `crates/engine/tests/harness_personal_ops.rs`

**Canonical storage and projection boundary**
- `data/personal/events.jsonl` is the only durable personal-operations event store.
- Inbox, today, resume, and daily-brief views are deterministic HTTP projections
  rebuilt from that ledger; no second mutable snapshot store is authoritative.

**Acceptance**
- [x] Capture is durable before classification begins.
- [x] Failed classification leaves an actionable inbox item, not lost data.
- [x] Replaying the ledger reproduces projections exactly.

### Task 1.2: Add inbox and “What was I doing?” APIs

**Files**
- Create: `crates/engine/src/harness/personal_ops.rs`
- Modify: `crates/engine/src/harness.rs`
- Test: `crates/engine/tests/harness_personal_ops.rs`

**Endpoints**
- `POST /v1/personal/captures`
- `GET /v1/personal/inbox`
- `POST /v1/personal/items/{id}/classify`
- `POST /v1/personal/items/{id}/schedule`
- `POST /v1/personal/items/{id}/complete`
- `GET /v1/personal/resume`
- `GET /v1/personal/briefs/today`

**Acceptance**
- [x] Mutations require idempotency and operator identity.
- [x] Resume card uses recent explicit activity and receipts, not speculative surveillance.

## Phase 2 — Scheduling and reminders

### Task 2.1: Add standards-based calendar adapters

**Files**
- [x] Create: `crates/engine/src/personal_ops/calendar.rs`
- [x] Create: `config/adapters/calendar.toml.example`
- [x] Test: `crates/engine/tests/calendar_adapter.rs`

**Approach**
- [x] Support `.ics` import/export first.
- [x] Add CalDAV as a supervised adapter after local fixtures pass.
- [x] Store secret references, never credentials, in config.

**Acceptance**
- [x] Time zone and daylight-saving transitions are tested (ICS export uses UTC; CalDAV follows).
- [x] Duplicate sync does not duplicate events.
- [x] External updates remain distinguishable from Arda-authored reminders.

**Implementation update (2026-08-04):** Task 2.1 is implemented. The
calendar adapter module provides `IcsExporter`, `IcsImporter`, and
`deduplicate_events`, plus a supervised CalDAV GET/PUT client with bounded
timeouts and retries. The config example stores secret references rather than
CalDAV credentials.

### Task 2.2: Route reminders through Oromë

**Files**
- [x] Extend: `crates/spine/interface/arda-orome/src/types.rs`
- [x] Create: `crates/spine/interface/arda-orome/src/personal_reminder.rs`
- [x] Test: `crates/spine/interface/arda-orome/tests/personal_reminder.rs`
- [x] Extend: `crates/engine/src/harness/personal_ops.rs` (reminder attempt/ack endpoints)
- [x] Extend: `crates/engine/src/harness.rs` (new routes registered)
- [x] Test: `crates/engine/tests/harness_personal_ops.rs` (reminder flow integration tests)

**Acceptance**
- [x] "Attempted" and "Delivered" are never conflated.
- [x] Repeated reminders respect fatigue caps, quiet windows, and explicit snooze/dismiss state.

**Implementation update (2026-08-02):** Task 2.2 is implemented. The
personal reminder adapter provides pure-logic routing evaluation
(`evaluate_reminder_routing`) that checks quiet mode, snooze windows,
minimum intervals (15 min), max attempts, and dismissal. Receipt
builders (`suppressed_receipt`, `delivered_receipt`,
`acknowledgement_receipt`) never conflate "Attempted" with "Delivered".
Harness endpoints `POST /v1/personal/reminders/attempt` and
`POST /v1/personal/reminders/:id/acknowledge` record these events into
the append-only personal-ops log so projections remain replayable.
All 28 Phase 2 tests pass.

## Phase 3 — HUD experience

### Task 3.1: Build Personal Operations module

**Files**
- [x] Create: `apps/arda-hud/src/components/arda/modules/PersonalOperationsModule.tsx`
- [x] Create: `apps/arda-hud/src/components/arda/modules/PersonalOperationsModule.test.tsx`
- [x] Create: `apps/arda-hud/src/lib/personalOps.ts`
- [x] Integrate the module into the HUD module registry and configured layouts.

**First screen**
- one next action;
- rapid capture field;
- resume card;
- today timeline;
- reminders awaiting acknowledgement;
- quiet-mode status.

**Acceptance**
- [x] Capture can be completed from keyboard in one action after focus.
- [x] The UI never requires categorization before save.
- [x] Screen-reader labels, reduced motion, high contrast, and keyboard paths are tested.

### Task 3.2: Add review-assisted organization

Classification suggestions show confidence and rationale. Bulk review is bounded; no automatic conversion of every note into a task.

**Acceptance**
- [x] Suggestions disclose confidence and rationale.
- [x] Operator confirmation is explicit and writes operator-authored evidence.
- [x] Bulk confirmation is capped at 10 items and never runs automatically.

## Phase 4 — Voice input and daily replay

### Task 4.1: Add local speech capture adapter

**Files**
- [x] Create: `adapters/voice-capture/README.md`
- [x] Create: `adapters/voice-capture/arda_adapter.py`
- [x] Create: `adapters/voice-capture/tests/test_adapter.py`
- [x] Add supervised configuration under `config/adapters/`.

**Acceptance**
- [x] Audio retention defaults to ephemeral; transcript retention is explicit.
- [x] Transcript is editable before any external send or governed action.
- [x] Offline/failure returns the audio capture to a recoverable inbox state.

### Task 4.2: Produce morning and transition briefs

Briefs summarize operator-authored schedules, unresolved captures, recent run receipts, and explicitly connected projects. They must cite source records and expose uncertainty.

**Files**
- [x] Create: `crates/engine/src/harness/personal_briefs.rs`
- [x] Modify: `crates/engine/src/harness.rs`
- [x] Test: `crates/engine/tests/harness_personal_ops.rs`

**Acceptance**
- [x] Morning and transition endpoints expose scheduled items and unresolved captures.
- [x] Recent receipts and projects come only from their explicit local registries.
- [x] Every brief carries source records and uncertainty disclosures.

## Phase 5 — Optional wellness support

Implement only after a separate privacy review:

- validated-device adapters;
- operator-authored medication schedule replay;
- self-report capture;
- trend summaries with evidence class;
- export packet for optional clinician review.

No diagnosis, treatment recommendation, emergency authority, or silent escalation is in scope.

## Verification ladder

```bash
cargo test -p arda-core -- --test-threads=1
cargo test -p arda-engine --test personal_ops_store --test harness_personal_ops --test calendar_adapter -- --test-threads=1
cargo test -p arda-orome --test personal_reminder -- --test-threads=1
cargo fmt --package arda-core --package arda-engine --package arda-orome -- --check
cargo clippy -p arda-core -p arda-engine -p arda-orome --tests
```

The HUD `lint`, full `test`, and production `build` scripts are now enforced.
Lint currently succeeds with pre-existing warnings elsewhere in the HUD.

## Release acceptance

- [x] Thought-to-durable-capture is under five seconds on the retained live operator path.
- [x] Restart loses no accepted capture.
- [x] Daily brief cites its sources and can be corrected.
- [x] Reminder delivery truth is receipted.
- [ ] A week-long dogfood run demonstrates manageable reminder volume and successful context recovery. This operator/time gate transferred to the [private-alpha operational acceptance record](../operations/personal-operations-private-alpha-acceptance.md) and is not claimed complete here.
- [x] Operator can export or delete personal application data without damaging Arda system receipts.

**Implementation update (2026-08-06):** Phases 0–4 are implemented. Authenticated
`GET /v1/personal/data/export` and idempotent
`DELETE /v1/personal/data` paths now export or delete only the requesting
operator's personal event records. Deletion writes a separate, hashed-operator
receipt under `audit/personal-data-deletions/` and leaves system run,
governance, and execution receipts untouched. The HUD exposes export plus an
explicit two-step deletion action. Focused engine and HUD tests and the HUD
production build pass. At that checkpoint, the plan remained active pending daily brief correction,
failure-injection/restart recovery, runtime privacy and accessibility validation,
and the one-week dogfood period. The strict engine
Clippy gate remains blocked by the unrelated existing
`arda-outpost-protocol/src/watchlist.rs` argument-count warning and
`crates/engine/src/harness/research.rs` unnecessary-allocation warning.

**Live acceptance update (2026-08-10):** The retained authenticated Discord
capture reached the canonical personal event log 1.892602 seconds after Gateway
receipt and survived another deployed-root restart. A live isolated operator
proved sourced brief correction, context resume, delivered-to-deferred reminder
state with duplicate suppression, restart recovery, export, and idempotent
operator-only deletion while the system receipt tree remained byte-identical.
The linked [P5.1 receipt](../evidence/2026-08-10-p5.1-live-personal-operations-acceptance.md)
records exact evidence and scoped gates. The [P5.5 dogfood window](../evidence/2026-08-10-p5.5-personal-operations-dogfood-window.md)
is active through 2026-08-17; manageable reminder burden and operator acceptance
remain open.

**Implementation closeout (2026-08-10):** P5.1 closes the remaining automatable
implementation, correction, restart, identity, export, deletion, privacy, and
receipt-preservation gaps. The implementation plan is therefore complete.
P5.5 remains real, open operator evidence and continues only in the operational
acceptance record; no elapsed time or operator verdict is inferred by archiving.

## Rollout rule

Begin as a private operator alpha. Do not market medical capability. Mirromere consumes Personal Operations APIs only after the text-first workflow is useful and stable.
