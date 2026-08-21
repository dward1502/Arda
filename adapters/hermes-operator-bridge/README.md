# Hermes Operator Bridge

Authenticated Discord-to-Arda handoff for narrow private-language intents and
explicit `arda …` fallback commands.

## Boundary

- Hermes owns Discord credentials, authorization, and replies.
- Explicit authorized Discord messages beginning with `arda ` remain the
  deterministic fallback.
- Private authorized messages are intercepted only for bounded forms: capture,
  context recovery, explicit research, or an objective naming an attached
  project UUID.
- Consequential verbs without a named target/project produce a clarification;
  nothing is saved or executed.
- Ordinary conversation and all natural-language messages in shared rooms stay
  with normal Hermes dispatch.
- Arda receives normalized credential-free events over `127.0.0.1:7878` only.
- The gateway authorization check binds accepted transport identity to the
  configured canonical `ARDA_OPERATOR_ID`; raw platform user IDs are not used
  as Arda storage authority.
- Personal commands are rejected by Arda outside a private conversation.
- Attachments are rejected.

## Durability

The plugin writes each normalized event atomically to `$HERMES_HOME/state/arda-operator-bridge/pending/` with mode `0600` before delivery. Successful and terminal 4xx responses remove it. Network failures and 5xx responses retain it and trigger bounded retries. A later event in the same destination reactivates retained backlog after a gateway restart. Arda's event identity remains authoritative for replay rejection; HTTP 409 duplicate-event responses are treated as an already-completed delivery.

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
