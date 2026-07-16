# Arda

> The slimmed-down continuation of Annunimas: a local-first, auditable agent control plane.

Arda is the operator-facing shell that evolved from Annunimas. It keeps the same control-plane purpose — routing model calls, coordinating agents, recording decisions, gating autonomy, and visualizing operational state — but refactored into a smaller, more composable `arda-*` crate layout. The canonical Rust workspace lives in this repo; `~/Annunimas` remains the live reference architecture and should not be modified unless explicitly requested.

## Repository Map

| Path | Kind | Purpose |
|--- |--- |--- |
| `crates/` | Rust workspace | Shared/host libraries for Arda. |
| `apps/arda-launcher` | Tauri app | The operator desktop launcher (onboarding experience). |
| `apps/arda-launcher/src` | React/TS | Frontend UI, components, styles. |
| `apps/arda-launcher/src-tauri` | Rust/Tauri | Desktop shell, native config, capabilities. |
| `config/` | Config | Operator-managed config and generated runtime env files. |
| `docs/` | Docs | Architecture, operations, plans, and identity docs. |

## Recommended Reading Order

1. This file (`README.md`).
2. `AGENTS.md` — working rules and canonical source layout.
3. `docs/identity/ARDA_IDENTITY.md` — Annunimas-to-Arda identity transfer and operating assumptions.
4. `apps/arda-launcher/README.md` — what the launcher is and how to run it.
5. `apps/arda-launcher/src-tauri/tauri.conf.json` — desktop packaging/config.
