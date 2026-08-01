# arda-launcher index

## Canonical documents

- [`README.md`](README.md) — app purpose, command contract, development, and status.
- [`STATUS.md`](STATUS.md) — current first-class state and verification evidence.
- [`BREAKDOWN.md`](BREAKDOWN.md) — backend/frontend module and consumer breakdown.
- [`OWNERSHIP.md`](OWNERSHIP.md) — launcher-owned concerns and authority limits.
- [`src-tauri/README.md`](src-tauri/README.md) — native-shell orientation.
- [`src-tauri/src/onboarding/README.md`](src-tauri/src/onboarding/README.md) — onboarding subsystem guide.
- [`src-tauri/src/onboarding/INDEX.md`](src-tauri/src/onboarding/INDEX.md) — onboarding module index.

## Frontend entry points

- [`package.json`](package.json) — pnpm test/lint/build/Tauri scripts.
- [`pnpm-lock.yaml`](pnpm-lock.yaml) — reproducible frontend dependency resolution.
- [`index.html`](index.html) — Vite HTML entry point.
- [`tsconfig.json`](tsconfig.json) — frontend TypeScript project configuration.
- [`tsconfig.node.json`](tsconfig.node.json) — Vite/Node TypeScript configuration.
- [`vite.config.ts`](vite.config.ts) — React, Tailwind, dev-server, and HMR config.
- [`public/`](public/) — static launcher assets.
- [`src/main.tsx`](src/main.tsx) — React root.
- [`src/App.tsx`](src/App.tsx) — registry gate and onboarding state flow.
- [`src/components/OnboardingPanel.tsx`](src/components/OnboardingPanel.tsx) — readiness/service-plan projection.
- [`src/lib/tauri-core-compat.ts`](src/lib/tauri-core-compat.ts) — typed command boundary.
- [`src/lib/tauri-core-compat.test.ts`](src/lib/tauri-core-compat.test.ts) — frontend command-contract tests.
- [`src/scenes/`](src/scenes/) — atmospheric onboarding scene.

## Native entry points

- [`src-tauri/Cargo.toml`](src-tauri/Cargo.toml) — Rust package and dependencies.
- [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json) — desktop and bundle config.
- [`src-tauri/capabilities/default.json`](src-tauri/capabilities/default.json) — webview permissions.
- [`src-tauri/src/main.rs`](src-tauri/src/main.rs) — native binary entry.
- [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs) — typed commands and Tauri builder.
- [`src-tauri/src/onboarding/mod.rs`](src-tauri/src/onboarding/mod.rs) — onboarding exports and test wiring.
- [`src-tauri/src/onboarding/console.rs`](src-tauri/src/onboarding/console.rs) — console interaction helpers.
- [`src-tauri/src/onboarding/constants.rs`](src-tauri/src/onboarding/constants.rs) — onboarding constants.
- [`src-tauri/src/onboarding/device.rs`](src-tauri/src/onboarding/device.rs) — device discovery and profiles.
- [`src-tauri/src/onboarding/environment.rs`](src-tauri/src/onboarding/environment.rs) — coordinated environment URL discovery.
- [`src-tauri/src/onboarding/guided.rs`](src-tauri/src/onboarding/guided.rs) — guided-session orchestration.
- [`src-tauri/src/onboarding/helpers.rs`](src-tauri/src/onboarding/helpers.rs) — reusable onboarding helpers.
- [`src-tauri/src/onboarding/io.rs`](src-tauri/src/onboarding/io.rs) — terminal input/output boundary.
- [`src-tauri/src/onboarding/prerequisites.rs`](src-tauri/src/onboarding/prerequisites.rs) — prerequisite checks.
- [`src-tauri/src/onboarding/private_config.rs`](src-tauri/src/onboarding/private_config.rs) — secret-safe private config merge.
- [`src-tauri/src/onboarding/provider.rs`](src-tauri/src/onboarding/provider.rs) — provider selection and configuration.
- [`src-tauri/src/onboarding/readiness.rs`](src-tauri/src/onboarding/readiness.rs) — read-only readiness projection.
- [`src-tauri/src/onboarding/registry.rs`](src-tauri/src/onboarding/registry.rs) — contract-registry gate.
- [`src-tauri/src/onboarding/service_plan.rs`](src-tauri/src/onboarding/service_plan.rs) — read-only service action plan.
- [`src-tauri/src/onboarding/types.rs`](src-tauri/src/onboarding/types.rs) — serialized onboarding contracts.
- [`src-tauri/src/onboarding/tests.rs`](src-tauri/src/onboarding/tests.rs) — backend behavior tests.

Generated `dist/`, `*.tsbuildinfo`, and declaration outputs are build artifacts,
not maintained source entry points.
