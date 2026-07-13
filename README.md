# Arda

> The single entry point that unifies the ARDA ecosystem for the operator.

Arda is the operator-facing shell for the broader ARDA project. Today it is a
freshly scaffolded monorepo containing a **Rust workspace** (`arda` + `arda-core`)
and an **arda-launcher** desktop application (Tauri 2 + React + TypeScript +
Tailwind v4) that renders the ARDA onboarding / world-tree experience. The rust
packages are placeholder libraries; the launcher is the only executable surface
so far and is where active development lives.

## Vision

One calm, beautiful surface that boots the operator into the ARDA world and
gates the way to the surrounding services (agent loop, tool gate, service
registry, signal grid, council, and HUD). The launcher is meant to feel less
like "an app" and more like the title screen of a world — a starfield, a
growing tree, and a single call to action.

Long term, the `arda-core` crate is intended to host shared logic (domain types,
receipts, config) that the launcher and the other ARDA repos can depend on.

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

