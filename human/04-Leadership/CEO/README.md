---
title: "CEO Role Folder"
date: 2026-05-03
created: 2026-05-03T06:15:00Z
tags:
  - soterion/core
  - soterion/agentic
  - soterion/type/knowledge
  - soterion/domain/agents
  - soterion/project/annunimas
  - soterion/role/ceo
  - soterion/priority/high
soterion:
  version: 1.0
  language: markdown
  agent_access: public
  mnemosyne:
    priority: high
    retention: long-term
    related_to:
      - "soterion/role/ceo"
      - "soterion/project/annunimas"
      - "soterion/domain/agents"
  metadata:
    author: mythos
    source: human-vault
    format: yaml
    encoding: utf-8
related:
  - "[[CEO/beliefs]]"
  - "[[CEO/thoughts]]"
  - "[[CEO/counsel]]"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# CEO Role Folder

This is the human-facing thought surface of the top-of-system orchestrator.

The **role** is `CEO`. The **persona** currently embodying it is `arandur`
(see `core/realm/annunimas.toml [orchestrator]`). If the persona is renamed
or replaced, this folder's role binding does not change — the agent_id does.

## Purpose

This folder is the machine-human bridge for the orchestrator. It is
Obsidian-linked so that Daniel (the human principal, `𓋹`) and the CEO
agent can share a single working notebook without either side losing
authorship.

### What lives here

- [`thoughts.md`](thoughts.md) — readable executive thought stream.
  What the system is noticing. What is blocked. What matters next.
  Written by the orchestrator; read by the human.

- [`beliefs.md`](beliefs.md) — working world model.
  What the orchestrator currently holds true about the business,
  the fleet, the clients, and the trajectory. Updated as evidence
  arrives. The human is invited to challenge anything in here.

- [`counsel/`](counsel/) — sub-agent advisor folder (first sub-agent
  under the orchestrator role).

## Boundaries

- This is **readable executive synthesis**, not raw chain-of-thought.
- Sub-agents under the orchestrator keep their own sub-folders here.
- Content that is strictly personal to Daniel belongs in `human/Notes/`,
  not here.
- Machine-written entries should be signed (agent id + UTC timestamp).
  Human-written entries need no signature — the human is the principal.

## Soterion marks used in this folder

- `𓀀` — orchestrator speaking
- `𓎛` — counsel speaking
- `𓋹` — human principal speaking (Daniel)
- `∇` — sovereign statement (self-governed)
- `◇` — decision point
- `◈` — verifiable truth claim
- `⚡` — joule cost noted

## Renaming the role

To rename from `CEO` to another label (e.g. `Conductor`, `Chief`):

1. Edit `core/realm/annunimas.toml` → `[orchestrator].role_name` and
   `[orchestrator].folder`.
2. Move this folder: `mv human/CEO human/<new_folder>`.
3. The `[orchestrator].agent_id` persona is a separate variable — the role
   and the persona can be changed independently.
