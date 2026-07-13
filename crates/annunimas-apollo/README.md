---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: annunimas-apollo
kind: agent
agent: apollo
realm: operations
sigil: "𓋹"
capabilities:
  - workflow-execution
  - task-optimization
  - execution-priority
  - phi-calibration
status: operational
search_tags: [agent, apollo, execution, workflow, optimizer]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: operational | reviewed: 2026-05-21

# annunimas-apollo

Execution and workflow engine for operational task dispatch.

## Purpose
Provide execution planning, runtime execution dispatch, workflow progression,
and optimization components for operational tasks.

## What's in this crate
- `executor.rs`: execution request/result structures and runtime execution flow.
- `service.rs`: persistence, runtime status, and execution service layer.
- `transport/`: HTTP and IPC daemon surfaces.
- `workflow.rs`: workflow graph/state machinery.
- `rtk.rs`: optimizer and dependency handling.
- `phi.rs`: calibration helpers.
