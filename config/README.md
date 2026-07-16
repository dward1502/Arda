# Config

Operational configuration for Annunimas runtime environments.

## Layout

| Folder | Purpose |
|---|---|
| `governance/` | gate/runtime toggles and policy config |
| `systemd/` | system unit files and overrides |
| `monitoring-setup/` | monitoring and alert rules |

Convention: all configs are environment-aware. Default values are documented;
overrides belong in local env files, not committed configs.
