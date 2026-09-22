---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "reconciliation"
  owner: "RUMIL"
  status: "approved"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "reconciliation", "fabric", "f2", "business"]
---

> 🜏 Soterion: 📜 reconciliation | owner: RUMIL | status: approved | reviewed: 2026-09-17

# F2 — Business Apps Reconciliation

Reconcile the three in-scope business apps (CoverCoINC, wgtt, skylightpros)
against actual source and commands. Draft only missing contracts; preserve dirty
work as read-only until its ownership and approved mutation scope are understood.

Source evidence gathered read-only on 2026-09-17 from each repo's filesystem,
git state, manifests, and docs. No project source mutated.

## Reconciliation result

None of the three business apps are currently in `data/workbench/projects.json`.
The existing registry contains only Arda-system and acceptance-fixture entries.
All three contracts below are **missing** and are drafted here as review items,
not attached.

### 1. CoverCoINC

**Physical root:** `/var/home/mythos/Eregion/CoverCoINC/`
**Remote:** `https://github.com/dward1502/CoverCoINC.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, 0 commits behind origin/master.
Latest commit `38a533f` "adjusted contact info and took out calculator".
Working tree has unstaged modified files (README.md and possibly others).

**Stack (from package.json):**
- Next.js `^15.3.1`, React `^19.1.0`
- Supabase (`@supabase/supabase-js ^2.103.3`, `@supabase/ssr ^0.10.2`)
- Resend `^6.5.2`, SES (`@aws-sdk/client-sesv2 ^3.913.0`)
- React Hook Form, Zod, Sharp, Tailwind CSS 4, TypeScript 5
- Node `>=20 <21` required

**Commands (from package.json scripts):**
- `dev`: `next dev`
- `build`: `next build`
- `start`: `next start`
- `lint`: `next lint`

**Checks:** No `test` script defined in package.json. Lint is the only
declared quality gate. The May 2026 monetization doc reported `npm run build`
passed but warned about missing env vars causing Supabase fallback data.

**Artifacts:** `.next/` output directory (already present in tree — a prior
build exists). No dedicated artifact declarations.

**Permissions needed:**
- network: **allow** (this is a deployed web app — Resend email, Supabase
  client, SES all need outbound network; the current registry pattern of
  `network.allow: false` is wrong for a business web app)
- filesystem: **write** for build output, `.next/`, possibly `.env.local`
  handling
- secrets: **env_names** must include at minimum `RESEND_API_KEY`, `SES_REGION`,
  `EMAIL`, plus whatever Supabase keys the deployed app needs

**Dirty work preserved (read-only):**
- `.env` exists (97 bytes, keys: `EMAIL`, `RESEND_API_KEY`, `SES_REGION`) —
  contains real secret values; do not read values into any contract
- `README.md` is modified (unstaged) — the boilerplate README is being worked on
- `audit-notes.md` is a placeholder checklist, last updated 2026-03-26

**Documentation state:**
- `README.md` is still the default Next.js boilerplate — not project-specific
- `audit-notes.md` is a placeholder with no findings documented
- No deployment/runbook docs exist yet (the May doc flagged this as the
  highest-value work: "deployment, form/email verification, Supabase/env
  setup, SEO/content polish, and conversion tracking")
- `amplify.yml` exists for Amplify deployment

**Missing contract fields (draft, pending operator review):**
- `identity.name`: `covercoinc`
- `identity.kind`: `web-app` (or `nextjs`)
- `workspace.root`: `/var/home/mythos/Eregion/CoverCoINC`
- `runtime.adapter`: `node` (Next.js app; build/start/lint via npm)
- `commands`: `build`, `start`, `lint` (no test command — that's a gap, not a
  value to invent)
- `checks`: lint only for now; test coverage gap noted
- `artifacts`: `.next/` build output
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true** (needs outbound for email + Supabase)
- `permissions.secrets.env_names`: must be specified by operator (don't invent)
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: lead-generation/contact website; deploy + form/email
  verification + Supabase env setup are the immediate paid-work angle per the
  May monetization doc

**Review items (do not invent values):**
- Exact secret env names and deployment target (Vercel? Amplify? other?) must
  come from operator, not from reading `.env`
- No test command exists — contract should record that gap rather than
  fabricate one
- README is boilerplate — acceptance criteria need operator definition

---

### 2. WGTT (World Stage Travels)

**Physical root:** `/var/home/mythos/Eregion/wgtt/`
**Remote:** `https://github.com/dward1502/wgtt.git` (fetch + push)
**Git state (2026-09-17):** branch `main`, latest commit `b531b1f`
"feat(admin): surface live operations stats". Working tree clean (`git status
--short` returned empty). No dirty work to preserve.

**Stack (from package.json):**
- Next.js `16.0.5`, React `19.2.0` (note: PROJECT_STATUS.md says Next.js 14 —
  that doc is stale)
- Supabase (`@supabase/supabase-js ^2.86.2`, `@supabase/ssr ^0.9.0`)
- WeTravel API, Travefy API, Crossbar webhooks
- Resend, OpenRouter, Radix UI, Recharts, Framer Motion, shadcn/ui
- Tailwind CSS 4, TypeScript 5
- Package manager: pnpm (pnpm-lock.yaml present)

**Commands (from package.json scripts):**
- `dev`: `next dev`
- `build`: `NODE_OPTIONS="--require ./scripts/suppress-baseline-warnings.cjs"
  BASELINE_BROWSER_MAPPING_IGNORE_OLD_DATA=true
  BROWSERSLIST_IGNORE_OLD_DATA=true next build`
- `start`: `next start`
- `lint`: `eslint`
- `test`: `pnpm lint` (the "test" script is just lint — not a real test suite)

**Checks:** Lint is the declared quality gate and the only CLI check. The May
2026 monetization doc reported `pnpm build` passed but `pnpm lint` failed with
51 problems (16 errors, 35 warnings), including `app/profile/page.tsx`
accessing `checkSession` before declaration and explicit-any errors.

**Artifacts:** `.next/` output directory (present). Supabase schema/migration
files in `database/` and `database/migrations/`. No dedicated artifact
declarations beyond build output.

**Permissions needed:**
- network: **allow** (WeTravel API, Travefy API, Crossbar webhooks, Resend,
  Supabase, OpenRouter all need outbound network)
- filesystem: **write** for build output
- secrets: **env_names** — `.env.local` has 19 keys including
  `WETRAVEL_PARTNER_API_KEY`, `WETRAVEL_WEBHOOK_SIGNING_SECRET`,
  `CROSSBAR_WEBHOOK_SECRET`, `SUPABASE_SERVICE_ROLE_KEY`, `TRAVEFY_API_KEY`,
  `OPENROUTER_API_KEY`, `RESEND_API_KEY`, `NEXT_PUBLIC_SUPABASE_URL`,
  `NEXT_PUBLIC_SUPABASE_ANON_KEY`, etc. Operator must specify which are secret
  vs public; don't read values.

**Dirty work:** None. Clean working tree.

**Documentation state (richer than CoverCoINC):**
- `SETUP.md` — Supabase setup, schema import, RLS, admin user, build/run,
  production checklist
- `PROJECT_STATUS.md` — project overview, architecture, completed pages,
  security/auth, next steps (but stale: says Next.js 14, March 2026)
- `ADMIN_OPERATIONS_PLAN.md` — detailed working plan for `/admin` as operations
  console; integration map for Supabase/WeTravel/Travefy/Crossbar; proposed
  build phases; immediate questions to answer offline
- `docs/AMAZON-AWS-AMPLIFY-CHECKLIST.md` — deployment checklist (pre-deployment
  done, console setup/env vars/custom domain/first deploy/post-deploy verification
  not checked off)
- `docs/DEPLOYMENT_ADMIN_READINESS_SPRINT_2026-05-20.md` — exists
- `database/schema.sql`, `database/rls_policies.sql`, migrations — real schema
- `components.json` — shadcn/ui config

**Missing contract fields (draft, pending operator review):**
- `identity.name`: `wgtt`
- `identity.kind`: `web-app` (or `nextjs`)
- `workspace.root`: `/var/home/mythos/Eregion/wgtt`
- `runtime.adapter`: `node` (pnpm-based Next.js app)
- `commands`: `build`, `lint` (test is just lint — gap noted); could add
  `dev`, `start` for run
- `checks`: lint (with known 51-problem failure state documented); no real test
  suite exists
- `artifacts`: `.next/` build output; optionally database schema files
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true**
- `permissions.secrets.env_names`: operator-specified from the 19 keys in
  `.env.local`
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: travel booking platform with WeTravel/Travefy/Crossbar
  integrations; launch readiness + admin operations integration is the paid-work
  angle per the May monetization doc

**Review items (do not invent values):**
- Lint currently fails (51 problems) — contract should record that the check is
  declared but not currently passing, not hide it
- `test` script is `pnpm lint` — not a real test suite; record the gap
- PROJECT_STATUS.md is stale relative to actual code (Next.js 14 vs 16.0.5)
- Exact secret env names must come from operator
- ADMIN_OPERATIONS_PLAN.md has open questions about trip/booking/itinerary/
  payment/roster ownership that affect what "acceptance" means for this project

---

### 3. Skylight Pros

**Physical root:** `/var/home/mythos/Eregion/skylightpros/`
**Remote:** `https://github.com/dward1502/skylightpros.git` (fetch + push)
**Git state (2026-09-17):** branch `master`, latest commit `ae3a2d6`
"ci: provide build-time placeholder env". HEAD is **0 commits behind**
origin/master (the May doc said 5 behind, but current state is synced).
Working tree has **multiple unstaged modified files** — dirty work to preserve.

**Dirty work preserved (read-only, do not mutate):**
Modified files (from `git status --short`):
- `M PRODUCTION_HARDENING_CHECKLIST.md`
- `M README.md`
- `M docs/CONFIG_INDEX.md`
- `M docs/TESTING_CHECKLIST.md`
- `M middleware.ts`
- `M src/app/(public)/login/page.tsx`
- `M src/app/admin/components/AdminTopBar.tsx`
- `M src/app/admin/layout.tsx`
- `A src/app/admin/users/UsersInviteClient.tsx` (new)
- `A src/app/admin/users/page.tsx` (new)
- `A src/app/api/admin/users/invite/route.ts` (new)
- `M src/lib/access-control.ts`
- `A src/lib/admin/users.ts` (new)
- `M src/lib/field-access.ts`

This is active development work in progress (admin users feature being built).
The contract must preserve this dirty state — no writes into this tree until
ownership and approved mutation scope are understood.

**Stack (from package.json):**
- Next.js `15.5.4`, React `19.1.0`
- Prisma (`@prisma/client ^5.22.0`, `prisma ^5.22.0`)
- Supabase (`@supabase/supabase-js ^2.103.0`, `@supabase/ssr ^0.7.0`)
- NextAuth v5 beta, AWS SDK (SES, S3, SNS, Cognito), Twilio, Google APIs,
  HubSpot, Anthropic SDK, OpenAI SDK
- Agenda (`agenda ^6.2.4`, `@agendajs/postgres-backend ^3.0.4`) for job queue
- Redis (Upstash), React Query, Recharts, Fr  mer Motion, shadcn/ui
- Tailwind CSS 4, TypeScript 5, Vitest
- Package manager: pnpm 10.33.0 (`packageManager` field set)

**Commands (from package.json scripts):**
- `dev`: `NODE_OPTIONS='--max-old-space-size=3072' next dev --turbo`
- `build`: `NODE_OPTIONS='--max-old-space-size=2048' next build --no-lint`
- `start`: `next start`
- `lint`: `eslint .`
- `test`: `vitest run`
- `typecheck`: `NODE_OPTIONS='--max-old-space-size=3072' tsc --noEmit`
- `build:full`: `pnpm lint && pnpm typecheck && next build --no-lint`
- `ci`: `pnpm lint && pnpm typecheck && pnpm test && next build --no-lint`
- `prisma:generate`: `prisma generate`
- `prebuild`: `pnpm prisma:generate`
- `smoke:e2e`: `node scripts/e2e-smoke.mjs`
- `uptime:check`: `node scripts/uptime-check.mjs`

**Checks:** This project has the richest check surface of the three:
- `test` (vitest run) — real test suite exists
- `lint` (eslint)
- `typecheck` (tsc --noEmit)
- `build` (next build)
- `ci` combines all four

The May 2026 monetization doc reported:
- `pnpm test` failed: 22 passed, 1 failed, 1 suite failed, 1 skipped.
  Failure 1: `src/app/api/contact/route.test.ts` expected 200 but received 500
  because SES config reported "Region is missing". Failure 2:
  `src/app/api/quote/route.test.ts` suite failed to load `@/lib/agenda`.
- `pnpm typecheck` timed out after 240 seconds.
- `pnpm build` timed out after 300 seconds during Next production build.

**Artifacts:**
- `.next/` build output (present — a prior build exists)
- `prisma/` schema directory
- `node_modules/` (present, pnpm-managed)
- No dedicated artifact declarations beyond build output

**Permissions needed:**
- network: **allow** (Supabase, SES, S3, SNS, Twilio, HubSpot, Google APIs,
  Anthropic, OpenAI, Agenda postgres backend all need outbound network)
- filesystem: **write** for build output, `.next/`, Prisma client generation
- secrets: **env_names** — `.env.local` (1145 bytes) and `.env.example`
  (1676 bytes) exist. Operator must specify which env vars are needed; don't
  read values. Likely includes Supabase keys, SES keys, Twilio keys, HubSpot
  keys, Anthropic/OpenAI keys, Redis/Upstash keys, NextAuth secrets, etc.

**Documentation state (very rich — best documented of the three):**
- `AGENT.md` — agent instructions for the project
- `STRUCTURE_AND_SCHEMA.md` — structure and DB schema docs
- `SUPABASE_FINAL.md` — Supabase setup
- `ROUTES_AND_ENTRYPOINTS.md` — route map
- `REPO_INDEX.json` (57KB), `REPO_TREE.md` (13KB) — comprehensive repo maps
- `PRODUCTION_HARDENING_CHECKLIST.md` (8.6KB) — detailed hardening checklist
  (but has merge conflict markers — the May doc flagged this; current `git
  status` shows it's still modified, so the conflict may be unresolved)
- `STAGING_SIGNOFF_CHECKLIST.md` — staging sign-off checklist (incomplete per
  May doc: missing sign-off date, approvers, known-issues section)
- `progress.txt` — extensive field worker UX and media queue work notes
- `FIELD_WORKER_BACKLOG.md` — mobile field app, job checklist, media capture,
  inventory scan, office media queue, scoped PWA, safety/permissions epics
- `docs/TASKS.md` — social media management implementation plan (modified)
- `docs/CONFIG_INDEX.md` — config index (modified)
- `docs/TESTING_CHECKLIST.md` — testing checklist (modified)
- `SCRIPTS.md`, `SKILLS.md` equivalents
- `middleware.ts` — auth middleware (modified)
- `env.d.ts`, `globals.d.ts`, `next-auth.d.ts` — type declarations
- `.lighthouse-budget.json` — Lighthouse performance budgets
- `tailwind.config.ts`, `postcss.config.mjs`, `eslint.config.js` — config files

**Missing contract fields (draft, pending operator review):**
- `identity.name`: `skylightpros`
- `identity.kind`: `web-app` (or `nextjs`)
- `workspace.root`: `/var/home/mythos/Eregion/skylightpros`
- `runtime.adapter`: `node` (pnpm-based Next.js app with Prisma)
- `commands`:
  - `build`: `pnpm build` (from project root)
  - `test`: `pnpm test`
  - `lint`: `pnpm lint`
  - `typecheck`: `pnpm typecheck`
  - `ci`: `pnpm ci` (full pipeline)
  - `prisma:generate`: `pnpm prisma:generate` (prebuild step)
  - working_dir for all: `/var/home/mythos/Eregion/skylightpros`
- `checks`:
  - `test` (vitest) — declared, currently failing (SES Region + agenda load
    failures per May doc; re-verify current state before relying on it)
  - `lint` (eslint) — declared
  - `typecheck` (tsc) — declared, currently timing out per May doc
  - `build` (next build) — declared, currently timing out per May doc
- `artifacts`: `.next/` build output; Prisma client (generated into
  `node_modules/@prisma/client`)
- `permissions.authority`: `approval_required`
- `permissions.network.allow`: **true**
- `permissions.filesystem.write`: **true** (build output, Prisma generate)
- `permissions.secrets.env_names`: operator-specified from `.env.local`
- `rollback.strategy`: `git_revert`
- `provenance.declared_by`: operator
- `provenance.purpose`: field ops automation platform (jobs, inventory, media,
  social, leads, calendar, users); production stabilization + field ops launch
  is the paid-work angle per the May monetization doc

**Review items (do not invent values):**
- Dirty working tree with active admin-users feature development — preserve as
  read-only; do not force writes to manufacture test conditions
- PRODUCTION_HARDENING_CHECKLIST.md has merge conflict markers (still modified
  in working tree) — do not treat as approved checklist until resolved
- Test suite currently failing (SES Region + agenda load) — record as known
  failure, not a passing check
- Typecheck and build currently timing out — record as known reliability issue
- Exact secret env names must come from operator (many integrations = many keys)
- STAGING_SIGNOFF_CHECKLIST.md is incomplete — acceptance criteria need operator
  definition

---

## Cross-project observations

1. **All three need `network.allow: true`.** The existing registry pattern of
   `network.allow: false` is wrong for deployed business web apps. This is a
   pattern-level correction, not a per-project invention.

2. **None have real test suites that currently pass.** CoverCoINC has no test
   script at all. WGTT's "test" is just lint. SkylightPros has a real vitest
   suite but it's currently failing. The contracts should record these gaps
   rather than invent passing checks.

3. **SkylightPros is the most complex and most dirty.** It has the most
   integrations, the richest docs, the most check surface, and the most active
   dirty work (admin users feature). It's also the one with current build/
   typecheck timeouts and test failures. The contract must preserve its dirty
   state.

4. **WGTT has the most complete deployment documentation** (SETUP.md,
   ADMIN_OPERATIONS_PLAN.md, AWS Amplify checklist) but its PROJECT_STATUS.md
   is stale. The ADMIN_OPERATIONS_PLAN.md has open ownership questions that
   affect acceptance criteria.

5. **CoverCoINC is the simplest but least documented.** Boilerplate README,
   placeholder audit notes, no deployment docs. The May doc's recommended first
   sprint (launch/forms/domain/analytics handoff) depends on docs that don't
   exist yet.

6. **All three have real secret env vars in `.env`/`.env.local` files.**
   Contracts must reference secret env names without reading or reproducing
   values. The operator must specify which env names each contract needs.

## What F2 does not do

- Does not attach any of these contracts to the registry (that's F3, requiring
  operator approval with idempotency receipts)
- Does not read secret values from `.env` files
- Does not run build/test/lint commands against these projects (that would
  mutate state and touch secret-dependent code paths)
- Does not resolve the dirty work in skylightpros
- Does not resolve the merge conflicts in skylightpros's checklist
- Does not invent env names, deployment targets, or acceptance criteria

## What F3 needs from the operator

Before any of these three contracts can be attached:

1. **Deployment target** for each (Vercel, Amplify, custom, etc.) — affects
   artifact paths and run commands
2. **Secret env names** each contract is allowed to reference (not values)
3. **Acceptance criteria** for each — what "done" means for CoverCoINC launch,
   WGTT admin operations, SkylightPros stabilization
4. **Authority level** for each — are these approval_required, or does one of
   them need a different authority class?
5. **Dirty work decision** for skylightpros — is the current admin-users work
   in progress something Arda should be aware of but not touch, or is it
   excluded from the contract's mutation scope?

## Source evidence

- `git -C <repo> status --short`, `git -C <repo> log --oneline -5`,
  `git -C <repo> remote -v`, `git -C <repo> rev-list --count HEAD..origin/master`
- `cat <repo>/package.json` for commands, dependencies, package manager
- `ls -la <repo>/` and `find <repo>/src -type f` for structure
- `cat <repo>/.env`, `<repo>/.env.local`, `<repo>/.env.example` for env key
  names only (values not read)
- `cat <repo>/README.md`, `audit-notes.md`, `SETUP.md`, `PROJECT_STATUS.md`,
  `ADMIN_OPERATIONS_PLAN.md`, `PRODUCTION_HARDENING_CHECKLIST.md`,
  `STAGING_SIGNOFF_CHECKLIST.md`, `docs/*.md`, `amplify.yml`, `components.json`
- `PROJECT_MONETIZATION_STATUS_2026-05-13.md` for monetization context
