# Personal Operations privacy review

**Review date:** 2026-08-10
**Scope:** `arda.personal-ops.v1`, the local harness APIs, calendar and voice adapters, HUD projection, and optional wellness work proposed in `docs/plans/2026-07-29-personal-operations-plan.md`.

## Decision

**Phase 5 wellness support is not approved for implementation yet.**

The current local-only foundation is suitable for continued private-alpha work on capture, scheduling, reminders, context recovery, and operator-reviewed voice transcription. It is not yet suitable for validated-device ingestion, medication-history handling, trend summaries, or clinician export.

Approval requires closing the blocking controls below and repeating this review against the resulting implementation.

## Data classes and authority

| Data | Authority | Default handling | Prohibited behavior |
|---|---|---|---|
| Text/audio capture | operator-authored | local append-only event; audio adapter defaults ephemeral | external upload without explicit adapter policy and receipt |
| Classification | operator-authored, imported, or inferred | evidence class retained; inferred values remain reversible | inferred overwrite of operator-authored fields |
| Calendar records | operator-authored or imported | local ICS/CalDAV adapter; credentials remain secret references | inline credentials or silent external synchronization |
| Reminder state | Oromë delivery receipt plus operator acknowledgement | attempted, delivered, acknowledged, deferred, dismissed, and failed remain distinct | inferring adherence from delivery or acknowledgement |
| Wellness/health records | self-reported, imported, or validated-device measured | not enabled by this review | diagnosis, dosage changes, treatment advice, emergency authority, or silent escalation |
| Voice transcript | local adapter output pending operator review | editable; retention must be explicit | governed action or external send before operator review |
| Presence/camera context | optional contextual evidence | not a personal identity or health authority | sole-source identity, wellness, or adherence inference |

## Verified controls

- The harness rejects non-loopback binds, and Personal Operations mutations check loopback peers.
- The personal event contract is append-only and separates reminder attempts from delivery and acknowledgement.
- Health types carry an evidence class and non-clinical disclosure.
- Reminder routing has bounded attempt counts, minimum intervals, quiet-window evaluation, snooze, and dismissal behavior.
- Calendar configuration stores credential references rather than credential values.
- `/data/personal/` is ignored by Git so runtime captures are not accidentally added to source control.
- Every mutation cross-checks the request operator against `x-arda-operator-id`
  and derives a deterministic event ID from the bounded idempotency key.
  Item mutations also reject a capture owned by another operator.
- Personal state uses owner-only `0700` directory and `0600` event-log modes.
- Authenticated export and idempotent operator-only deletion are implemented;
  deletion emits a content-free hashed-operator receipt and does not modify
  system run, governance, or execution receipts.
- When more than one operator has personal records, reads require
  `x-arda-operator-id` and project only that operator. The no-header compatibility
  path is limited to the loopback-only empty/single-operator profile and is not
  remote or multi-user authentication.

## Blocking gaps

1. **PO-PRIV-004 — retention policy is incomplete.** Text captures, imported calendar content, transcripts, attachments, and source audio need independent retention controls. Ephemeral audio must have verified deletion behavior; a declaration alone is insufficient.
2. **PO-PRIV-005 — adapter egress receipts are incomplete.** Any CalDAV or future device adapter must declare endpoint scope, transmitted fields, reason, result, and receipt reference while excluding credentials and sensitive payload bodies.
3. **PO-PRIV-006 — at-rest protection and backup scope are undefined.** The release backup/restore path must explicitly include or exclude Personal Operations data, document encryption expectations, and test restore/delete behavior.
4. **PO-PRIV-007 — wellness semantics lack dedicated misuse tests.** Before Phase 5, tests must prove that reminders cannot infer adherence, device records cannot become diagnoses, trend summaries expose evidence classes and uncertainty, and no automatic emergency or clinician communication exists.

PO-PRIV-001 through PO-PRIV-003 are closed for the loopback-only private-alpha
profile by the [P5.1 live acceptance receipt](../evidence/2026-08-10-p5.1-live-personal-operations-acceptance.md).
Their closure does not approve wellness ingestion or remote/multi-user exposure.

## Required gate for a future Phase 5 review

- authenticated or explicitly single-user operator identity enforcement;
- durable idempotency and replay tests for every mutation;
- owner-only personal state storage with tested backup/restore boundaries;
- separate retention and deletion controls for events, transcripts, attachments, and audio;
- bounded export and deletion with receipts that contain no sensitive payload;
- adapter egress allowlists and redacted receipts;
- threat and misuse fixtures covering medication, device, trend, clinician-export, and silent-escalation prohibitions;
- operator acceptance of the exact data inventory and retention defaults.

Until these gates pass, optional wellness work remains deferred. Private-alpha capture and planning work must continue to use neutral language and explicit evidence classes.