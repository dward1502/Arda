# Payment rail security gate

Status: **offline fixture only; testnet and live funds denied**

Applies to `arda.payment-capability.v1` and the offline x402 fixture. This is a
security gate, not authorization for a wallet, signer, provider client, testnet,
or live-funds deployment.

## Current authority boundary

- `arda-core` validates contracts and deterministic fixtures only.
- There is no network client, RPC endpoint, wallet library, signer, custody
  process, provider credential, testnet adapter, or live adapter in this slice.
- `PaymentSecurityPolicy` accepts only `offline_fixture`.
- `live_funds_authorized` and `automatic_environment_promotion` must both be
  false.
- A successful fixture receipt explicitly records `authorizes_testnet: false`
  and `authorizes_live_funds: false`.
- Any future funds movement remains a separately proposed and approved task.

## Custody and signing

Custody is outside Arda's governance record. The contract stores only an opaque
`redacted:` custody-adapter reference. Unknown fixture fields are rejected, and
secret-like custody references containing private-key, seed-phrase, mnemonic,
or wallet-secret markers fail validation. Prompts, Vairë, logs, and fixture
receipts have no private-key field.

A future signer must be a separate least-privilege adapter that receives the
approved canonical payload, returns a public signature/receipt, and never
returns key material. Its threat model and independent review are prerequisites,
not inferred from this fixture.

## Spending and emergency controls

- Canonical fixed-point amounts avoid floating-point ambiguity.
- Quote amount must fit the per-action limit.
- Prior cumulative spend plus quote amount must fit the cumulative limit.
- Approval is bound to quote ID, terms digest, amount, asset, payee, network,
  expiry, and originating run.
- `emergency_revoked` fails closed before verification.
- Replays are rejected by both idempotency key and challenge nonce.

## Provider, quote, and finality failures

- Provider errors and non-confirmed responses fail closed.
- Amount, asset, recipient, network, quote, challenge, nonce, and expiry are
  matched exactly; substitutions are rejected.
- Quote TTL is bounded by policy.
- Settlement receipts must be non-empty and meet required confirmations.
- The only v1 reorg policy is `revalidate_and_suspend`; loss of finality must
  suspend downstream acceptance rather than silently retain a confirmed state.
- No retry may reuse an idempotency key or nonce.

## Accounting, compensation, and operator visibility

Every contract requires:

- acceptance evidence and artifact receipt references;
- refund/dispute/compensation state plus a process reference;
- accounting classification, tax jurisdiction review marker, and export code;
- explicit budget values and `operator_visibility_required: true`.

This contract exports accounting evidence; it does not autonomously mutate an
accounting system. Operator-facing balance, cumulative spend, pending finality,
and compensation state must exist before any real-rail adapter can be accepted.

## Environment promotion

There is no offline-to-testnet or testnet-to-mainnet transition in v1. Changing
`requested_mode` to `testnet` or `live`, setting `live_funds_authorized`, or
setting `automatic_environment_promotion` fails validation. A future
environment requires a new reviewed adapter, explicit operator approval,
environment-specific budgets, credential/custody review, failure and reorg
tests, accounting review, and an emergency-revoke drill.

## Executable evidence

`cargo test -p arda-core --test payment_capability` covers:

- explicit business-scoped activation and denial to normal tasks;
- exact approval and run binding;
- per-action and cumulative budgets;
- replay, expiry, wrong amount, wrong asset, wrong recipient, and wrong network;
- fixture signature and settlement receipt verification;
- provider failure and insufficient finality;
- emergency revoke and live-rail denial;
- required acceptance, compensation, and accounting fields;
- private-key ingress rejection and secret-free fixture receipts.

These tests prove `payment_fixture_verified` only. They do not satisfy testnet,
live-funds, operator-acceptance, or release-support evidence.
