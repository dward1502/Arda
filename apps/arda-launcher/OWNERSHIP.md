# arda-launcher ownership

## Owned here

- Native launcher window, capability policy, icons, and package configuration.
- Atmospheric operator entry scene and registry-gated **Begin** interaction.
- Tauri serialization boundary for registry, readiness, and service-plan status.
- Environment-profile discovery of operator paths and configured service URLs.
- Read-only onboarding panel and visible command failure state.
- Prerequisite, device, provider, guided-session, and private-config proposal
  types and helpers internal to launcher onboarding.

## Owned elsewhere

- Canonical governance/task types: `arda-core`.
- Contract registry format and validation: `arda-contract-registry`.
- Manwe runtime bind/default endpoint: Manwe and fleet/runtime configuration.
- Root service composition and process supervision: root `arda` daemon/engine.
- Provider credentials and private environment values: operator-owned private
  configuration; never frontend display data.
- Service approval, queue mutation, execution, and durable approval receipts:
  governance/runtime owners, not the read-only frontend command surface.
- AppImage `linuxdeploy`/`strip` compatibility: external Tauri packaging
  toolchain and target build environment.

## Authority boundary

The registered commands discover and project current state. A `ServicePlan`
action may say that a human gate is required, but returning or displaying that
action does not approve or execute it. The backend's apply and private-config
write functions remain unregistered. Any future mutation command must require an
explicit operator action, preserve approval receipts, and add backend plus
frontend contract tests before being exposed.

Changing the workspace's `:7171` compatibility default is a cross-owner fleet
migration. Launcher may consume environment-discovered endpoints but must not
silently redefine the shared default for other consumers.
