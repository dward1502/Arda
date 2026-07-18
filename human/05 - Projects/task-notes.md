---
title: "Task Notes"
last_updated: 2026-05-14
soterion:
  type: project_summary
  category: summaries
  project: annunimas
  agent_access: public
  mnemosyne_priority: high
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Task Notes - Annunimas Memory Integration

## Current Task
Phase 1 Week 1: Modify Hermes to query Mnemosyne before subagent spawning

### Problem
- main.rs is too large (5899 lines) to process linearly
- Stopping and delegating is the correct approach

### Next Steps
1. Delegate finding subagent spawning logic in main.rs
2. Use search_files to find patterns like "spawn", "subagent", "Mnemosyne"
3. Create a code flow map for task execution

### Status
- [x] Recognized the file size issue
- [x] Delegated analysis task
- [ ] Need to wait for delegation results
- [ ] Then modify Hermes to query Mnemosyne before spawning subagents

### Files Already Read
- core/clients/_registry.toml
- core/edge/README.md
- core/edge/model_profiles.toml
- core/edge/targets.toml
- core/personal/personal-identity.toml
- core/projects/_registry.toml
- core/realm/agents.toml (all sections)
- core/realm/annunimas.toml
- core/realm/boot.toml
- Various crate lib.rs files
- docs/architecture.md
- docs/governance/AGENTS.md
- scripts/*.sh and *.py files

### Key Insight
The system uses IPC (Inter-Process Communication) between daemons:
- Prometheus, Charon, Hermes, Hades, Athena, Mnemosyne, Oracle, Plutus
- All communicate via socket files in /run/user/1000/
- Need to understand the orchestration flow before modifying
