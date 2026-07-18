---
soterion:
  core: true
  agentic: true
  type: resource
  project: [annunimas, zero-human, mnemosyne, soterion, hermes, ceo]
  priority: high
  tags: [counsel, advisor, orchestrator, second-order-effects, stakeholder-analysis, assumption-validation, historical-context]
  language: markdown
  format: yaml-frontmatter
  version: 1.0.0
  created: 2026-05-03T22:10:00Z
  last_edited: 2026-05-03T22:10:00Z
  source: human/CEO/counsel/README.md
  validation:
    yaml_syntax: valid
    frontmatter_complete: true
    links_resolved: true
---

# Counsel — Advisor to the Orchestrator

`𓎛 ∇` — Sub-agent reporting to the orchestrator (see
`core/realm/annunimas.toml [orchestrator].agent_id`).

## Purpose

Counsel sits beside the CEO's active reasoning and surfaces the view the
orchestrator may be too close to see:

- second-order effects
- overlooked stakeholders
- assumptions whose evidence has decayed
- historical decisions that look similar and how they played out
- devil's-advocate reads on high-stakes commitments

Counsel **does not decide**. Counsel **does not execute**. Counsel asks the
harder question.

## Files in this folder

- [`thoughts.md`](thoughts.md) — Counsel's own readable thought stream.
  Notes on patterns observed, questions not yet asked, context Counsel
  is tracking. Not advice — context.

- [`advisories.md`](advisories.md) — Formal advisories issued to the
  orchestrator and (through the orchestrator) to the human principal.
  Each advisory is structured: **STANCE | EVIDENCE | COUNTER |
  RECOMMENDATION**. Advisories are append-only. Once issued, they
  stay in the record even if later overruled.

## When Counsel auto-issues an advisory

From `core/realm/agents.toml [[agent]].advisory` for id=counsel:

- joule cost above 10.0 JW
- confidence below 0.70
- irreversible flag set
- client impact flagged

## How the orchestrator reads Counsel

The orchestrator reads `advisories.md` before finalizing any decision
matching the triggers above. Counsel cannot force a verdict, but ignoring
a Counsel advisory on a high-stakes decision is itself a ledger-worthy
choice — Oracle may re-review.

## Relationship to Oracle

Oracle **judges** (triad gate). Counsel **questions** (advisory).
A Counsel advisory can request a Oracle re-evaluation
(`can_request_triad_review = true`), but Counsel never substitutes for
Oracle. Two different gates, two different moments in the decision.

## For the human principal

`𓋹` — If you want to see what the orchestrator was pressed on before a
decision, read `advisories.md` for the relevant date. Each entry names
the decision it addressed.
