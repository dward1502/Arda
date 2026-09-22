---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-22"
  tags: [m2, scheduling, restart, verification]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-22

# M2 — Installed Scheduling and Restart

## Prerequisites
- Daemon running on port 7878, 37 projects loaded
- M1 complete: `m1-gate1-execute-20260922` all 4 nodes succeeded
- wgtt project ID: `70656753-1e03-4375-9a29-ecf220292c19`
- No existing schedules in the database (confirmed empty)

## Step-by-step

### M2.1 — Schedule states through resident authority
1. Create an objective via operator message API (`POST /v1/operator/message` with objective command)
2. Add immediate schedule (next_wake_ms = now) via the schedule store
3. Run the objective runtime round — verify schedule becomes due and objective is claimed
4. Add deferred schedule (next_wake_ms = now + 60s) — verify it stays pending until wake
5. Add recurring schedule (recurrence = PT1M) — verify it reschedules after each execution
6. Test pause/resume/cancel on a scheduled objective — verify paused schedules are not claimed
7. Test terminal wake suppression — verify a cancelled schedule does not wake

### M2.2 — Durable checkpoint across forced restart
1. Start a run with a deferred schedule (next_wake_ms = now + 5s)
2. Let the objective begin execution (checkpoint sequence advances)
3. Kill the daemon mid-execution (SIGTERM after checkpoint persisted)
4. Restart the daemon
5. Verify the run continues from the durable checkpoint — same run/authority, no duplicate execution
6. Verify the schedule resumes and the objective completes without replaying completed work

### M2.3 — Verification failure with bounded retry
1. Create an objective with a declared verification check that will fail
2. Run the objective — verification fails
3. Observe bounded retry: the runtime retries up to MAX_OBJECTIVE_ATTEMPTS=5
4. After correction, verification passes and the objective closes
5. If all retries exhausted, verify the objective stops with justified failure state

## Evidence required
- Schedule creation: canonical schedule records with idempotency keys
- Execution: run events showing claimed → executing → succeeded/failed transitions
- Restart: checkpoint sequence numbers before/after kill, no duplicate execution
- Retry: attempt counts, verification failure events, correction events

## Exit gate
- Installed scheduling, interruption, and correction succeed unattended
- Resident path proven (not CLI simulation or direct store manipulation)
- MAX_OBJECTIVE_ATTEMPTS=5 cap verified in resident ObjectiveRuntime