---
sigil: SCROLL
soterion:
  id: soterion-language
  version: 1.0.0
  classification: general-document
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: general
  tags:
    - documentation
    - general
  mnemosyne:
    lineage: soterion-language-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

---
title: Soterion Language Specification
date: '2026-05-03'
soterion:
  tags:
  - soterion-language-specification
  type: architecture
  source: docs
---

# sigil: SCROLL
# Soterion — Unicode Navigation Metadata Layer

## Concept
Soterion is a metadata layer that adds navigational sigils to files and data entries. It gives humans, agents, and runtime cleanup jobs a compact shared language for routing, retention, and review.

The design is intentionally two-layered:
- Stable machine layer: names such as `COIN`, `SG_ROUTE_FAILOVER`, IDs, tags, severity, and retention.
- Unicode flow layer: rendered glyphs such as `🪙`, `☥`, `📜`, and machine-signature strings derived from the registry.

The machine layer is canonical for safety. The Unicode layer is canonical for compact human-machine flow and targeted grep.

## Planned YAML Header Format
```yaml
---
soterion:
  sigil: "📜"
  realm: "knowledge"
  tags: [ecst, solar]
---
```

For first-line file lifecycle markers, use the comment syntax that matches the file:

```text
# 🪙
// 🪙
<!-- 🪙 -->
```

`🪙` is equivalent to legacy `sigil: COIN` and is intended for completed cleanup/archive targeting.

## File Lifecycle Sigils
Defined in `meta/soterion_sigils.yaml` under `file_lifecycle_sigils`.

| Name | Glyph | Code Point | Meaning |
|------|-------|------------|---------|
| ANKH | ☥ | U+2625 | executable continuity / operational action |
| EYE | 👁 | U+1F441 | observation / review / ingest |
| SCROLL | 📜 | U+1F4DC | documentation / reference |
| COIN | 🪙 | U+1FA99 | completed or cleanup/archive targeting |
| REPAIR | 🛠 | U+1F6E0 | active repair or implementation surface |
| ORPHAN_TEMP | ⌛ | U+231B | temporary orphan under observation |
| QUARANTINE | ⛔ | U+26D4 | unsafe or blocked artifact |
| CONDEMNED | ☠ | U+2620 | approved destructive removal target |
| ARCHIVED | 🗄 | U+1F5C4 | archived continuity artifact |

## Sigil Dictionary (Draft)
| Sigil | Meaning | Realm |
|-------|---------|-------|
| 𓁿 | Knowledge / Memory | data/knowledge |
| 𓂀 | Observation / Ingestion | agents/athena |
| 𓆣 | Energy / JouleWork | core/ledger |
| 𓀀 | Command / CEO | ceo |
| 𓋹 | Human Override | human |

## Implementation Plan
- Keep `meta/soterion_sigils.yaml` as the source of truth for glyph/codepoint/reference meaning.
- Let HADES and compaction code resolve both Unicode glyphs and legacy text aliases through the registry.
- Use Unicode glyphs for concise operator streams and first-line lifecycle targeting.
- Keep stable machine IDs for logs, receipts, and safety decisions.
- Use `docs/contracts/soterion-communication-contract.md` and `core/state/soterion_communication_contract.json` for A2A, Discord, HUD, and agent-user message headers.


## See Also
