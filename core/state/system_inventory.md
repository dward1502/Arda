# System Inventory

Generated from current fleet/state and core configuration.

## Fleet Nodes

| Host | Address | Services |
| --- | --- | --- |
| beelink | 100.103.125.88 | Qwen3.5-4B :9337, Grafana :3000, Prometheus :9090, OpenWebUI :8080, MinIO :9000 |
| warden | 100.110.85.37 | llama.cpp :1234 |
| annunimas-server | 100.102.250.115 | 3 LLM lanes (:8093, :8094, :8081) |

## Annunimas Agents / Services

Sources inspected:
- `core/realm/agents.toml`
- `core/state/fleet_nodes.json`
- `core/state/*`
- local system services inventory

Record current state here after service and agent checks on the target fleet.