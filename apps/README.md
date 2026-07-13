# apps

Operator-facing applications that sit on top of the Arda / ARDA core.

## Index

| App | Stack | Purpose |
| --- | --- | --- |
| [`arda-launcher`](./arda-launcher) | Tauri 2 + React + TypeScript + Tailwind v4 | The operator desktop launcher — the atmospheric onboarding / title screen for the ARDA ecosystem. |

## Conventions

- Each app is independently buildable; `arda-launcher` is the active one.
- Apps depend on shared logic from `crates/arda-core` where it exists.
- Documentation per app lives in its own `README.md` (and `src-tauri/README.md`
  for the native Tauri surface).
