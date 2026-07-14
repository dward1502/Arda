# Arda

> The single entry point that unifies the ARDA ecosystem for the operator.

Arda is the operator-facing shell for the broader ARDA project. Today it is a
freshly scaffolded monorepo containing a **Rust workspace** and an
and an **arda-launcher** desktop application (Tauri 2 + React + TypeScript +
Tailwind v4) that renders the ARDA onboarding / world-tree experience. The rust
packages are placeholder libraries; the launcher is the only executable surface
so far and is where active development lives.


## Repository Map

| Path | Kind | Purpose |
| --- | --- | --- |
| `crates/` | Rust workspace | Shared/host libraries for Arda. |
| `crates/arda-core` | Rust lib | Placeholder core library (scaffold). |
| `crates/README.md` | Docs | Index of crates + ARDA-wide crate conventions. |
| `apps/arda-launcher` | Tauri app | The operator desktop launcher (onboarding experience). |
| `apps/arda-launcher/src` | React/TS | Frontend UI, components, styles. |
| `apps/arda-launcher/src-tauri` | Rust/Tauri | Desktop shell, native config, capabilities. |




## Recommended Reading Order

1. This file (Arda central README).
2. `apps/arda-launcher/README.md` — what the launcher is and how to run it.
3. `apps/arda-launcher/src-tauri/tauri.conf.json` — desktop packaging/config.

