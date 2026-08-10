---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "interface_contract"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-27"
---

# arda-orome

Typed communication, provider-dispatch, governance-recording, gRPC, and opt-in resident-service contracts for the Arda spine.

## Default surface

The default build exposes:

- `comm`: A2H messages, channels, priorities, attachments, responses, and bounded queues;
- `governance`: ledger-backed approval and interruption decisions;
- `grpc`: generated health-model and route-governance tonic surfaces;
- `message`: A2A messages, threads, TTL, signatures, and envelopes;
- `operator_bridge`: credential-free Hermes `MessageEvent` normalization,
  exact-once operator-session persistence, approval replay protection,
  audience redaction, correlated responses, and transport health;
- `provider`: adapter registry, bounded dispatch, fanout, fleet policy, streaming, metrics, receipts, and an opt-in receipt-backed HTTP JSON transport;
- `types`: boardroom, council, approval, completion, interruption, and operator payloads.

Manwe owns provider/model selection and inference-routing policy. `arda-orome` owns transport-facing contracts and outcome receipts.

## `service-runtime` feature

`service-runtime` deliberately compiles the preserved resident-service migration surface:

- `agent`, `service`, and the service child modules;
- MCP browser/channel/protocol/server/tool support;
- context cache/enrichment and Mnemosyne integration;
- Discord health and safe-message contracts;
- the resident protocol and registry support required by that closure.

The feature is opt-in. Its historical resident-service compatibility dispatch remains deterministic/no-network `ManualTransport`; configured providers are reported offline unless a test channel proves health. Governance, memory, and resource evidence emitted by `HermesService` is rooted in the configured service project (or explicit `ARDA_MNEMOSYNE_HOME` / `ARDA_PLUTUS_HOME` overrides), rather than process-global current-directory defaults. The provider API additionally exposes `HttpJsonTransport` for explicitly configured, policy-gated live dispatch. Serenity, formatter, relay, slash-command, legacy edge, and unused generated-contract shims were retired rather than advertised as working integrations.

## Integration

Implement `provider::ProviderTransport`, register a `ProviderConfig`, and dispatch through `ProviderRuntime`. Do not bypass timeout, retry, expiry, fanout, fleet-scope, metrics, or receipt behavior.

For phone/chat ingress, project Hermes Agent's normalized `MessageEvent` into
`operator_bridge::BridgeRequest`; never pass `raw_message`, free-form metadata,
or credentials. Approval ingestion additionally requires an
`ApprovalBinding` loaded separately from canonical Arda state, so transport JSON
cannot mint its own pending approval. The engine consumer is
`arda_engine::orome::OromeOperatorRuntime`.

`HttpJsonTransport` is available with `service-runtime`. It posts a bounded JSON envelope containing `message_id`, `payload`, `streaming`, and `fleet_scope` to an HTTP provider endpoint. A successful response must contain a non-empty `message_id`; optional `chunks` become stream events. Redirects are disabled, response bodies are bounded, and HTTP success without a provider message ID is treated as failure rather than delivery proof.

Fleet policy is fail-closed:

- `Local` is the only default scope;
- `TrustedFleet` requires both the scope and every target provider ID to be explicitly allowlisted;
- `External` requires the scope plus operator approval;
- operators must review the endpoint associated with every allowlisted provider ID because configuration remains deployment authority.

`DispatchReceipt::delivery_proven()` is true only when dispatch succeeded and the concrete transport returned a provider message ID. `ManualTransport` never satisfies that condition.

Direct consumers:

- `arda-engine`: provider contracts and deterministic smoke dispatch;
- Manwe `grpc`: generated gRPC contracts;
- `arda-aule` `full-cli`: A2H contracts.

## Documentation

- `STATUS.md` — current evidence and remaining boundaries.
- `BREAKDOWN.md` — exact module classification and invariants.
- `OWNERSHIP.md` — authority boundaries.
- `INDEX.md` — deterministic navigation.

The completed stabilization plan was retired after its durable source-tree,
feature, transport, and fleet-policy decisions were absorbed into the maintained
documents above. Future capability work requires a new plan with a concrete
owner and consumer.
