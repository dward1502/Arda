# System Inventory

Generated from current fleet/state and core configuration.

## Fleet Nodes

|| Host | Address | Services |
|| --- | --- | --- |
|| beelink | 100.103.125.88 | llama.cpp Prism :9337 (`Ternary-Bonsai-8B-Q2_0`, context=32768) |
|| warden | 100.110.85.37 | llama.cpp :1234 (`Qwen3.5-4B-Q4_K_M.gguf`, context=65536 validated on Pi5) |
|| annunimas-server | 100.102.250.115 | llama.cpp :8093 LFM Fast General, :8094 Qwen Coder |

## Annunimas Agents / Services

Sources inspected:
- `core/realm/agents.toml`
- `core/state/fleet_nodes.json`
- `core/state/*`
- local system services inventory

Record current state here after service and agent checks on the target fleet.