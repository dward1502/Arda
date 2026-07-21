# arda-hud

Operator HUD for the live **`manwe`** gateway — the local control surface that
surfaces which models are available on the daemon-supervised gateway (port
`7171`, reserved workspace-wide).

## Stack

- Tauri 2 (Rust shell)
- React 19 + TypeScript
- Tailwind CSS v4 (via `@tailwindcss/vite`)
- Vite 8 dev/build

Mirrors the `arda-launcher` clean-root layout: `src-tauri/` holds the native
shell, `src/` holds the React surface.

## What it does

On load it `fetch`es `http://127.0.0.1:7171/v1/models` (OpenAI-compatible, the
same contract `manwe` serves) and lists the available models. A REFRESH button
re-probes the gateway. This is the thinnest possible proof that the HUD ↔
gateway link the `arda` daemon already wires is real and observable.

## Local dev

```bash
pnpm install
pnpm tauri dev      # launches the Tauri window (needs a display)
# or just the web surface:
pnpm dev            # vite dev server on :1421
```

## Build

```bash
pnpm install
pnpm tauri build    # produces the platform bundle under src-tauri/target
```

## Relationship to the rest of Arda

- `arda` (root daemon) supervises `manwe` on `:7171` and reaps it on shutdown.
- `arda-launcher` is the atmospheric onboarding/title screen.
- `arda-hud` is the operator dashboard for the gateway those two produce.

This app is intentionally minimal — it is the seed for the Section 3 UI work
(R3F/3D primitives, fuller MVC) and must not pre-build the deferred remote
("Growth rings") surface described in `docs/REFACTOR_PLAN.md` §5.
