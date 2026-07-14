# Arda Refactor Plan — "One local machine, grown from a clean root"

Status: DRAFT / v1 · Owner: dward · Last touched: 2026-07-13

## Guiding intent
- Arda = the world (Earth). Silmarillion/Tolkien names for the *system's internal
  machinery*; Arda stays the name for the whole system + the daemon entry point.
- Kill bloat at the root: no MVC-less sprawl, no copy-paste scene code, no
  41-file routing mesh for one machine.
- Single local entry point first (`arda` daemon). Remote/fleet is a later growth ring,
  not baked in from day one.
- Hermes (or any harness) taps in through ONE well-defined surface, not by
  reaching into internals.

## Current state (verified)
- `Arda/` Cargo workspace: root bin `arda` (src/main.rs) is the intended single entry
  point. It supervises `apps/arda-launcher` (Tauri) and (now) `apps/arda-hud`
  (Tauri + React dashboard for the `manwe` gateway).
- Vendored `crates/annunimas-*` (24 crates) — grabbed wholesale from Annunimas.
  `annunimas-charon` alone is 41 source files of multi-provider routing, echo gate,
  bandits, quota, telemetry. Massive overkill for one box.
- `Arda-HUD/` is a SEPARATE repo: heavy R3F operational scene (dozens of .glb districts,
  hologram rig). Bloaty, no clean structure.
- Old connect model: hosted `annunimas-charon` @ `localhost:5110` as an OpenAI-compat
  router. You are dropping that for a single local machine.

## Naming map (proposed Silmarillion layer)
Adopt gradually; rename a crate when you actually touch it, not all at once.

| Old (annunimas-*)      | Proposed (Silmarillion) | Role in new system                    |
|------------------------|-------------------------|---------------------------------------|
| annunimas-charon       | `manwe`                 | single local inference gateway (port 7171) |
| annunimas-athena       | `varda`                 | ingest / knowledge triage             |
| annunimas-oracle       | `mandos`                | reasoning / validation                |
| annunimas-prometheus   | `melkor-council`→`aule` | orchestration / autopilot (TBD)       |
| annunimas-mnemosyne    | `vaire`                 | memory continuity                     |
| annunimas-hermes       | `orome`                 | comms bridge (harness taps here)      |
| annunimas-warden       | `tulkas`                | monitoring / guardhouse               |
| annunimas-ceo          | `arandur`               | top orchestrator                      |
|| arda-engine            | `arda-engine` (keep)    | supervision spine                     ||
Long term, the `arda` daemon should host/reuse shared types as needed; this repo's binaries/apps are the live surfaces.

## Refactor plan

Decide the final map in Section 0 before renaming anything.

## Section 0 — Decide the naming + scope contract (FROZEN 2026-07-13)
Decisions locked:
- Gateway crate = `manwe`. Local port = `7171` (deliberately NOT 5110, to avoid
  cross-contamination with the old hosted charon and any agent that still probes 5110).
- Naming map above is FINAL. Rename a crate only when you actually refactor it.
- Port `7171` is reserved workspace-wide: no other service may claim it.

- [x] Freeze the Silmarillion name table above.
- [x] Rule: a crate is renamed ONLY when you refactor it; the old `annunimas-*`
      name stays until then so `git` history + builds stay green.
- [x] Rule: every crate gets an `INDEX.md` + 1 paragraph purpose in Cargo.toml comments.
- [x] Rule: no scene/app without a clear Model / View / Controller split.

## Section 1 — The clean entry point (`arda` daemon)
Goal: `arda` boots the local system and exposes ONE tap-in surface.
- [x] Rewrite src/main.rs to: boot engine → start `manwe` (local gateway @7171) →
      optionally supervise UI apps → open the harness surface.
- [x] Define the harness surface explicitly (see Section 2). Hermes connects HERE,
      not to a hidden internal port.
- [x] Replace hardcoded `apps/arda-launcher/src-tauri/target/*` paths with a
      config-driven service registry (toml) so adding/removing apps is data, not code.
- [x] Keep `--once` smoke-test flag. Add `--no-ui` for headless/local-only mode.

## Section 2 — Single local inference gateway (the `manwe` crate)
Goal: replace annunimas-charon's 41-file mesh with ONE local OpenAI-compat endpoint.
- [x] New crate `crates/manwe`: listens on `127.0.0.1:7171`, serves
      `/v1/chat/completions` + `/v1/models`. (Port 7171 is the frozen contract — see Sec 0.)
- [x] Support exactly the local providers you run (e.g. Ollama @11434, a local llama.cpp,
      or one OpenRouter key). NO multi-vendor bandit/echo-gate machinery.
- [x] Thin provider catalog in toml; no runtime adaptive routing, no quota mesh.
- [x] Keep the `/v1/models` shape so Hermes `smart_model_routing` keeps working.
- [x] AI assist: I'll diff charon's `service.rs`/`bootstrap_defaults.rs` against what you
- [x]   actually call, and list the exact files safe to drop.

## Section 3 — UI apps under `apps/` (structure-first)
- [x] `arda-launcher`: keep, but enforce MVC. Move scene primitives (WorldTree,
      ParticleSmoke, Background) into a `scenes/` + `components/` + `state/` split.
- [x] Fresh HUD, not a migration: `apps/arda-hud` created as a clean Tauri 2 + React +
      TS + Tailwind v4 app. It is NOT the heavy R3F operational scene from the old
      `Arda-HUD` repo — it surfaces the live `manwe` gateway `/v1/models` (verified,
      builds to an AppImage in `lothlorien`, see Section 6 below). Structure is
      MVC-lite (`App.tsx` holds the typed fetch + state, `main.tsx` bootstraps,
      `styles/theme.css` holds the theme); the full `scenes/ components/ state/
      controllers/` R3F split is deferred until features are added back.
- [x] Shared R3F primitives live outside `crates/arda-core`; drop any planned
       `arda-core` reference and keep shared UI primitives either in `engine` or
       `apps/` workspace crates.
- [x] Daemon supervises both via the Section 1 registry; no hardcoded paths.

---

## Section 6 — Tauri native builds run in distrobox `lothlorien`

The host (Fedora/el10) is MISSING the GTK/WebKit system libs Tauri needs
(`webkit2gtk-4.1`, `gtk+-3.0`, `gdk-pixbuf-2.0`, `libsoup-3`, etc.). The
Ubuntu 24.04 distrobox container **`lothlorien`** has all of them and the
project is mounted at the same absolute path, so **all Tauri builds for
`arda-launcher` and `arda-hud` are done inside `lothlorien`.**

### Verify the container + toolchain
```bash
distrobox list                                  # confirm lothlorien is Running
distrobox enter lothlorien -- bash -lc '
  node --version      # v22.x
  pnpm --version      # v11.x
  cargo --version     # 1.94+
  pkg-config --exists webkit2gtk-4.1 && echo OK || echo MISSING
'
```
(The `.bashrc` line complaining about `/home/linuxbrew/.linuxbrew/bin/brew` is
harmless — it only prints an error and does not abort the shell.)

### Build the HUD (release binaries + .AppImage/.deb/.rpm)
```bash
# ⚠️ CORRECT — release build + bundle:
distrobox enter lothlorien -- bash -lc '
  cd /var/home/mythos/Eregion/Arda/apps/arda-hud
  pnpm exec cargo-tauri build
'

# ❌ WRONG — do NOT use `pnpm tauri build`:
#   package.json maps "tauri" -> "cargo-tauri dev", so `pnpm tauri build`
#   actually launches `tauri dev` (dev profile, tries to open a GUI window),
#   compiles in `dev` mode, and HANGS with no display inside the container.
#   It never produces a bundle. Use `pnpm exec cargo-tauri build` instead.
```
Equivalent from inside `src-tauri`:
```bash
cd /var/home/mythos/Eregion/Arda/apps/arda-hud/src-tauri
cargo tauri build        # same result, release + AppImage
```

### Verified artifacts (built 2026-07-13 inside lothlorien)
```
apps/arda-hud/src-tauri/target/release/bundle/
  appimage/arda-hud_0.1.0_amd64.AppImage   (~77 MB)
  deb/arda-hud_0.1.0_amd64.deb
  rpm/arda-hud-0.1.0-1.x86_64.rpm
```
The Rust crate compiles cleanly with the bundled `webkit2gtk-4.1` toolchain;
`cargo check` / `cargo tauri build` in `lothlorien` is the durable fix for the
host `pkg-config` / missing-`.pc` failures — they are NOT a source-level blocker.

### Frontend-only checks (no Tauri needed)
```bash
cd /var/home/mythos/Eregion/Arda/apps/arda-hud
pnpm install
pnpm build            # tsc -b && vite build  -> dist/
```
`pnpm build` was verified passing; the `src/vite-env.d.ts` side-effect CSS
import fix is required (both apps have it).

## Section 4 — Parsing the vendored annunimas crates
Goal: keep useful logic, drop the fleet/bloat, rename to Silmarillion.
- [ ] For each `annunimas-*` crate: read its `lib.rs` + `INDEX.md`, list what it
      actually does vs what the new local system needs. crates/old-annunimas/
- [ ] Extract the useful core into a Silmarillion-named crate; delete the rest.
- [ ] Order: manwe (Sec 2) → varda (ingest) → mandos (reasoning) → vaire (memory)
      → orome (comms). Leave orchestration (aulë) for last.
- AI assist: per crate I'll produce a "keep / shrink / delete" sheet with file-level
  evidence before you touch anything.

## Section 5 — Growth rings (deferred)
- Remote nodes, fleet topology, multi-machine routing = a LATER ring, built ON TOP of
  the clean local root. Do not pre-build it now.
- When the time comes, `manwe` grows a `remote` adapter; the daemon gains a topology
  registry. Not before.

## How we work
- You hit sections manually; I help parse each crate/scene and tell you what's safe to
  remove, with evidence (file:line), before you delete.
- Every rename/delete is a small batch; build + targeted tests must pass before moving on.
- This file is the source of truth — update it as decisions land.
