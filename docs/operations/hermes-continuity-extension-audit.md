---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "extension_surface_audit"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 extension_surface_audit | owner: HERMES | status: active | reviewed: 2026-08-17

# Hermes Continuity Extension Surface Audit

## Audited runtime

- Hermes Agent `v0.20.3 (2026.8.16.2)` from `$HERMES_HOME/hermes-agent`.
- Python `3.11.15`.
- Repository plugin `arda-operator-bridge` version `0.2.0` is enabled in the active profile.
- Baseline bridge tests pass 3/3 and the plugin compiles with `py_compile`.

Authoritative references were the current Hermes [hooks](https://hermes-agent.nousresearch.com/docs/user-guide/features/hooks), [plugin system](https://hermes-agent.nousresearch.com/docs/user-guide/features/plugins), and [plugin authoring guide](https://hermes-agent.nousresearch.com/docs/developer-guide/plugins). Installed source was inspected only to confirm the deployed version's invocation and session-key behavior.

## Supported extension contracts

### Normalized inbound dispatch

`pre_gateway_dispatch(event, gateway, session_store, **kwargs)` runs for user-originated gateway messages before normal authentication/agent dispatch. The callback must accept forward-compatible keyword context.

The normalized event/source surface supplies bounded identity and routing inputs including:

- platform;
- stable platform user id;
- chat id and chat type;
- thread/topic id where the platform supports it;
- message id and timestamp;
- message text/type and media references.

The hook may return `skip`, `rewrite`, `allow`, or `None`. Continuity observation must return `None` so ordinary conversation continues through Hermes. Only the existing explicit `arda ` command path may return `skip` after handling the command.

### Session lifecycle

Public plugin lifecycle hooks include `on_session_start`, `on_session_end`, and `on_session_reset`. Gateway event hooks separately expose events such as `gateway:startup`, `session:start`, `session:reset`, `agent:end`, and `command:*`. Hooks are process-loaded; repository edits do not hot-reload the active gateway.

Hermes session identity remains native Hermes authority. Continuity may retain the gateway session key/current session id and an Arda lineage reference, but it must not create a second transcript database. `session_store.append_to_transcript(...)` is documented for silent ingestion, not needed for out-of-band continuity metadata.

### Thread/topic and delivery semantics

The stored gateway session route owns platform, chat, thread/topic, profile, and conversation history. A thread/topic change is therefore a surface transition, not a display-name change. Any destination route must be derived from authenticated source identity and current gateway authorization.

Public side-channel delivery is available through the registered platform adapter (`gateway.adapters[platform].send(...)`). Gateway injection is separately privileged: the plugin must be explicitly granted `allow_gateway_injection`, must already know a stored session key, and cannot provide an arbitrary replacement route. Hermes rechecks the stored route against current authorization. An accepted asynchronous injection proves scheduling only, not completed model execution or platform delivery.

## Existing bridge assessment

Continuity belongs in the existing `adapters/hermes-operator-bridge/` plugin:

- it already owns the authenticated Hermes-to-Arda loopback boundary;
- it already provides mode-`0600` pending persistence and bounded retry;
- command and continuity events share source identity and endpoint trust requirements;
- no hook-lifecycle or deployment constraint requires a sibling plugin.

The command implementation currently calls private gateway helpers `_is_user_authorized` and `_deliver_platform_notice`. They are deployed compatibility debt, not a continuity API precedent. Continuity must use normalized authenticated hook context and public adapter/delivery APIs where available. A later bounded refactor may remove the private command calls without changing this placement decision.

## Continuity implementation constraints

1. Emit minimal out-of-band metadata and return `None` for ordinary messages.
2. Never copy raw transcripts, credentials, display-name-only identity, or unrestricted event metadata into Arda.
3. Preserve current `arda ` interception and durable retry behavior.
4. Use a stable operator reference plus Hermes session id/lineage, platform/chat/thread surface id, privacy/domain class, and idempotency key.
5. Treat gateway restart, duplicate replay, thread changes, expiry, shared-surface privacy, and Arda unavailability as explicit states.
6. Do not grant tools, data domains, destination routes, or action authority through a handoff.
7. Restart the gateway after plugin deployment and verify the first genuine message for hook-signature errors.

## Decision

Extend `arda-operator-bridge`; do not create a sibling continuity plugin. The repository plugin remains a bounded bridge around Hermes' native session and delivery authority, while Arda owns governed continuity references and handoff receipts.