# arda-launcher

Tauri + React + TypeScript desktop entry point for the Arda runtime.

This project is in active transition. Its formal purpose is replacing and rebranding the Annunimas architecture piece by piece. That means the current codebase is expected to undergo cycles of deletion, addition, and renaming as subsystems are ported and verified individually.

## What this application is supposed to do

- On first launch: detect missing Arda subsystems and run their setup flow.
- On later launches: detect the existing Arda installation state and run the appropriate onboarding path.
- Present a single coherent desktop surface instead of requiring separate terminals or scripts for each subsystem.

## What is in scope for Arda (porting from Annunimas)

- fleet and node topology
- network mesh / provider routing
- LLM provider config and fallbacks
- memory and recall systems
- core runtime state and runtime persistence
- operator diagnostics and logging surfaces

## Current state

- Frontend: React 19 + Three.js intro/onboarding UI.
- Backend: Tauri shell with stub Rust commands. No subsystem probing or setup logic yet.
- Wiring between UI and runtime: not implemented.

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
- Long-term expectation: Rust backend moves from placeholder to real system detection and setup orchestration.
