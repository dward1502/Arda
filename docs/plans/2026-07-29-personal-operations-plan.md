# Arda Personal Operations Implementation Plan

> **For Hermes:** Phase 0 contract preservation may proceed against the versioned Workbench project/run foundation. Do not begin the Personal Operations service, reminder delivery, or HUD beta until the Workbench run/event boundary is stable. Use `subagent-driven-development`; preserve health and accessibility guardrails.

**Goal:** Build an accessibility-aware personal operations application that captures thoughts immediately, organizes them later, supports schedules and reminders, reconstructs context, and produces calm daily briefs without becoming a clinical or coercive system.

**Architecture:** Personal Operations is a first-party application over the Arda kernel. It reuses task, event, memory, communications, governance, and receipt contracts; it does not create a second queue or memory system. The first UI is a focused HUD module, with Mirromere consuming the same projections later.

**Tech stack:** Rust/Serde contracts, Vairë memory, Oromë delivery, Tauri/React HUD, local SQLite or append-only event store behind a repository trait, CalDAV/iCalendar adapters, local STT as an optional input adapter.

**Target stage:** Stage 5 private alpha; not a Workbench 1.0 blocker  
**Primary doctrine:** **Capture now, organize later**

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

**Canonical paths**
- `data/personal/events.jsonl`
- `data/personal/inbox.json`
- `data/personal/today.json`
- `data/personal/daily_brief.json`

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
- [ ] Add CalDAV as a supervised adapter after local fixtures pass.
- [x] Store secret references, never credentials, in config.

**Acceptance**
- [x] Time zone and daylight-saving transitions are tested (ICS export uses UTC; CalDAV follows).
- [x] Duplicate sync does not duplicate events.
- [x] External updates remain distinguishable from Arda-authored reminders.

**Implementation update (2026-08-02):** Task 2.1 is implemented. The
calendar adapter module provides `IcsExporter`, `IcsImporter`, and
`deduplicate_events` with full test coverage (8 tests). The config
example stores secret references for CalDAV credentials.

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
- Create: `apps/arda-hud/src/components/arda/modules/PersonalOperationsModule.tsx`
- Create: `apps/arda-hud/src/components/arda/modules/PersonalOperationsModule.test.tsx`
- Create: `apps/arda-hud/src/lib/personalOps.ts`
- Modify: `apps/arda-hud/src/App.tsx`

**First screen**
- one next action;
- rapid capture field;
- resume card;
- today timeline;
- reminders awaiting acknowledgement;
- quiet-mode status.

**Acceptance**
- Capture can be completed from keyboard in one action after focus.
- The UI never requires categorization before save.
- Screen-reader labels, reduced motion, high contrast, and keyboard paths are tested.

### Task 3.2: Add review-assisted organization

Classification suggestions show confidence and rationale. Bulk review is bounded; no automatic conversion of every note into a task.

## Phase 4 — Voice input and daily replay

### Task 4.1: Add local speech capture adapter

**Files**
- Create: `adapters/voice-capture/README.md`
- Create: `adapters/voice-capture/arda_adapter.py`
- Create: `adapters/voice-capture/tests/test_adapter.py`
- Add supervised configuration under `config/adapters/`.

**Acceptance**
- Audio retention defaults to ephemeral; transcript retention is explicit.
- Transcript is editable before any external send or governed action.
- Offline/failure returns the audio capture to a recoverable inbox state.

### Task 4.2: Produce morning and transition briefs

Briefs summarize operator-authored schedules, unresolved captures, recent run receipts, and explicitly connected projects. They must cite source records and expose uncertainty.

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

`apps/arda-hud/package.json` does not currently define a `lint` script; add and
enforce that gate before the Phase 3 HUD beta rather than listing a command that
cannot run.

## Release acceptance

- Thought-to-durable-capture median is under five seconds in operator testing.
- Restart loses no accepted capture.
- Daily brief cites its sources and can be corrected.
- Reminder delivery truth is receipted.
- A week-long dogfood run demonstrates manageable reminder volume and successful context recovery.
- Operator can export or delete personal application data without damaging Arda system receipts.

## Rollout rule

Begin as a private operator alpha. Do not market medical capability. Mirromere consumes Personal Operations APIs only after the text-first workflow is useful and stable.
