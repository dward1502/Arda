---
soterion:
  sigil: "SCROLL"
  glyph: "[scroll]"
  code_point: "U+1F4DC"
  role: "info-supply"
  owner: "RUMIL"
  status: "completed"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "fabric", "f5", "rumil"]
---

> Soterion: [scroll] | info-supply | owner: RUMIL | status: completed | reviewed: 2026-09-17

# F5 -- Rumil Project Information Supply

## Context

Rumil is responsible for supplying project purpose, requirements, source,
test, runtime, receipt, and dependency-drift comparisons to the daily-loop owner.

This document records the supply data for each production project currently
registered in the Arda workbench registry.

## Supply Target Projects

These are the production projects currently registered in the Arda workbench
registry as of 2026-09-17.

### 1. arda-tool-gate-source-review

**Purpose**: Arda Tool Gate source code review and integration readiness
assessment. Source inspection and cited retained findings only. No project
writes, network tools, secrets, deployment, or permission expansion.

**Requirements**:
- Source code inspection
- Cited retained findings only
- No project writes, network tools, secrets, deployment, or permission expansion

**Source**:
- Root: /var/home/mythos/Eregion/Arda-Tool-Gate
- Remote: https://github.com/dward1502/Arda-tool-gate.git
- Language: Rust
- Adapter: hermes

**Test**:
- Test command: none (read_only project)

**Runtime**:
- Adapter: hermes
- Authority: read_only

**Receipt**:
- Approval ID: hermes-user-message-710481
- Proposal ID: c32-real-tool-gate-source-attachment
- Idempotency key: c32-tool-gate-attachment-user-710481

**Dependency-drift**:
- Depends on Arda core
- Uses Manwe inference adapter

### 2. arda-rust-example

**Purpose**: Arda Rust example project. Runs Arda core tests; demonstrates Rust
project structure.

**Requirements**:
- Run Arda core tests
- Demonstrate Rust project structure

**Source**:
- Root: . (Arda monorepo root)
- Remote: (Arda repository)
- Language: Rust
- Adapter: cargo

**Test**:
- Test command: cargo test -p arda-core

**Runtime**:
- Adapter: cargo

**Receipt**:
- Approval ID: stage4-research-approval
- Proposal ID: stage4-research-proposal
- Idempotency key: stage4-research-attach-v1

**Dependency-drift**:
- Depends on Arda core

### 3. arda-provider-route-audit

**Purpose**: Arda provider route audit within P1 scope. File-tool-only
inspection; no terminal checks, deployment, network, credential, or filesystem
mutation authority. Returns cited findings in retained result summary.

**Requirements**:
- File tools only
- No terminal checks, deployment, network, credential, or filesystem mutation
- Return cited findings in retained result summary, not created artifacts

**Source**:
- Root: . (Arda monorepo root)
- Remote: (Arda repository)
- Language: documentation
- Adapter: hermes

**Test**:
- Test command: none

**Runtime**:
- Adapter: hermes
- Authority: read_only

**Receipt**:
- Approval ID: cli-attachment-f66a4ca3-b917-4194-96b8-54e76a844437
- Proposal ID: c31-provider-route-audit-contract
- Idempotency key: c31-provider-route-audit-attachment-b26d9ca2-fc63-4b2d-bc84-db0fd99a6bd5

**Dependency-drift**:
- Depends on Arda core
- Depends on Manwe config, providers, fleet state

### 4. covercoinc

**Purpose**: CoverCoINC lead-generation / contact website. Deployment, form and
email verification, Supabase env setup, SEO/content polish, and conversion
tracking are the immediate paid-work angle.

**Requirements**:
- npm build passes
- npm lint passes
- Deployment target configured (operator to specify)
- Contact and quote forms verified with Resend in production-like environment
- Supabase server/client data paths verified with real env vars
- Deploy / domain / analytics / SEO checklist completed

**Source**:
- Root: /var/home/mythos/Eregion/CoverCoINC
- Remote: https://github.com/dward1502/CoverCoINC.git
- Language: TypeScript
- Package manager: npm
- Node version: >=20 <21
- Adapter: node

**Test**:
- Test command: none (no test script in package.json)
- Lint: npm run lint (passes, 0 errors)
- Build: npm run build (passes)

**Runtime**:
- Adapter: node
- Network allow: true (outbound needed for Resend email, Supabase client, SES)
- Filesystem write: true (.next/ build output)
- Secret env_names: EMAIL, RESEND_API_KEY, SES_REGION

**Receipt**:
- Approval ID: operator-approved-f3-business-covercoinc-e9406cdd-3d52-4309-81b5-52bf663f122b
- Proposal ID: operator-message:f3-business-attach-covercoinc-e9406cdd-3d52-4309-81b5-52bf663f122b
- Idempotency key: gateway:f3-business-attach-covercoinc-e9406cdd-3d52-4309-81b5-52bf663f122b

**Dependency-drift**:
- Next.js 15.3.1, React 19.1.0
- Supabase 2.103.3
- Resend 6.5.2
- AWS SDK SES v2

### 5. wgtt

**Purpose**: World Stage Travels travel booking platform. WeTravel / Travefy /
Crossbar integrations. Launch readiness plus admin operations integration is the
paid-work angle.

**Requirements**:
- pnpm build passes
- pnpm lint passes (0 errors, 17 warnings)
- Deployment target configured (operator to specify)
- Supabase schema / migrations / RLS verified in actual target project
- WeTravel sync / admin flows, webhook ingestion, email / contact flows verified
- Resolved open questions in ADMIN_OPERATIONS_PLAN.md

**Source**:
- Root: /var/home/mythos/Eregion/wgtt
- Remote: https://github.com/dward1502/wgtt.git
- Language: TypeScript
- Package manager: pnpm
- Adapter: node

**Test**:
- Test command: pnpm run lint (the test script runs lint -- not a real test suite)
- Lint: pnpm run lint (passes, 0 errors, 17 warnings)
- Build: pnpm run build (passes)

**Runtime**:
- Adapter: node
- Network allow: true (outbound needed for WeTravel API, Travefy API, Crossbar webhooks, Resend, Supabase, OpenRouter)
- Filesystem write: true (.next/ build output)
- Secret env_names: WETRAVEL_PARTNER_API_KEY, WETRAVEL_WEBHOOK_SIGNING_SECRET, CROSSBAR_WEBHOOK_SECRET, SUPABASE_SERVICE_ROLE_KEY, TRAVEFY_API_KEY, OPENROUTER_API_KEY, RESEND_API_KEY

**Receipt**:
- Approval ID: operator-approved-f3-business-wgtt-70656753-7af2-40fd-afcb-a0f823bd95e2
- Proposal ID: operator-message:f3-business-attach-wgtt-70656753-7af2-40fd-afcb-a0f823bd95e2
- Idempotency key: gateway:f3-business-attach-wgtt-70656753-7af2-40fd-afcb-a0f823bd95e2

**Dependency-drift**:
- Next.js 16.0.5, React 19.2.0
- Supabase 2.86.2
- WeTravel API
- Travefy API
- Crossbar webhooks
- Resend
- OpenRouter

### 6. skylightpros

**Purpose**: Skylight Pros field operations automation platform (jobs, inventory,
media, social, leads, calendar, users). Production stabilization plus field ops
launch is the paid-work angle.

**Requirements**:
- pnpm test passes (32 tests)
- pnpm lint passes
- pnpm build passes
- pnpm typecheck completes without timeout (not yet re-verified)
- pnpm ci passes (lint + typecheck + test + build -- blocked until typecheck verified)
- Dirty worktree cleaned up (admin-users feature work landed or stashed)
- Supabase migrations and env contract verified in staging
- STAGING_SIGNOFF_CHECKLIST.md completed (sign-off date, approvers, known-issues)
- PRODUCTION_HARDENING_CHECKLIST.md conflict markers resolved (currently clean) and review completed
- Field worker flow demo: job list -> open job -> media capture -> scan/outtake -> office media queue

**Source**:
- Root: /var/home/mythos/Eregion/skylightpros
- Remote: https://github.com/dward1502/skylightpros.git
- Language: TypeScript
- Package manager: pnpm
- Node version: >=20
- Adapter: node

**Test**:
- Test command: pnpm run test (passes, 32 tests, 8 files)
- Lint: pnpm run lint (passes)
- Build: pnpm run build (passes)
- Typecheck: pnpm run typecheck (not yet re-verified -- historically timed out at 240s)

**Runtime**:
- Adapter: node
- Network allow: true (outbound needed for Supabase, SES, S3, SNS, Twilio, HubSpot, Google APIs, Anthropic, OpenAI, Agenda PostgreSQL backend, Redis / Upstash)
- Filesystem write: true (.next/ build output, Prisma client generation)
- Secret env_names (28): DATABASE_URL, DIRECT_URL, SUPABASE_SERVICE_ROLE_KEY, NEXT_PUBLIC_SUPABASE_URL, NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY, NEXT_PUBLIC_RECAPTCHA_SITE_KEY, RECAPTCHA_SECRET_KEY, GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, AWS_REGION, RESEND_API_KEY, TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, TWILIO_PHONE_NUMBER, HUBSPOT_ACCESS_TOKEN, HUBSPOT_APP_SECRET, HUBSPOT_PRIVATE_APP_TOKEN, LEAD_INGEST_TOKEN, YELP_API_KEY, YELP_CLIENT_ID, ANTHROPIC_API_KEY, ANTHROPIC_MODEL, GOOGLE_CALENDAR_CLIENT_EMAIL, GOOGLE_CALENDAR_ID, GOOGLE_CALENDAR_PRIVATE_KEY, GOOGLE_CALENDAR_TIMEZONE, EMAIL_FROM, ADMIN_EMAILS

**Receipt**:
- Approval ID: operator-approved-f3-business-skylightpros-f6d1304d-1da1-463a-8af9-6b1fd8226fc5
- Proposal ID: operator-message:f3-business-attach-skylightpros-f6d1304d-1da1-463a-8af9-6b1fd8226fc5
- Idempotency key: gateway:f3-business-attach-skylightpros-f6d1304d-1da1-463a-8af9-6b1fd8226fc5

**Dependency-drift**:
- Next.js 15.5.4, React 19.1.0
- Prisma 5.22.0
- Supabase 2.103.0
- Resend 6.10.0
- Twilio 5.13.1
- Anthropic SDK 0.69.0
- OpenAI SDK 6.34.0
- Agenda 6.2.4
- Upstash Redis 1.37.0

## Supply Method

Rumil can supply this information to the daily-loop owner by:

1. Project list: regularly scanning the registry for production project entries
2. Project info: extracting metadata for each project
3. Test status: running each project's test command and recording the result
4. Runtime status: recording each project's runtime information
5. Dependency-drift: detecting dependency changes for each project

## Notes

- This information is generated from contracts stored in the registry.
- Actual source code must be read separately.
- Metadata does not grant permission.
