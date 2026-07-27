---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "interface_contract"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-26"
---

> 🜏 Soterion: ⟁ interface_contract | owner: HADES | status: active | reviewed: 2026-07-26

# arda-orome

Typed communication, provider-dispatch, governance-recording, and gRPC contracts for the Arda
spine.

## Current scope

The compiled production library exposes:

- A2H and A2A message/envelope types;
- ledger-backed task-approval and interruption governance records;
- provider adapter, registry, streaming, timeout/retry, fanout, fleet-scope, metrics, and receipt
  contracts;
- generated health-model and route-governance gRPC clients, servers, and messages;
- typed boardroom, council, approval, completion, interruption, and operator event payloads.

Manwe owns provider selection and inference-routing policy. `arda-orome` owns the bounded
transport-facing contracts and outcome receipts, not daemon lifecycle or model choice.

## Public modules

| Module | Contract |
|---|---|
| `comm` | A2H messages, channels, priorities, attachments, responses, and bounded queue |
| `governance` | `GovernanceHooks` and ledger-backed policy decisions |
| `grpc` | Generated health-model and route-governance tonic surfaces |
| `message` | A2A messages, threads, TTL, signatures, and hop envelopes |
| `provider` | Provider adapters, registry, dispatch orchestration, streaming, and receipts |
| `types` | Shared interface and operator payload schemas |

`intent`, `registry`, `router`, and `message_retry_expiry` are currently compiled only for unit
tests. They are not public production modules.

## Provider integration

Implement `provider::ProviderTransport`, register a `ProviderConfig`, and dispatch through
`ProviderRuntime`. Do not bypass runtime timeout, retry, expiry, fanout, fleet-scope, metrics, or
receipt behavior. `ManualTransport` is deterministic and no-network; it is not a production
provider client.

## Consumers

- `arda-engine` re-exports provider contracts and runs the deterministic smoke dispatch.
- Manwe's `grpc` feature implements and serves the generated gRPC contracts.
- `arda-aule`'s `full-cli` feature consumes the A2H message surface.

## Stability boundary

The compiled/default and no-default-feature surfaces pass their gates. The repository tree is not
yet fully source-clean: 35 Rust files are not reachable from `lib.rs`, and the `http` feature
enables dependencies without enabling an HTTP module. These are recorded as active stabilization
work in `PLAN.md`; unwired files must not be presented as supported crate behavior.

## Verification

Current dated commands, test counts, consumer checks, and known boundaries are in `STATUS.md`.

## Documentation

- `STATUS.md` — current stability and verification evidence.
- `BREAKDOWN.md` — compiled/test-only/unwired implementation map.
- `PLAN.md` — active stabilization decisions and future proposals.
- `OWNERSHIP.md` — authority and non-ownership boundaries.
- `INDEX.md` — deterministic crate navigation.
