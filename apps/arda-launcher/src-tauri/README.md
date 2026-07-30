# arda-launcher — native (Tauri 2)

The native side of the **arda-launcher** desktop app. This directory is a
standard Tauri 2 application: it owns the window, the icon set, the capability
(permission) policy, and the Rust entrypoint that hosts the React webview.

## Key Files

| File | Purpose |
| --- | --- |
| `Cargo.toml` | Tauri app, opener plugin, and onboarding dependencies. |
| `tauri.conf.json` | Window, icon, build, and bundle configuration. |
| `build.rs` | Tauri build script (codegen; no extra schema). |
| `capabilities/default.json` | Permission grants for the webview/runtime. |
| `icons/` | Application icon set referenced by `tauri.conf.json`. |
| `src/main.rs` | Entrypoint — calls `app_lib::run()`. |
| `src/lib.rs` | Registry/readiness/service-plan commands and app builder. |

## How It Fits Together

- `src/main.rs` delegates to `run()` in `src/lib.rs`.
- `lib.rs` builds the Tauri `Builder`, registers `tauri-plugin-opener`, and
  exposes three read-only onboarding commands.
- `src/onboarding/mod.rs` includes the eight-test onboarding suite through
  `#[cfg(test)] mod tests`.
- `tauri.conf.json` defines the window (title, size, decorations) and points at
  the `icons/` set. The frontend is served by Vite during `tauri dev` and built
  by `pnpm build` for `tauri build`.

## Configuration Notes

- The window title/size and bundle identifier are in `tauri.conf.json`.
- Capabilities (what the webview may do) are in `capabilities/default.json`.
- If you add a Tauri plugin, register it in `src/lib.rs` and add its permission
  to `capabilities/default.json`.

## Building

```bash
cd apps/arda-launcher
pnpm tauri dev      # run with hot-reload
pnpm tauri build    # produce a bundled installer
```

The Rust side is also covered by the workspace build:

```bash
cargo test -p arda-launcher --all-features
cargo clippy -p arda-launcher --all-targets --all-features -- -D warnings
```
