# Personal Operations private-alpha acceptance

**Status:** Operational acceptance remains open; not a Workbench 1.0 blocker  
**Implementation authority:** `docs/archive/2026-07-29-personal-operations-plan.md` after implementation closeout  
**Evidence rule:** Do not infer operator acceptance from automated tests, author activity, proxy use, or synthetic fixtures.

**2026-08-17 disposition:** The first bounded dogfood window ended with only its
baseline row populated. It supplies no genuine usefulness or burden verdict, so
all operator gates below remain open. Any later attempt requires a new bounded
window; this record must not be backfilled into an acceptance claim.

## Purpose

This record owns the time-dependent operator evidence that remains after the Personal Operations implementation is complete. Keeping these checks here prevents a finished implementation plan from remaining in `docs/plans/` while preserving honest private-alpha acceptance.

Personal Operations remains local, optional, non-clinical, and subordinate to Arda's canonical event, memory, governance, communications, and receipt authorities.

## Automated prerequisites

The implementation closeout must establish all of the following before operator evidence is accepted:

- append-only capture, classification, schedule, reminder, correction, completion, export, and deletion behavior;
- restart/reopen recovery from the durable personal event ledger;
- source-cited morning, transition, resume, and today projections;
- explicit correction of brief content through an operator-authored event;
- identity, idempotency, privacy, keyboard, screen-reader, reduced-motion, and high-contrast tests;
- no modification of Workbench run, execution, or governance receipts during personal-data deletion or correction.

Automated tests prove implementation behavior only. They do not satisfy the operator checks below.

## Operator evidence gates

### A1 — Thought-to-durable-capture latency

- [ ] The operator completes at least 10 representative keyboard captures in the native Tauri HUD.
- [ ] Median elapsed time from focused input to visible durable-save confirmation is below five seconds.
- [ ] Each accepted capture remains present after closing and restarting Arda.
- [ ] The evidence records native build/source identity, sample count, median, maximum, failures, and UTC start/end times.

### A2 — Runtime privacy and accessibility

- [ ] The operator verifies rapid capture, correction, reminder acknowledgement, export, and two-step deletion in the native Tauri HUD.
- [ ] Keyboard-only operation, screen-reader labels/status announcements, reduced motion, and high contrast remain usable.
- [ ] Export contains only the requesting operator's personal records.
- [ ] Deletion removes only that operator's personal records and preserves system receipts.
- [ ] Any unavailable or degraded backend source is shown explicitly rather than replaced with synthetic success data.

### A3 — Seven-day private-alpha dogfood

- [ ] One uninterrupted seven-day observation window is identified by UTC start/end times and one source/build identity.
- [ ] The operator uses capture and context recovery on representative days rather than relying on fixture traffic.
- [ ] Reminder attempts, delivered/acknowledged states, snoozes, dismissals, and quiet-window suppressions remain distinguishable.
- [ ] Daily or transition briefs cite their source records and at least one real correction is replayed after restart.
- [ ] The closeout reports capture count, reminder volume, acknowledgement/snooze/dismissal counts, recovery uses, corrections, failures, and known limitations.
- [ ] The operator explicitly records whether reminder volume was manageable and context recovery was useful.

## Evidence location

Store accepted evidence under:

```text
docs/evidence/personal-operations/private-alpha/<run-id>/
```

The closeout should contain a machine-readable receipt plus a concise human-readable report. Raw private content, contact details, health data, and credentials must not be committed. Counts, timings, redacted identifiers, source digests, and operator verdicts are sufficient.

## Classification

These gates determine whether Personal Operations may advance from implemented optional application to accepted private alpha. They do not block Workbench 1.0, Stage 5 replacement development, or Stage 6 release qualification. Failed or unavailable operator evidence keeps Personal Operations classified as implemented but not private-alpha accepted; it does not authorize fabricated evidence or proxy sign-off.
