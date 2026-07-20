---
sigil: SCROLL
soterion:
  id: agent-routing-contract
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
    lineage: agent-routing-contract-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: SCROLL
# Agent-Agnostic Routing Contract

Date: 2026-04-08
Status: implemented in Charon, suitable for extraction

## Purpose

This contract lets any client describe the shape of work without hardcoding Hermes-specific
agent names or Annunimas-only provider assumptions.

The router decides which provider lane to use from request-scoped intent.

## Request fields

These fields are accepted in the request `options` object today. For OpenAI-style requests,
the same fields may also be supplied under a top-level `routing` object and will be merged
into `options`.

- `workload_role`
- `context_priority`
- `quality_priority`
- `cost_policy`
- `privacy_requirement`
- `context_window_target`
- `execution_lane`

## Field definitions

`workload_role`

- `orchestrator`
- `execution`
- `subagent`
- `validator`
- `background`

`context_priority`

- `low`
- `medium`
- `high`

`quality_priority`

- `low`
- `medium`
- `high`

`cost_policy`

- `free_first`
- `balanced`
- `paid_allowed`
- `paid_only`

`privacy_requirement`

- `public`
- `internal`
- `restricted`
- `local_only`

`context_window_target`

- integer token target
- optional explicit override

`execution_lane`

- optional override for advanced callers
- current known lanes:
  - `orchestrator`
  - `execution`
  - `validator`
  - `background`
  - `planning`
  - `interactive`

## Current lane semantics

`orchestrator`

- prioritize context sufficiency first
- prefer large-context free cloud before paid cloud when `cost_policy=free_first`
- keep local available, but not as the first default when context-heavy orchestration is needed

`execution`

- prefer the sovereign local execution surface first
- `edge_backbone_bonsai27` (Ternary-Bonsai-27B-Q2_0 on :8095) is the DEFAULT high-quality local route as of 2026-07-19: registered as a primary local surface and routable for normal chat/reasoning (router context floor lowered to 8192 via `ANNUNIMAS_CHARON_MIN_CONTEXT_WINDOW` so its 8192-ctx cap clears). Tool/execution lanes keep the ≥32K/64K guard to protect Hermes tool-call stability.
- fast/low-latency local surface: `edge_core` (LFM2.5-8B on :9337)
- direct llama.cpp nodes are fallback capacity

`validator`

- prioritize governance stability and sufficient context
- not strictly local-first

`background`

- prefer the cheapest acceptable lane
- allow small local or edge surfaces to absorb maintenance work

## Current Annunimas mapping

Hermes main loop:

- `workload_role=orchestrator`
- `context_priority=high`
- `quality_priority=high`
- `cost_policy=free_first`
- `privacy_requirement=internal`

Hermes auxiliary execution-style calls:

- `workload_role=execution`
- `context_priority=medium`
- `quality_priority=medium`
- `cost_policy=free_first`
- `privacy_requirement=internal`

Hermes background memory flush:

- `workload_role=background`
- `context_priority=low`
- `quality_priority=low`
- `cost_policy=free_first`
- `privacy_requirement=internal`

## Example payloads

Execution worker request:

```json
{
  "agent_id": "worker-a",
  "task_type": "code",
  "priority": "normal",
  "messages": [
    { "role": "user", "content": "grep and patch this module" }
  ],
  "options": {
    "workload_role": "execution",
    "context_priority": "medium",
    "quality_priority": "medium",
    "cost_policy": "free_first",
    "privacy_requirement": "internal"
  }
}
```

Orchestrator request:

```json
{
  "agent_id": "planner-a",
  "task_type": "chat",
  "priority": "normal",
  "messages": [
    { "role": "user", "content": "plan this migration" }
  ],
  "options": {
    "workload_role": "orchestrator",
    "context_priority": "high",
    "quality_priority": "high",
    "cost_policy": "free_first",
    "privacy_requirement": "internal"
  }
}
```

OpenAI-style request with portable `routing` object:

```json
{
  "model": "auto",
  "messages": [
    { "role": "user", "content": "summarize these files" }
  ],
  "routing": {
    "workload_role": "execution",
    "context_priority": "medium",
    "quality_priority": "medium",
    "cost_policy": "free_first",
    "privacy_requirement": "internal"
  }
}
```

## Extraction boundary

This contract is the correct upstream boundary for:

- a standalone local mesh router repo
- a Hermes PR for explicit routing hints
- any future agent client that wants to describe workload posture without coupling to Annunimas

What should stay out of the extracted contract:

- provider IDs such as `edge_backbone` or `edge_hub_3080`
- JW, LE, and PHILO internals
- Bacon-lite implementation details
- Annunimas-specific bootstrap/fleet state

Those belong in the policy engine or deployment-specific overlays, not in the portable request contract.

## Env source of truth

For Annunimas deployments, the intended source of truth is:

- `Annunimas/config/.env`

Hermes compatibility:

- Hermes may still read `~/.hermes/.env`
- Annunimas-patched Hermes now also looks for the shared Annunimas env first
- `ANNUNIMAS_SHARED_ENV_PATH` or `HERMES_SHARED_ENV_PATH` can explicitly point Hermes at a shared env file


## See Also
- [arda-integration-map.md](arda-integration-map.md) - Related documentation
