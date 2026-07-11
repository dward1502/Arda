# arda-launcher

Tauri + React + TypeScript desktop launcher for the Arda HUD.

- React 19 frontend rendered in a Tauri 2 window
- Three.js scene via React Three Fiber / Drei / postprocessing
- Rust backend in `src-tauri/` with Tauri commands and system integration

## Requirements

- Node.js 20+
- pnpm
- Rust stable + system build tools
- Tauri 2 prerequisites for your platform:
  https://tauri.app/start/prerequisites/

## Install

```
pnpm install
```

## Run

- Frontend only:
  ```
  pnpm dev
  ```
- Full Tauri window:
  ```
  pnpm tauri
  ```

Default Vite dev server port: 1420.

## Build

```
pnpm build
pnpm tauri build
```

## Project layout

```
src/
  App.tsx
  components/
    Background.tsx
    ParticleSmoke.tsx
    WorldTree.tsx
    OnboardingText.tsx
    ArdaLogo.tsx
  styles/
src-tauri/
  src/
    main.rs
    lib.rs
  capabilities/
  tauri.conf.json
```

## Notes

- CSS/theming is in `src/styles/` and Tailwind via `@tailwindcss/vite`.
- Rust entrypoint: `src-tauri/src/main.rs` -> `arda_launcher_lib::run()`.
- Existing backend command example: `greet` in `src-tauri/src/lib.rs`.
