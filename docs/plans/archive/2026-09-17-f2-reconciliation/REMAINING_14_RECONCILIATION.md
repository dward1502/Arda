---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "reconciliation"
  owner: "RUMIL"
  status: "approved"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "reconciliation", "fabric", "f2", "remaining-14"]
---

> 🜏 Soterion: 📜 reconciliation | owner: RUMIL | status: approved | reviewed: 2026-09-17

# F2 — Remaining 14 Projects Reconciliation

Reconcile the 14 in-scope projects without F2/F3 contracts against actual source
and commands. Draft only missing contracts; preserve dirty work as read-only until
its ownership and approved mutation scope are understood.

Source evidence gathered read-only on 2026-09-17 from each project's filesystem,
git state, manifests, and docs. No project source mutated.

## Reconciliation result

All 14 projects are **not** currently in `data/workbench/projects.json` (beyond
any fixture/worktree copies already present). The existing registry contains
Arda-system, acceptance-fixture, and the 3 contracted business apps. All 14
contracts below are **missing** and are drafted here as review items, not attached.

Physical survey completed per-project; deeper command verification deferred to F3
attachment phase (operator-approved, idempotency receipted).

---

## Group A: Core Arda Parts (7) — Promotion Candidates

These are the parted-out Arda crates promoted for eventual public release.
All are Rust crates with remote `https://github.com/dward1502/Arda-*.git`.

### 1. Arda-Agent-Loop-Contract

**Physical root:** `/var/home/mythos/Eregion/Arda-Agent-Loop-Contract/`
**Remote:** `https://github.com/dward1502/Arda-Agent-Loop-Contract.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `feef33c`. Working tree clean.
**Stack (from Cargo.toml):** `agent-loop-contract` v0.1.0. Dependencies: anyhow, clap (derive), serde (derive), serde_json, toml. Dev-deps: tempfile.
**Commands (inferred from crate shape):**
- `cargo build`
- `cargo test`
- `cargo doc`
**No .env files, no secrets.**
**Documentation:** README.md (3401 bytes) — describes portable inspect-act-verify loop contract and local validator. docs/, examples/, schemas/ directories present.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-agent-loop-contract`
- `identity.kind`: `rust-library`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Agent-Loop-Contract`
- `runtime.adapter`: `cargo`
- `commands`: `cargo test`, `cargo build` (needs live verification)
- `checks`: test (needs verification)
- `artifacts`: library target
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false** (library crate, no outbound needs)
- `permissions.secrets.env_names`: [] (none)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Portable inspect-act-verify operating loop contract; public release candidate
**Review items (do not invent values):**
- Commands not yet live-verified on 2026-09-17
- Crate references `repository = "https://github.com/dward1502/agent-loop-contract"` (note: different repo name from physical remote — operator to clarify)
- Public release intent — needs operator confirmation of promotion scope

---

### 2. Arda-Council

**Physical root:** `/var/home/mythos/Eregion/Arda-Council/`
**Remote:** `https://github.com/dward1502/Arda-Council.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `ecb6861`. Working tree has dirty state: `target/` directory deleted from tracking (D entries in git status — build artifacts removed, not source dirt). No source file modifications.
**Stack (from Cargo.toml):** `annunimas-council` v0.1.0. Dependencies: serde (derive), serde_json, chrono (serde). Dev-deps: serde_json.
**Commands (from README.md):** `cargo test`, `cargo doc --no-deps`
**No .env files, no secrets.**
**Documentation:** README.md (2391 bytes) — multi-agent boardroom deliberation and consensus building. INDEX.md present. Docs reference Annunimas root protocol and CODEMAP.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-council`
- `identity.kind`: `rust-library`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Council`
- `runtime.adapter`: `cargo`
- `commands`: `cargo test`, `cargo doc --no-deps`
- `checks`: test (needs verification)
- `artifacts`: library target
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false**
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Multi-agent boardroom deliberation and consensus building; public release candidate
**Review items:**
- `target/` deletion is tracked in git — not source dirt, but build artifacts were committed previously. Contract should note this.
- Crate uses path dependencies on `../../Annunimas/crates/annunimas-core` and `../../Annunimas/crates/annunimas-mnemosyne` — these are external workspace references, not vendored. Build in isolation may require those paths to exist or be replaced.
- Status: "Blueprint-stage" per README — early development, not production-ready

---

### 3. Arda-Forge-Mind

**Physical root:** `/var/home/mythos/Eregion/Arda-Forge-Mind/`
**Remote:** `https://github.com/dward1502/Arda-Forge-Mind.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `25c6eeb`. Working tree has `target/` deleted from tracking (build artifacts removed). No source file modifications.
**Stack (from Cargo.toml):** `annunimas-forge-mind` v0.1.0. Dependencies: tokio (full), serde (derive), serde_json, anyhow, tracing, tracing-subscriber (env-filter), sha2. Dev-deps: none listed.
**Commands (inferred):** `cargo build`, `cargo test`
**No .env files, no secrets.**
**Documentation:** README.md (10274 bytes) — sovereign 3D asset forge for Annunimas (Blender, texturing, slicing, ARDA scene production). INDEX.md present.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-forge-mind`
- `identity.kind`: `rust-application`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Forge-Mind`
- `runtime.adapter`: `cargo`
- `commands`: `cargo build`, `cargo test` (needs verification)
- `checks`: test (needs verification)
- `artifacts`: binary + library targets
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false** (local 3D forge; no outbound needs declared)
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: 3D asset forge for Annunimas; public release candidate
**Review items:**
- `target/` deletion tracked in git — build artifacts were committed previously
- Heavy dependencies (tokio full, sha2) — build may be resource-intensive
- Path dependency on `annunimas-tool-harness` (local path) — external workspace reference

---

### 4. Arda-HUD

**Physical root:** `/var/home/mythos/Eregion/Arda-HUD/`
**Remote:** `https://github.com/dward1502/arda-hud.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `d9c906b`. **Dirty work:** 4 modified files:
- `M scripts/node_monitor.sh`
- `M src-tauri/Cargo.toml`
- `M src/lib/ardaBundleTypes.ts`
- `M src/lib/ardaSource.ts`
**Stack (from package.json):** React 19 + TypeScript + Vite frontend; Tauri 2 desktop shell. pnpm@10.8.1. Node >=20.19.0.
**Commands (from package.json scripts):**
- `dev`: `vite`
- `build`: `tsc && vite build`
- `test`: `vitest run`
- `tauri:dev:stable`: Tauri dev with NVIDIA/Wayland workarounds
- `tauri:build:stable`: Tauri build --no-bundle with environment setup
- `package:stable`: wrapper script
- `launch:stable`: wrapper script
**No .env files, no secrets.**
**Documentation:** README.md (8978 bytes) — comprehensive. INDEX.md, ARDA_CONTRACTS_MANIFEST.md, multiple contract docs under docs/contracts/. REPAIR_VERIFICATION.md present.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-hud`
- `identity.kind`: `desktop-app`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-HUD`
- `runtime.adapter`: `node` (pnpm-based; Tauri for native shell)
- `commands`:
  - `build`: `pnpm build` (tsc + vite build)
  - `test`: `pnpm test` (vitest run)
  - `dev`: `pnpm dev` (vite)
  - `tauri:build:stable`: Tauri build with env setup
  - `tauri:dev:stable`: Tauri dev with env setup
- `checks`:
  - `test` (vitest) — declared, needs verification
  - `build` (tsc + vite) — declared, needs verification
- `artifacts`: `dist/` build output; Tauri binary (when built)
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false** (desktop app; no outbound needs declared in contract)
- `permissions.secrets.env_names`: [] (none found)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Operator-facing frontend/desktop shell for Annunimas; public release candidate
**Dirty work preserved (read-only):**
- 4 modified files — active development on HUD source. Preserve until operator cleans up or approves mutation scope.
**Review items:**
- Dirty worktree with 4 modified source files — do not force writes to manufacture test conditions
- Tauri build requires specific env vars (PKG_CONFIG_PATH, __NV_DISABLE_EXPLICIT_SYNC, WEBKIT_* , GDK_BACKEND=x11) — build environment must be configured
- `scripts/node_monitor.sh` modified — shell script, review for correctness
- `src-tauri/Cargo.toml` modified — Tauri Rust config changed
- `src/lib/ardaBundleTypes.ts` + `src/lib/ardaSource.ts` modified — core TypeScript source changes
- Status: active Tauri/React operator surface; public release candidate

---

### 5. Arda-Human

**Physical root:** `/var/home/mythos/Eregion/Arda-Human/`
**Remote:** `https://github.com/dward1502/Arda-human.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `ecb6861`. Working tree clean.
**Stack (from Cargo.toml):** `annunimas-human-tenant` v0.1.0. Lib: `annunimas_human` (src/lib.rs). Dependencies: annunimas-core (path), annunimas-mnemosyne (path), serde (derive), serde_yaml, serde_json, anyhow, walkdir, regex. Dev-deps: tempfile.
**Commands (inferred):** `cargo build`, `cargo test`
**No .env files, no secrets.**
**Documentation:** README.md (424 bytes) — minimal. INDEX.md present. Docs/ and fixtures/ directories.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-human`
- `identity.kind`: `rust-library`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Human`
- `runtime.adapter`: `cargo`
- `commands`: `cargo build`, `cargo test` (needs verification)
- `checks`: test (needs verification)
- `artifacts`: library target
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false**
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Human tenant layer for Annunimas; public release candidate
**Review items:**
- Path dependencies on `../../Annunimas/crates/annunimas-core` and `../../Annunimas/crates/annunimas-mnemosyne` — external workspace references
- Minimal README — acceptance criteria need operator definition
- `publish = false` in Cargo.toml — not intended for crates.io publishing

---

### 6. Arda-Service-Registry

**Physical root:** `/var/home/mythos/Eregion/Arda-Service-Registry/`
**Remote:** `https://github.com/dward1502/Arda-Service-Registry.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `041be3a`. Working tree clean.
**Stack (from Cargo.toml):** `annunimas-service-registry` v0.1.0. Dependencies: serde (derive), serde_json, chrono (serde), thiserror, tracing. Dev-deps: serde_json.
**Commands (inferred):** `cargo build`, `cargo test`
**No .env files, no secrets.**
**Documentation:** README.md (15560 bytes) — substantial. INDEX.md present. src/, tests/, target/ directories.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-service-registry`
- `identity.kind`: `rust-library`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Service-Registry`
- `runtime.adapter`: `cargo`
- `commands`: `cargo build`, `cargo test` (needs verification)
- `checks`: test (needs verification)
- `artifacts`: library target
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false**
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Service registry for Annunimas; public release candidate
**Review items:**
- Commands not yet live-verified
- Substantial README — may contain acceptance criteria

---

### 7. Arda-Signal-Grid

**Physical root:** `/var/home/mythos/Eregion/Arda-Signal-Grid/`
**Remote:** `https://github.com/dward1502/Arda-Signal-Grid.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `e321029`. Working tree clean.
**Stack (from Cargo.toml):** `annunimas-signal-grid` v0.1.0. Dependencies: serde (derive), serde_json, chrono (serde). Dev-deps: serde_json.
**Commands (inferred):** `cargo build`, `cargo test`
**No .env files, no secrets.**
**Documentation:** README.md (3133 bytes). INDEX.md present. src/, tests/ directories.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `arda-signal-grid`
- `identity.kind`: `rust-library`
- `workspace.root`: `/var/home/mythos/Eregion/Arda-Signal-Grid`
- `runtime.adapter`: `cargo`
- `commands`: `cargo build`, `cargo test` (needs verification)
- `checks`: test (needs verification)
- `artifacts`: library target
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false**
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Signal grid for Annunimas; public release candidate
**Review items:**
- Commands not yet live-verified
- Minimal dependencies — lightweight crate

---

## Group B: Additional Business Apps (3) — Operator-Directed In-Scope

### 8. wakita

**Physical root:** `/var/home/mythos/Eregion/wakita/`
**Remote:** `https://github.com/RavensNestInc/wakita.git` (fetch + push)
**Original upstream:** `https://github.com/shun1000/wakitaRE.git`
**Git state (2026-09-17):** branch `main`, HEAD `bd79cc6`. Working tree clean.
**Stack (from package.json):** Express.js 2.0.0 + TypeScript. Scripts: start (node server.js), dev (nodemon), test (jest --coverage), build:static, ts:check. Dependencies: express, mongoose, passport (local/JWT/Facebook/Google OAuth), aws-sdk (S3, Secrets Manager), bcryptjs, cors, etc. Dev-deps: jest, supertest, typescript, nodemon.
**Commands:**
- `npm start` — production server
- `npm run dev` — dev server with nodemon
- `npm test` — jest with coverage
- `npm run build:static` — static export to dist-static/
- `npm run ts:check` — tsc --noEmit
**No .env files found in survey.** `.env.example` present in README (lists MONGODB_URI, JWT_SECRET, OAuth keys, AWS keys, SESSION_SECRET, etc.)
**Documentation:** README.md (3968 bytes) — comprehensive. AZURE_DEPLOY.md present. GitHub Actions CI/CD workflows. Structure: server/, public/, scripts/, tests/, dist-static/.
**Ownership boundary:** RavensNestInc remote (different org than dward1502). Authors: Daniel Ward, Shun Wakita. License: Apache-2.0.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `wakita`
- `identity.kind`: `web-app`
- `identity.class`: `business`
- `workspace.root`: `/var/home/mythos/Eregion/wakita`
- `workspace.repository.remote`: `https://github.com/RavensNestInc/wakita.git`
- `runtime.adapter`: `node`
- `runtime.language`: `typescript`
- `runtime.package_manager`: `npm`
- `commands`:
  - `start`: `npm start`
  - `dev`: `npm run dev`
  - `test`: `npm test` (jest --coverage)
  - `build:static`: `npm run build:static`
  - `ts:check`: `npm run ts:check`
- `checks`:
  - `test` (jest) — declared, needs verification
  - `ts:check` (tsc) — declared, needs verification
- `artifacts`: `dist-static/` (static export); server runtime
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true** (Express app with OAuth, AWS, MongoDB — needs outbound)
- `permissions.secrets.env_names`: operator-specified from `.env.example` (MONGODB_URI, JWT_SECRET, FACEBOOK_APP_ID/SECRET, GOOGLE_CLIENT_ID/SECRET, AWS_REGION/ACCESS_KEY/SECRET, SESSION_SECRET, etc.)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Real estate website — Wakita & Associates. Two modes: local Express preview and Azure Static Web Apps deployment.
**Review items:**
- Ownership boundary: RavensNestInc org (not dward1502) — operator to confirm mutation authority for this remote
- No .env file present in checkout — operator must supply values if running
- Two deployment modes (Express-hosted vs Azure Static) — contract needs to specify which mode is in scope
- Original upstream is shun1000/wakitaRE — forks from external author; license Apache-2.0
- Jest tests present — verify current state before relying on them
- `dist-static/` already present — prior static build exists

---

### 9. filamentDB

**Physical root:** `/var/home/mythos/Eregion/filamentDB/`
**Remote:** `https://github.com/dward1502/filamentDB.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `26f2ecc`. Working tree clean.
**Stack (from package.json):** Next.js 14.0.4 + React 18 + TypeScript. AWS Amplify adapter + UI. MUI 5 (material, icons, emotion). React Hook Form, react-dropzone, pdf-lib, sharp, uuid. Dev-deps: eslint, typescript, @types/*.
**Commands (from package.json scripts):**
- `dev`: `next dev`
- `build`: `next build`
- `start`: `next start`
- `lint`: `next lint`
**Env files present:** `.env` (148 bytes), `.env.production` — contain real values; do not read.
**Documentation:** README.md (1383 bytes) — Next.js boilerplate. Only package.json + README.md surveyed; deeper structure TBD.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `filamentdb`
- `identity.kind`: `web-app`
- `identity.class`: `business`
- `workspace.root`: `/var/home/mythos/Eregion/filamentDB`
- `runtime.adapter`: `node`
- `runtime.language`: `typescript`
- `runtime.package_manager`: `yarn` (yarn.lock present) or `npm` (package.json present; no packageManager field)
- `commands`:
  - `dev`: `npm run dev` (next dev)
  - `build`: `npm run build` (next build)
  - `start`: `npm run start` (next start)
  - `lint`: `npm run lint` (next lint)
- `checks`:
  - `lint` (next lint) — declared, needs verification
  - `build` (next build) — declared, needs verification
  - `test`: none defined — gap recorded
- `artifacts`: `.next/` build output
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true** (AWS Amplify — needs outbound)
- `permissions.secrets.env_names`: operator-specified from `.env` and `.env.production` (values not read)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: Next.js web app with AWS Amplify; deeper purpose TBD from full survey
**Review items:**
- Only surface survey done (package.json + README.md) — deeper structure (src/, amplify/, .graphqlconfig.yml) not yet examined
- `.env` and `.env.production` exist — contain real values; do not read into contract
- Package manager ambiguous: yarn.lock present but no packageManager field in package.json — operator to confirm
- Next.js 14.0.4 (older than other business apps which are on 15.x/16.x)
- Boilerplate README — acceptance criteria need operator definition
- AWS Amplify integration — deployment target TBD

---

### 10. ravensnestweb

**Physical root:** `/var/home/mythos/Eregion/ravensnestweb/`
**Remote:** `https://github.com/RavensNestInc/ravensnestweb.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, HEAD `9d4305ba`. Working tree clean.
**Stack (from package.json):** pnpm@10.17.1 workspace. Node >=20 <23. Mono-repo with apps/web (Vite+React) and apps/api (Node.js+TypeScript). 3D print business platform.
**Commands (from package.json scripts):**
- `dev:web`: `pnpm --filter web dev`
- `dev:api`: `pnpm --filter api dev`
- `build`: `pnpm -r build`
- `build:web`: `pnpm --filter web build`
- `build:api`: `pnpm --filter api build`
- `start`: `pnpm --filter api start`
- `lint`: `pnpm -r lint`
- `lint:web`: `pnpm --filter web lint`
- `lint:api`: `pnpm --filter api lint`
- `db:migrate`: `pnpm --filter api db:migrate`
- `db:seed`: `pnpm --filter api db:seed`
**No .env files found in surface survey.**
**Documentation:** README.md (4109 bytes). PROJECT_PLAN.md, TASKS.md, EXECUTION_TASKS.md present. apps/ and packages/ workspace structure. docs/ directory.
**Ownership boundary:** RavensNestInc remote (different org than dward1502).
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `ravensnestweb`
- `identity.kind`: `web-platform`
- `identity.class`: `business`
- `workspace.root`: `/var/home/mythos/Eregion/ravensnestweb`
- `workspace.repository.remote`: `https://github.com/RavensNestInc/ravensnestweb.git`
- `runtime.adapter`: `node`
- `runtime.language`: `typescript`
- `runtime.package_manager`: `pnpm`
- `commands`:
  - `build`: `pnpm build` (pnpm -r build)
  - `build:web`: `pnpm build:web`
  - `build:api`: `pnpm build:api`
  - `lint`: `pnpm lint` (pnpm -r lint)
  - `dev:web`: `pnpm dev:web`
  - `dev:api`: `pnpm dev:api`
  - `start`: `pnpm start` (pnpm --filter api start)
  - `db:migrate`: `pnpm db:migrate`
  - `db:seed`: `pnpm db:seed`
- `checks`:
  - `lint` (pnpm -r lint) — declared, needs verification
  - `build` (pnpm -r build) — declared, needs verification
  - `test`: not defined at root level — gap recorded (may exist in sub-packages)
- `artifacts`: `dist/` (present), `apps/web/dist/`, `apps/api/` build outputs
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true** (3D print business platform with API — likely needs outbound)
- `permissions.secrets.env_names`: operator-specified (none found in surface survey)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: 3D print business platform — customer/operator frontend + Node.js API
**Review items:**
- Ownership boundary: RavensNestInc org (not dward1502) — operator to confirm mutation authority
- Mono-repo with pnpm workspaces — commands are scoped to filters (--filter web, --filter api)
- Deeper survey needed: apps/web/package.json, apps/api/package.json for full command/stack detail
- No .env files in surface survey — may exist in apps/api/ or apps/web/
- `dist/` present — prior build exists
- PROJECT_PLAN.md and TASKS.md present — acceptance criteria may be defined there

---

## Group C: Internal Projects (2) — Non-Promotion, In-Scope as Internal Work

### 13. relic-kiosk

**Physical root:** `/var/home/mythos/Eregion/relic-kiosk/`
**Remote:** None — no .git directory. Stub directory, not a standalone git project.
**Git state:** N/A — no git repository.
**Stack (from package.json):** `annunimas-relic` v0.1.0, type: module. Scripts: deploy:citadel (bash deploy_to_citadel.sh), prepare:vendor (rsync three.js from Arda-HUD), serve (npm run prepare:vendor && python3 -m http.server 8091 --bind 127.0.0.1), test (node --test test/*.test.mjs), validate (node --check + npm test).
**Commands:**
- `npm run serve` — serve on http://127.0.0.1:8091
- `npm run test` — node --test test/*.test.mjs
- `npm run validate` — node --check + npm test
- `npm run prepare:vendor` — vendor three.js from Arda-HUD
- `npm run deploy:citadel` — deploy to Citadel
**No .env files.**
**Documentation:** README.md (4523 bytes). INDEX.md present. src/, public/, scripts/, test/ directories. index.html present.
**RELIC = first CITADEL geometry-avatar prototype.** References Arda scene adapter in test code. Intentionally separate from apps/citadel-companion.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `relic-kiosk`
- `identity.kind`: `prototype`
- `identity.class`: `internal`
- `workspace.root`: `/var/home/mythos/Eregion/relic-kiosk`
- `workspace.repository.remote`: null (no git — internal prototype)
- `runtime.adapter`: `node`
- `runtime.language`: `javascript`
- `runtime.package_manager`: `npm`
- `commands`:
  - `serve`: `npm run serve` (python3 http.server 8091)
  - `test`: `npm run test` (node --test)
  - `validate`: `npm run validate`
  - `prepare:vendor`: `npm run prepare:vendor`
  - `deploy:citadel`: `npm run deploy:citadel`
- `checks`:
  - `test` (node --test) — declared, needs verification
  - `validate` (node --check) — declared, needs verification
- `artifacts`: served static content (index.html, src/, public/)
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false** (local-only serve on 127.0.0.1:8091)
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `none-read-only` (no git — no reverts; manual cleanup only)
- `provenance.declared_by`: operator
- `provenance.purpose`: RELIC = first CITADEL geometry-avatar prototype; internal prototype, not for public release
**Review items:**
- No git repository — not a standalone project; treated as internal prototype
- `prepare:vendor` depends on Arda-HUD node_modules/three/ — cross-project dependency on Arda-HUD
- Serve binds to 127.0.0.1:8091 — local-only, no network exposure
- `deploy:citadel` script exists — deployment to Citadel hardware; operator to confirm scope
- References Arda scene adapter in test code — Arda-adjacent prototype
- Acceptance criteria: needs operator definition for prototype status

---

### 14. citadel-avatar

**Physical root:** `/var/home/mythos/Eregion/citadel-avatar/`
**Remote:** None — no .git directory. Stub directory, not a standalone git project.
**Git state:** N/A — no git repository.
**Stack (from package.json):** Only dependency: three ^0.183.2. No scripts defined. This is a runtime projection surface, not a buildable npm project.
**Commands:** None defined in package.json. Runtime is served locally on Pi (http://127.0.0.1:8080). Various .js files present (agent-kinetic-visualization.js, metatron-cube.js, scene-runtime-optimized.js, etc.) plus shell scripts (play_audio.sh, openai_tts.sh, remote_media_state.sh, etc.).
**No .env files.**
**Documentation:** README.md (2569 bytes). INDEX.md, DEPLOYMENT_SUMMARY.md, DEPLOYMENT_TO_PI5.md, SCENE_STATE.md, AGENT_KINETIC_VISUALIZATION.md present. Various .sh scripts and .json configs.
**Canonical Pi-facing Three.js app for the round Waveshare display.** Arda-adjacent projection surface.
**Missing contract fields (draft, pending operator review):**
- `identity.name`: `citadel-avatar`
- `identity.kind`: `runtime-projection`
- `identity.class`: `internal`
- `workspace.root`: `/var/home/mythos/Eregion/citadel-avatar`
- `workspace.repository.remote`: null (no git — internal runtime)
- `runtime.adapter`: `none` (runtime projection, not a buildable project)
- `runtime.language`: `javascript`
- `commands`: [] (no npm scripts; runtime is served, not built)
- `checks`: [] (no declared checks; runtime verification is manual/operational)
- `artifacts`: served HTML/JS content (index.html, agent-kinetic-visualization.js, etc.)
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **false** (local-only serve on Pi, 127.0.0.1:8080)
- `permissions.secrets.env_names`: []
- `rollback.strategy`: `none-read-only` (no git — manual recovery only)
- `provenance.declared_by`: operator
- `provenance.purpose`: Canonical Pi-facing Three.js app for round Waveshare display; Arda-adjacent projection surface
**Review items:**
- No git repository — not a standalone project; treated as internal runtime
- No npm scripts — not a buildable npm project; runtime is served directly
- package.json only declares three.js dependency — no build/test/lint scripts
- Deployment docs exist (DEPLOYMENT_TO_PI5.md, DEPLOYMENT_SUMMARY.md) — operator to confirm scope
- Run script (run.sh) and quick-verify.sh present — operational verification exists
- Serves on Pi at http://127.0.0.1:8080 — local-only, no network exposure
- Acceptance criteria: needs operator definition for runtime projection surface

---

## Cross-project observations

1. **7 core Arda parts** are Rust crates with similar shapes (cargo build/test, no secrets, no network). All are promotion candidates with remote dward1502/Arda-*.git. Commands largely inferred from crate shape — live verification needed.

2. **Arda-Council and Arda-Forge-Mind** have `target/` deletion tracked in git — build artifacts were previously committed. This is not source dirt but indicates prior build-in-commit history.

3. **Arda-HUD** is the only core Arda part with dirty work (4 modified source files). It's also the only one with a native Tauri build path requiring specific env vars.

4. **Arda-Human** has path dependencies on Annunimas workspace crates — builds in isolation may fail without those paths. Same for Arda-Council and Arda-Forge-Mind (tool-harness path dep).

5. **3 business apps** include 2 with RavensNestInc remotes (wakita, ravensnestweb) — different org ownership than dward1502. Operator must confirm mutation authority for these.

6. **wakita** is Express.js (not Next.js) — different runtime shape from other business apps. Has Jest tests and Azure Static Web Apps deployment path.

7. **filamentDB** is Next.js 14 (older) with AWS Amplify. Only surface-surveyed; deeper structure TBD. `.env` and `.env.production` exist with real values.

8. **ravensnestweb** is a pnpm mono-repo with filtered commands (--filter web, --filter api). Deeper per-package survey needed.

9. **2 internal projects** (relic-kiosk, citadel-avatar) have no git — they are stub directories, not standalone projects. relic-kiosk is a prototype with npm scripts; citadel-avatar is a runtime projection with no build scripts.

10. **relic-kiosk** depends on Arda-HUD for vendor three.js — cross-project dependency.

---

## What F2 does not do

- Does not attach any of these contracts to the registry (that's F3, requiring operator approval with idempotency receipts)
- Does not read secret values from `.env` files
- Does not run build/test/lint commands against these projects (that would mutate state and touch secret-dependent code paths)
- Does not resolve the dirty work in Arda-HUD
- Does not invent env names, deployment targets, or acceptance criteria
- Does not perform deeper surveys for filamentDB and ravensnestweb (deferred to F3 execution phase)

---

## What F3 needs from the operator

Before any of these 14 contracts can be attached:

1. **Approval** to attach each contract (or batched approval for groups A/B/C)
2. **Ownership confirmation** for RavensNestInc-remote projects (wakita, ravensnestweb)
3. **Public release intent confirmation** for the 7 core Arda parts
4. **Command verification** — live run of cargo test / pnpm test / npm test for each project (or operator waiver for ones that can't be verified in current environment)
5. **Cross-project dependency resolution** — Arda-Human, Arda-Council, Arda-Forge-Mind path deps on Annunimas workspace; relic-kiosk vendor dep on Arda-HUD
6. **filamentDB deeper survey** — full src/ structure, amplify/ config, env var identification
7. **ravensnestweb deeper survey** — apps/web and apps/api package.json inspection, env var identification
8. **Internal project scope** — relic-kiosk and citadel-avatar deployment/operational scope confirmation
