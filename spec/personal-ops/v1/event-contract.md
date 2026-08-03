# Personal Operations v1 — Event Contract

**Status:** Append-only design.

## 1. Overview

Personal Operations records every meaningful state change as an append-only
event. Projections (inbox, today, waiting, scheduled, completed) are derived
deterministically from the event log. The log is the system of record;
projections are caches.

## 2. Event log

The event log is a JSON-lines file (`.jsonl`). Each line is a
`PersonalOpsEnvelope<Event>` as defined by
`personal-ops.schema.json`. The `record` field discriminates on
`event_type`:

| event_type               | record variant            |
|--------------------------|---------------------------|
| `capture_recorded`       | `CaptureRecordedEvent`    |
| `item_classified`        | `ItemClassifiedEvent`     |
| `item_scheduled`         | `ItemScheduledEvent`      |
| `item_completed`         | `ItemCompletedEvent`      |
| `reminder_attempted`     | `ReminderAttemptedEvent`  |
| `reminder_acknowledged`  | `ReminderAcknowledgedEvent`|

### CaptureRecordedEvent

Appends a new `InboxCapture`. No ordering dependency on prior state.

```
{
  "event_type": "capture_recorded",
  "event_id": "<uuid>",
  "occurred_at": "<timestamp>",
  "operator_id": "<string>",
  "capture": { ...InboxCapture... }
}
```

### ItemClassifiedEvent

Transitions an item from `Unavailable`/`Imported`/`Inferred` to an operator-
or inferred-selected `PersonalItemKind`. Reversals are recorded as separate
classification events, never by editing prior events.

```
{
  "event_type": "item_classified",
  "event_id": "<uuid>",
  "occurred_at": "<timestamp>",
  "operator_id": "<string>",
  "item_id": "<uuid>",
  "kind": "task",
  "evidence_class": "operator_authored",
  "confidence": null,
  "rationale": null
}
```

Operator-authored classifications are immutable: a subsequent inferred
classification event targeting the same field is rejected by the store.

### ItemScheduledEvent

Assigns or replaces a `scheduled_at` time on an item. Replaces only the
scheduling projection; prior scheduling is preserved in the event log.

### ItemCompletedEvent

Marks an item completed. A completed item is removed from active projections
(inbox, today, waiting, scheduled) and appears only in the completed view.

### ReminderAttemptedEvent

Records an outbound reminder attempt through Oromë. State becomes
`Attempted`. This is distinct from delivered/acknowledged.

### ReminderAcknowledgedEvent

Records that an `Attempted` reminder reached the operator and was seen,
deferred, dismissed, or failed. State transitions to one of
`Delivered`, `Acknowledged`, `Deferred`, `Dismissed`, or `Failed`.

## 3. Projection rules

Projections are rebuilt by replaying the event log in order:

- **inbox**: captures that have not yet been classified into a PersonalItem.
  A capture may spawn an item; once an item exists the capture is no longer
  in the inbox.
- **today**: items classified to `Task` or `Reminder` with a `due_at` or
  `scheduled_at` on the current calendar day (operator local time), not
  completed.
- **waiting**: items awaiting operator acknowledgement (reminders in
  `Attempted` or `Deferred` state whose `minimum_interval_minutes` has not
  elapsed).
- **scheduled**: items with a future `scheduled_at` or `due_at`, not completed.
- **completed**: items for which a `completed_at` event exists.

Reclassification and rescheduling preserve history: the event log always
grows, never edits a prior event.

## 4. Idempotency and operator identity

Every mutating event carries an `operator_id`. Replay is deterministic; the
same event applied twice must not duplicate state. Clients may pass a
client-supplied `event_id` to ensure idempotency across retries.
