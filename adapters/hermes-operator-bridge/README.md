# Hermes Operator Bridge

Authenticated Discord-to-Arda handoff for concise `arda …` operator commands.

## Boundary

- Hermes owns Discord credentials, authorization, and replies.
- Only authorized Discord messages beginning with `arda ` are intercepted.
- Arda receives normalized credential-free events over `127.0.0.1:7878` only.
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
conversation continues through Hermes. It must not call private gateway methods,
copy transcripts, trust display names as identity, or expand tools/data/action
authority. The existing command path's `_is_user_authorized` and
`_deliver_platform_notice` calls are compatibility debt, not precedent for new
continuity code.

## Install

Copy this directory to `$HERMES_HOME/plugins/arda-operator-bridge/`, then restart Hermes Gateway. Do not add Discord credentials to this plugin or Arda configuration.

## Verify

```sh
python -m unittest -v adapters/hermes-operator-bridge/test_plugin.py
python -m py_compile adapters/hermes-operator-bridge/__init__.py
```
