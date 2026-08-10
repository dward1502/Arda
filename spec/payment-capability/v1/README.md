# Payment capability v1

`arda.payment-capability.v1` is the optional, business-scoped contract for a
single approved payment attempt. Its executable Rust representation is
`arda_core::payment_capability::PaymentCapabilityContract`.

## Contract boundary

A valid contract carries:

- project, objective, and run lineage;
- rail, provider, network, asset, payer, and payee identities;
- an exact quote with canonical fixed-point amount, expiry, and terms digest;
- per-action and cumulative spending limits;
- an acceptance condition and artifact receipts;
- idempotency key and challenge nonce;
- an approval receipt bound to the exact quote, terms, amount, payer, payee,
  network, asset, and run;
- settlement status, receipt, provider reference, and confirmations;
- refund/dispute/compensation state;
- accounting/tax export classification;
- a redacted custody-adapter reference; and
- a fail-closed security policy.

The capability must be explicitly selected as `payment` by a validated
business-scoped capability composition. It is not ambient on normal personal or
software tasks. Verification requires the current composition and its exact
`RunGraph`; a deserialized contract cannot self-assert activation. The versioned
replay ledger can be persisted and reloaded so idempotency keys and challenge
nonces remain blocked across restart.

## Offline x402 fixture

`fixtures/offline-x402.json` provides a deterministic quote, challenge, and
payment response. The `offline-fixture-sha256-v1` signature is an integrity
check at the fixture boundary only. It is not a wallet signature and must never
be represented as proof that a real x402 provider, testnet, custody system, or
live rail works.

Successful verification emits `arda.payment-fixture-receipt.v1` with
`payment_fixture_verified: true`, `authorizes_testnet: false`, and
`authorizes_live_funds: false`.

Unknown fields are rejected. Private keys, seed phrases, wallet secrets, and
mnemonics have no contract or receipt field and are rejected at fixture ingress.

See `docs/security/payment-rail-security-gate.md` before introducing any signer,
provider client, testnet, or live-funds path.
