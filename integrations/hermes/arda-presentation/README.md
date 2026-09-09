---
soterion:
  sigil: "SCROLL"
  role: "implementation_guide"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-09-07"
---

> 🜏 Soterion: 📜 implementation_guide | owner: HERMES | status: active | reviewed: 2026-09-07

# Arda presentation execution adapter

Hermes tool `arda_present` connects an authenticated operator's display request to the existing native HUD monitor contract over a same-user Unix socket. It is not an approval authority, knowledge-ingestion pipeline, new registry, or rendering receipt.

## Operation

- `status` reads native readiness and the registry.
- `present` requires `ambient_allowed: true`, an exact source, and a stable request ID. Replays preserve the session; conflicting reuse is rejected. It refuses occupied active slots and uses `monitor_1` through `monitor_5` only.
- Cached attachments and generated artifacts are copied into content-addressed `data/media/imports/`. Native MIME detection selects supported image, PDF/text/Markdown document, or MP4/WebM video descriptors. Audio, office documents and other unsupported types return an explicit error; descriptor routes are not all visually accepted.
- Public HTTP(S) URLs currently publish a web capture-stream descriptor. `play` on web URLs is explicitly rejected; YouTube/native Chromium integration is not established by this adapter. URL syntax checks are not complete DNS/redirect or subresource isolation. Authenticated/private web and live playback acceptance remain open.
- Successful publication is read back through a separate native status request. The result explicitly says rendering/playback is unverified.
- HUD unavailable/not yet restored returns an error. No durable pending inbox, automatic retention, cross-device transport, or standalone playback engine is added.

## Installation

Install this directory as the default profile's `arda-presentation` plugin, then enable BOTH the plugin and the intended platform toolset:

```bash
hermes plugins enable arda-presentation
hermes tools enable arda_presentation --platform discord
```

Restart the gateway from an external shell after installation/configuration, not from its own agent turn. Built-in tool override permission is unnecessary. Tool search may defer `arda_present`; missing from the initial top-level schema list is not proof it is unavailable. Verify the platform-resolved definitions before tool-search assembly and dispatch a real `status` request.

The native socket is `$XDG_RUNTIME_DIR/arda-hud/presentation.sock`; no TCP listener is opened. Files and socket remain same-user only. Native publications stay blocked until successful registry restoration. Corrupt browser storage is not interpreted as an empty registry.

## Build and checks

From the repository root:

```bash
python integrations/hermes/arda-presentation/test_plugin.py
cargo test --manifest-path apps/arda-hud/src-tauri/Cargo.toml --lib commands::monitor_surface
cd apps/arda-hud
pnpm exec vitest run src/lib/monitorSurfacePersistence.test.ts
pnpm build
cd ../..
cargo build --release --manifest-path apps/arda-hud/src-tauri/Cargo.toml --features tauri/custom-protocol --bin arda_hud
```

A plain Cargo release build does not enable packaged frontend serving in this checkout. Verify feature selection, preserve a rollback binary, stop and verify the old PID is absent before replacing the installed executable, then verify native `ready: true`. Process-active status alone is insufficient.

## Acceptance boundary

The original operator photograph was published and read back through the installed native adapter. The operator subsequently confirmed the original image and a successful Discord-phone-originated request. Full media playback, restart/replay durability, pinned-screen handling, and FPS acceptance remain open in [Personal System Experience](../../../docs/plans/PERSONAL_SYSTEM_EXPERIENCE.md). Do not report them as complete from this adapter's tests or publication result.
