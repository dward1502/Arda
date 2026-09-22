# Hermes Operator Bridge

Authenticated Hermes Gateway-to-Arda handoff for bounded conversational intents and
explicit `arda …` fallback commands.

## Boundary

- Hermes owns transport credentials, authorization, and replies.
- Explicit authorized gateway messages beginning with `arda ` remain the
  deterministic fallback.
- Private authorized messages are intercepted only for bounded forms: capture,
  context recovery, explicit research, or an objective naming an attached
  project UUID or exact unambiguous name.
- Consequential verbs without a named target/project produce a clarification;
  nothing is saved or executed.
- Shared rooms accept bounded project intake (`For project Routing review,
  objective inspect routing`) without personal capture or execution authority.
  Unknown/ambiguous project names fail without creating work.
- `Show work in this conversation` (or `arda objectives`) in a shared room lists
  only work admitted by this operator through the same adapter, platform,
  conversation and thread. Status omits objective text, projects and results;
  it includes explicit pause/resume/cancel commands bound to that scope.
  Resume preserves approval requirements; it does not grant write authority.
- Other shared-room commands fail closed; private context, results and approvals
  are not exposed by allowing project intake. Unrecognized ordinary conversation
  stays with normal Hermes dispatch. CLI intake uses the separate local boundary below.
- Arda receives normalized credential-free events over `127.0.0.1:7878` only.
- The gateway authorization check binds accepted transport identity to the
  configured canonical `ARDA_OPERATOR_ID`; raw platform user IDs are not used
  as Arda storage authority.
- Personal commands are rejected by Arda outside a private conversation.
- Attachments are rejected.

## Local CLI intake

The installed plugin registers `hermes arda --operation <stable-key>
--message-id <persisted-user-row> --command '<interpreted Arda command>'`.
Hermes resolves these internal references; the operator need not supply UUIDs or
repeat an ordinary request. `HERMES_SESSION_ID` selects the persisted originating
CLI session. If runtime compression has produced an unpersisted alias, explicitly
bind the actual persisted originating session; never insert history to satisfy
the adapter. Persisted compression continuations keep root conversation identity.
Delegated/branched/foreign sessions and synthetic summary rows are rejected.

`POST /v1/operator/local-messages` requires a separate `ARDA_HERMES_LOCAL_CAPABILITY`
on the daemon and the same owner-private capability in
`$HERMES_HOME/state/arda-operator-bridge/local-capability` (mode 0600).
It accepts `local_session`, CLI/private provenance only; gateway credentials are
not a fallback. The daemon trusts the local capability holder; the plugin retains
the real user-row reference and content digest locally, not as independently
daemon-verified human authorship. Commands are agent interpretations, not literal
human transport messages. No project permission or execution approval is implied.

Local intent is retained before posting. Reuse its operation key for retries;
changed command/provenance conflicts locally. A server 409 means read authoritative
state before retrying, not success. Use `arda objectives` for objective status;
`arda status <id>` addresses execution runs. CLI responses return to the invoking
conversation; this does not prove external gateway delivery.

## Durability

The plugin writes each normalized event atomically to `$HERMES_HOME/state/arda-operator-bridge/pending/` with mode `0600` before delivery. Successful and terminal 4xx responses remove it. Network failures and 5xx responses retain it and trigger bounded retries. A later event in the same destination reactivates retained backlog after a gateway restart. Arda's event identity remains authoritative for replay rejection; HTTP 409 duplicate-event responses are treated as an already-completed delivery.

The initial notice means queued locally in Hermes, not admitted by Arda. Admission
and cached-response retry use an outbox identity scoped by operator, adapter,
platform, chat, thread, audience and message ID. Persisted destinations are
checked before submission and delivery; legacy entries are reused only for the
same scoped identity and cannot be redirected by a colliding message ID.
Admission is confirmed only by the backend response. The connected regression
`gateway_python_bridge_roundtrip_uses_real_http_and_storage` runs this Python
bridge against the real Rust HTTP harness and SQLite with isolated fixture state;
transport authorization/delivery remain fixtures, not installed user acceptance.

## Reminder delivery

Version `0.5.0` can deliver due reminders to the most recent authorized Discord
DM after `ARDA_REMINDER_TRANSPORT=discord_dm` is set for Hermes Gateway and the
backend reports the transport configured. Shared rooms never become reminder
destinations. The poller respects the backend quiet window, attempt cap, and
minimum interval. It persists only reminder IDs and attempt timestamps across
gateway restarts. A reminder is recorded as `delivered` only when Discord
returns a provider message ID; failures remain `attempted`. Delivered reminders
are not repeated while acknowledgement is pending.

## Continuity placement

The audited Phase 2 continuity extension belongs in this plugin rather than a
sibling bridge. It shares the authenticated source identity, loopback transport,
and durable retry boundary while preserving Hermes as the session, transcript,
route, and delivery authority. See the
[extension-surface audit](../../docs/operations/hermes-continuity-extension-audit.md).

Continuity observation must use the public
`pre_gateway_dispatch(event, gateway, session_store, **kwargs)` context, emit
only bounded session/surface metadata out-of-band, and return `None` so normal
conversation continues through Hermes. It must prefer public gateway APIs, never
copy transcripts, never trust display names as identity, and never expand
tools/data/action authority. The existing command path's `_is_user_authorized`
and `_deliver_platform_notice` calls are compatibility debt; continuity reuses
only the former because this pre-auth hook has no public authorization callback.

Version `0.4.0` emits only stable operator/session/surface identity, privacy and
domain classification, bounded reference arrays, timestamps, and an idempotency
key to `/v1/continuity/events`. Delivery runs asynchronously, persists pending
events with mode `0600`, and retries boundedly across gateway restarts. Message
text, raw platform payloads, credentials, media, and transcript content are never
included. Because Hermes currently invokes `pre_gateway_dispatch` before its
normal authorization stage and exposes no public authorization callback there,
the bridge retains its already-deployed `_is_user_authorized` compatibility call
as the narrow precondition for both command and continuity paths.

## Install

Copy this directory to `$HERMES_HOME/plugins/arda-operator-bridge/`, then restart Hermes Gateway. Do not add Discord credentials to this plugin or Arda configuration.

## Verify

```sh
python -m unittest -v adapters/hermes-operator-bridge/test_plugin.py
python -m py_compile adapters/hermes-operator-bridge/__init__.py
```
