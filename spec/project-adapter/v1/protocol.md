# Arda Project Adapter Protocol v1

Status: Stage 4 Task 2.1 reference protocol

## Purpose

`arda.project-adapter.v1` is the bounded execution boundary between the Arda engine and a project- or agent-specific adapter. The canonical run graph, approvals, receipts, and recovery state remain owned by Arda. Adapter-local state is never canonical run state.

JSON Lines over stdio is the required reference transport. An implementation may bridge MCP or another transport, but it must preserve these messages and semantics; MCP is not the sole protocol.

## Framing and correlation

- stdin and stdout contain UTF-8 JSON Lines: exactly one JSON object followed by `\n` per frame.
- stdout is protocol-only. Diagnostics go to stderr.
- Every frame has `schema_version: "arda.project-adapter.v1"`, a non-empty `id`, and a `type`.
- A response to `initialize`, `health`, `request`, or `cancel` repeats that frame's `id` as `request_id`.
- Unknown schema versions, unknown message types, malformed JSON, duplicate terminal responses, and correlation mismatches fail closed.
- The engine must apply an independent line-size limit and must never wait indefinitely for a partial line.

The normative structural definition is `messages.schema.json`. The ordering and lifecycle rules below are semantic requirements that JSON Schema cannot express.

## Lifecycle

1. The engine starts one adapter process with an explicit executable, argument list, bounded working directory, and cleared/allowlisted environment.
2. The engine sends `initialize` before any other request.
3. The adapter replies once with `initialized`, advertising its effective capabilities. It must not advertise capabilities outside `allowed_capabilities`.
4. The engine sends `health`; the adapter replies with `health_status`. Only `status: "ready"` permits execution.
5. The engine sends one bounded `request`. The adapter may emit zero or more monotonically increasing `progress` frames, then exactly one terminal `result` or `denied_capability` frame.
6. The engine closes or terminates the one-request process after the terminal frame. Process reuse is not part of v1.

## Messages

### `initialize` / `initialized`

`initialize` declares the protocol version, canonical project root, and capabilities approved for this process. `initialized` advertises adapter identity, effective capabilities, and whether opaque recovery tokens are supported.

### `health` / `health_status`

`health` is side-effect free. `health_status.status` is `ready`, `degraded`, or `unavailable`; only `ready` permits a request.

### `request`

A request contains:

- `operation`: an adapter-defined operation identifier, never a shell command supplied by a browser;
- `arguments`: typed JSON data interpreted by the selected adapter;
- `timeout_ms`: a positive requested bound that cannot extend the engine's configured bound;
- `required_capabilities`: every capability required by this operation;
- `idempotency_key`: stable retry identity;
- optional opaque `recovery_token` previously returned by this adapter.

The adapter must emit `denied_capability` instead of executing when any required capability was not both approved and advertised.

### `progress`

Progress is advisory and non-terminal. `sequence` begins at 1 and strictly increases for each request. Progress may include a bounded JSON `detail` object.

### `result`

`result.status` is `succeeded`, `failed`, or `cancelled`. Every result includes provenance containing adapter identity/version, canonical cwd, UTC start/finish timestamps, and a SHA-256 digest of the canonical request frame. An optional opaque `recovery_token` may be returned. Arda records it as adapter evidence; it does not replace canonical checkpoints or event journals.

### `cancel` / `cancelled`

`cancel` names the active request ID. The adapter sets a cooperative cancellation signal and replies `cancelled` to acknowledge receipt. A cooperative handler then emits a terminal `result` with `status: "cancelled"`. The engine enforces cancellation independently: after a bounded grace interval it terminates and reaps the process even if no acknowledgement or terminal result arrives.

### `denied_capability`

This is a terminal response for the bounded request. It names the first denied capability and gives a non-empty reason. No project operation may have started before this response.

### `error`

Protocol, validation, lifecycle, and internal adapter failures use `error`. An error is terminal for the correlated frame. `retryable` is advisory; engine policy decides whether to retry.

## Engine security boundaries

The engine, not the adapter, is authoritative for these controls:

- executable must be an explicitly configured absolute regular file;
- canonical cwd must remain at or below the canonical project root;
- inherited environment is cleared; only configured keys present in the environment allowlist are passed;
- configured process timeout caps request timeout;
- stdout line length is bounded;
- timeout, cancellation, malformed output, and premature exit terminate and reap the child;
- arguments are passed directly to the executable without a shell.

Symlink resolution is part of executable and cwd validation. A lexical path that resolves outside the project root is rejected.

## Compatibility

V1 implementations accept only `arda.project-adapter.v1`. Additive optional fields require a new schema revision because v1 messages reject unevaluated properties. Incompatible protocol versions fail closed before health or execution.