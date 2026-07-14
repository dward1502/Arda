# apps

Operator-facing applications that sit on top of the Arda / ARDA core.

## Index

| App | Stack | Purpose |
| --- | --- | --- |
| [`arda-launcher`](./arda-launcher) | Tauri 2 + React + TypeScript + Tailwind v4 | The operator desktop launcher — the atmospheric onboarding / title screen for the ARDA ecosystem. |
| [`arda-hud`](./arda-hud) | Tauri 2 + React + TypeScript + Tailwind v4 | Operator dashboard for the live `manwe` gateway (port 7171) — surfaces available local models. |

## Conventions

- Each app is independently buildable; `arda-launcher` and `arda-hud` are the active ones.
- Apps share conventions, not forced coupling through `crates/arda-core`; shared Rust types should live in dedicated crates or in `engine`/`manwe` when they're not cross-cutting.
- Documentation per app lives in its own `README.md` (and `src-tauri/README.md`
  for the native Tauri surface).
