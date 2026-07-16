---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-07-13"
---

> 🜏 Soterion: 📜 contract | owner: PROMETHEUS | status: active | reviewed: 2026-07-13

# Governance Contract

This contract defines Arda governance as versioned policy-as-code with typed verdicts.

## Scope

Governance is an explicit control plane.
Human override remains first-class.
Operator root is immutable, auditable, and must declare intent+provenance per change.

## Policy entity schema

Stable policy entities:
- `actor`
- `capabilities[]`
- `clearance`
- `resource`
- `action`
- `rule_id`
- `version`
- `signature`

## Verdict schema

Every gatepass or denial emits an immutable receipt:
- `verdict`: `allow|deny|redirect`
- `reasons[]`
- `actor`
- `rule_id`
- `policy_version`
- `policy_hash`
- `timestamp`
- `receipt_id`
- `provenance`

Signature/resonance is trust metadata only.
It must never be used as programmatic truth.

## Evidence classes

| evidence class | meaning |
|---------------|---------|
| `documentation` | policy schema documented |
| `local_heuristic` | one policy.cpp/rs or TOML parser exists |
| `source_metadata` | policy version and hash emitted with each receipt |
| `runtime_receipts` | governance actions write immutable receipts |
| `policy_enforcement` | typed verdict engine accepts policy_version and gates actions |
| `independent_review_receipts` | operator-audited receipts exist for policy changes |
| `scoped_autonomy_policy` | named scope has explicit policy state |

## Current default projection

Expected current level: `policy_enforced` for configured lanes; system-wide default remains `runtime_receipted` until scoped policies are explicit.

## CLI behavior

- `arda governance policy <policy_id>` prints policy source text and version metadata.
- `arda governance receipt <receipt_id>` prints a typed governance receipt.

## Stop condition

Every gatepass or denial writes an immutable contract receipt tied to an exact policy version, policy hash, and actor.

