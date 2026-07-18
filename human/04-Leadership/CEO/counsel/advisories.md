---
soterion:
  core: true
  agentic: true
  type: resource
  project: [annunimas, zero-human, mnemosyne, soterion, hermes, ceo]
  priority: high
  tags: [counsel, advisories, append-only, formal-advice, orchestrator-communication, decision-record]
  language: markdown
  format: yaml-frontmatter
  version: 1.0.0
  created: 2026-05-03T22:11:00Z
  last_edited: 2026-05-03T22:11:00Z
  source: human/CEO/counsel/advisories.md
  validation:
    yaml_syntax: valid
    frontmatter_complete: true
    links_resolved: true
---

# Counsel Advisories — Append-Only Ledger

`𓎛 ∇` — Formal advisories issued to the orchestrator.

Each advisory has four fields. **Stance** is what Counsel believes.
**Evidence** is what makes Counsel believe it. **Counter** is the
strongest case against Counsel's own stance (devil's-advocate on
Counsel's own position — the test of a serious advisor). **Recommendation**
is what Counsel suggests the orchestrator do with this advisory.

Advisories are **append-only**. Once issued they stay in the record even
if later overruled. If Counsel's stance updates, a new advisory
references the prior one by id.

## Format

```
## ADV-YYYYMMDD-NN — <one-line subject>
- **Decision addressed:** <which orchestrator decision / task>
- **Trigger:** <joule_cost | confidence | irreversible | client_impact | discretionary>
- **Stance:** <what Counsel holds>
- **Evidence:** <citations, timestamps, prior decisions referenced>
- **Counter:** <strongest argument against Counsel's own stance>
- **Recommendation:** <what Counsel asks the orchestrator to consider>
```

## Advisories

_(no advisories issued yet)_
