# HUD frontend/backend convergence contract v1

**Status:** Draft contract preparation; implementation is blocked by the Stage 5 frozen-source boundary.

This directory defines the shared C0 fixture boundary for Rust, the Tauri bridge, and React. It does not authorize backend feature work while the Stage 5 candidate is frozen.

## Authority decisions

- `arda-engine` authenticates operator sessions and owns authoritative identifiers, timestamps, policy decisions, topology, revisions, and durable mutation receipts.
- React submits typed intent. It never submits a `RunGraph`, `policy_safe`, an approval timestamp, completion evidence, or a receipt digest as authority.
- Tauri transports typed intents and projections without creating authority.
- An idempotency key is stable for one operator intent and retry; the backend binds it to operator, action, and target and rejects conflicting reuse.
- Accepted mutations return a backend-issued receipt with a durable reference. Transport success alone is not acceptance.

## Workbench request boundary

| Operation | React supplies | Rust supplies |
|---|---|---|
| Plan | project ID, objective text/input mode, authenticated session reference, idempotency key | run/objective/node/edge IDs, graph topology, authority, budgets, retry/checkpoint/provenance, revision |
| Approve/reject | run ID, node ID, requested operator action, authenticated session and authority references, idempotency key | policy decision, approval ID/time, node transition, receipt |
| Complete | run ID, node ID, authenticated session and authority references, idempotency key | evidence validation, completion truth, receipt digest, terminal transition |

Cancellation, retry, project attachment, Research, Personal Operations, and monitor claims use the same intent/receipt split.

## Endpoint and command inventory

Names remain unchanged during preparation. C0 implementation versions the payloads behind these boundaries before changing consumers.

| Surface | Current transport names | Rust owner | Prepared change |
|---|---|---|---|
| Project | Tauri `validate_project_contract`, `attach_project_contract`; HTTP `POST /v1/projects/validate`, `POST /v1/projects/attach`, `GET /v1/projects` | `arda-engine::harness::projects` | Attachment accepts authenticated intent/authority references instead of a browser-created approval envelope. |
| Workbench planning and mutation | Tauri `plan_workbench_run`, `approve_workbench_run`, `complete_workbench_run_node`, `execute_workbench_provider_node`, `cancel_workbench_run`; HTTP `/v1/runs/plan` and `/v1/runs/:id/...` | `arda-engine::harness::runs` | Intent-only requests; Rust creates graph/IDs/decision/time/evidence/receipts. |
| Workbench reads and stream | Tauri `get_workbench_run`, `get_workbench_run_events`, `start_workbench_run_event_stream`; HTTP `GET /v1/runs/:id`, `/events`, `/events/stream` | run store plus Tauri bridge | Backend cursor, per-run stream ownership, gap reload, deduplication, reconnect, and terminal closure. |
| Research | Direct HTTP `/v1/research/questions`, `/watchlists`, `/watchlists/:id/{pause,resume,retire}`, `/briefs` | `arda-engine::harness::research_operator` | Authenticated session plus intent/receipt; remove browser-created approval authority. |
| Personal Operations | Direct HTTP `/v1/personal/...` capture/item/reminder/data/projection endpoints | `arda-engine::harness::personal_ops` | Authenticated session, stable retry key, aggregate revision, common load state and receipt. |
| Monitor sessions | Tauri `claim_monitor_slot`, `release_monitor_slot`, `push_surface_payload`, `refresh_monitor_slot_lease` | Tauri monitor-session runtime pending durable Rust ownership | Five canonical slots, typed session/content identity, revision/lease, durable reload, same-session handoff. |

## Projection vocabulary

Every projection uses exactly one of:

- `loading`: no authoritative response has completed;
- `healthy`: complete and within its freshness window;
- `stale`: last valid complete value exceeded its freshness window;
- `partial`: some named sources are missing, but a bounded projection remains usable;
- `degraded`: complete enough to operate with an explicit reduced-capability reason;
- `unavailable`: the authority cannot currently be reached and no usable value exists;
- `failed`: the authority responded or recovery ran, but the operation/projection failed.

All non-loading projections name source revision/time. Non-healthy states provide an operator-readable recovery action. Empty data is not evidence of `healthy`.

## Event stream contract

- The cursor is backend-issued and opaque to clients.
- Sequence is monotonic per run; `(run_id, sequence)` is the deduplication key.
- Reconnect sends the last accepted cursor.
- A gap triggers durable run reload before accepting later events.
- Each run owns an independent stream; opening another run never replaces it.
- Terminal events close only that run's stream.
- Browser storage may retain navigation hints, never recovery authority.

## Monitor-session v2 boundary

The canonical upper slots are `monitor_left_1`, `monitor_left_2`, `monitor_center`, `monitor_right_1`, and `monitor_right_2`. Each has independent owner/session/revision/lease/content identity. Opening a workstation uses the same `session_id`; it does not create a copy. The shared fixture deliberately covers web, media, document, terminal, and custom content.

Current `arda.arda_boardroom_slots.v1` data exposes only `monitor_left_1` through `monitor_left_4`; that is an implementation gap, not permission to weaken this contract.

## Shared fixtures

- Schema: `hud-frontend-backend-contract.schema.json`
- Passing fixture: `fixtures/valid-shared-contract.json`
- Fail-closed fixture: `fixtures/invalid-client-authority.json`

The fixed-fixture gate in `tests/test_workbench_contract_fixtures.py` validates the schema, both fixtures, the seven-state vocabulary, five canonical monitor slots, independent owners, and same-session workstation continuity. Rust, Tauri, and React implementation tests must consume the same passing fixture during C0 implementation rather than copying its values into language-specific fixtures.
