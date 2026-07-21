# Core

Operational configuration and coordination sources.

## Layout

| Folder | Purpose |
|---|---|
| `state/` | runtime topology and ledger schema |
| `queue/` | queue transport contracts |
| `projects/` | project config documents |
| `realm/` | realm-scoped policy/rules |
| `personal/` | operator-local personalization |
| `clients/` | external client configs |
| `edge/` | edge connectivity config |
| `metrics/` | metrics collection config |
| `knowledge/` | knowledge system setup |

Agents: treat `core/` as source-of-truth policy inputs, not data sinks.
