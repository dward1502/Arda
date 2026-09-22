---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "RUMIL"
  status: "draft"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "contract", "fabric", "f3", "business"]
---

> 🜏 Soterion: 📜 contract | owner: RUMIL | status: draft | reviewed: 2026-09-17

# F3 — Business App Contracts (draft)

Three project contracts drafted for the Workbench registry, reconciled against
actual source and commands during F2. All three are `approval_required`. None
attached yet — attachment requires operator approval with idempotency receipts
per the plan.

Commands and check results below were **verified live on 2026-09-17**, not
copied from stale documentation. Env names are listed as declared names only;
no secret values were read or reproduced.

## Contract 1 — CoverCoINC

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "cc-20260917-001",
      "name": "covercoinc",
      "kind": "web-app",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/CoverCoINC",
      "repository": {
        "remote": "https://github.com/dward1502/CoverCoINC.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "npm",
      "node_version": ">=20 <21"
    },
    "commands": [
      {
        "id": "build",
        "program": "npm",
        "args": ["run", "build"],
        "working_dir": "/var/home/mythos/Eregion/CoverCoINC",
        "timeout_seconds": 300,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passed",
          "exit_code": 0,
          "notes": "next build produced .next/ output; 0 errors"
        }
      },
      {
        "id": "lint",
        "program": "npm",
        "args": ["run", "lint"],
        "working_dir": "/var/home/mythos/Eregion/CoverCoINC",
        "timeout_seconds": 120,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passed",
          "exit_code": 0,
          "notes": "0 ESLint warnings or errors; next lint deprecation note emitted but non-fatal"
        }
      },
      {
        "id": "start",
        "program": "npm",
        "args": ["run", "start"],
        "working_dir": "/var/home/mythos/Eregion/CoverCoINC",
        "timeout_seconds": 60,
        "verified": null
      },
      {
        "id": "dev",
        "program": "npm",
        "args": ["run", "dev"],
        "working_dir": "/var/home/mythos/Eregion/CoverCoINC",
        "timeout_seconds": 60,
        "verified": null
      }
    ],
    "checks": [
      {
        "id": "lint",
        "command": "lint",
        "status": "passing",
        "verified_at": "2026-09-17"
      },
      {
        "id": "build",
        "command": "build",
        "status": "passing",
        "verified_at": "2026-09-17"
      },
      {
        "id": "test",
        "command": "test",
        "status": "none_defined",
        "notes": "package.json has no test script; gap recorded, not invented"
      }
    ],
    "artifacts": [
      {
        "id": "build-output",
        "path": ".next/",
        "type": "build-output",
        "verified": {
          "exists_at": "2026-09-17",
          "notes": "present from prior build"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": true,
        "notes": "deployed web app requires outbound for Resend email, Supabase client, SES"
      },
      "filesystem": {
        "write": true,
        "notes": "build output writes to .next/; package-lock.json updates possible"
      },
      "secrets": {
        "env_names": [
          "EMAIL",
          "RESEND_API_KEY",
          "SES_REGION"
        ],
        "notes": "declared names only from CoverCoINC/.env; values not read or reproduced"
      }
    },
    "protected_paths": [
      {
        "path": ".env",
        "policy": "read_only",
        "notes": "contains real secret values; do not mutate"
      },
      {
        "path": ".env.local",
        "policy": "do_not_create_or_touch",
        "notes": "does not exist in this checkout; if created, operator must supply values"
      }
    ],
    "dirty_worktree_policy": {
      "mode": "read_only",
      "modified_files": [
        "README.md"
      ],
      "notes": "README.md is unstaged modified; preserve as-is until operator cleans up"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md",
        "/var/home/mythos/Eregion/CoverCoINC/README.md",
        "/var/home/mythos/Eregion/CoverCoINC/audit-notes.md"
      ],
      "plans": [
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md"
      ],
      "notes": "README is boilerplate; audit-notes.md is placeholder. Acceptance criteria not yet defined in-project."
    },
    "acceptance": {
      "human_visible": [
        "npm run build passes",
        "npm run lint passes",
        "deployment target configured (operator to specify)",
        "contact/quote forms verified with Resend in production-like environment",
        "Supabase server/client data paths verified with real env vars",
        "deploy/domain/analytics/SEO checklist completed"
      ],
      "audit_integrated": true,
      "audit_refs": [
        "/var/home/mythos/Eregion/audits/CoverCoINC/audits/structure-audit.md",
        "/var/home/mythos/Eregion/audits/CoverCoINC/audits/startup-audit.md",
        "/var/home/mythos/Eregion/audits/CoverCoINC/audits/memory-audit.md"
      ],
      "audit_status": "completed_2026-03-25",
      "audit_notes": "3 audits done (structure, startup, memory) by Hermes Agent 2026-03-25; recommendations exist but not yet actioned"
    },
    "risks": {
      "current": [
        "No test script — quality gate is lint + build only",
        "README is Next.js boilerplate — no project-specific onboarding",
        "audit-notes.md is placeholder — no findings documented",
        "No deployment/runbook docs exist yet",
        "SES_REGION in .env — SES config was failing in May 2026 doc; current state re-verified passing but env-dependent"
      ],
      "next_objective": "operator to specify deployment target and acceptance criteria; first paid sprint per May monetization doc: launch/forms/domain/analytics handoff"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Business web app — CoverCoINC lead-generation/contact website. Deployment, form/email verification, Supabase/env setup, SEO/content polish, conversion tracking are the immediate paid-work angle per PROJECT_MONETIZATION_STATUS_2026-05-13.md. Commands and check results verified live 2026-09-17."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-covercoinc-contract",
  "idempotency_key": "f3-covercoinc-20260917"
}
```

## Contract 2 — WGTT (World Stage Travels)

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "wgtt-20260917-001",
      "name": "wgtt",
      "kind": "web-app",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/wgtt",
      "repository": {
        "remote": "https://github.com/dward1502/wgtt.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "pnpm",
      "node_version": ">=18"
    },
    "commands": [
      {
        "id": "build",
        "program": "pnpm",
        "args": ["run", "build"],
        "working_dir": "/var/home/mythos/Eregion/wgtt",
        "timeout_seconds": 300,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passed",
          "exit_code": 0,
          "notes": "next build with NODE_OPTIONS suppress-baseline-warnings; .next/ output produced"
        }
      },
      {
        "id": "lint",
        "program": "pnpm",
        "args": ["run", "lint"],
        "working_dir": "/var/home/mythos/Eregion/wgtt",
        "timeout_seconds": 120,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passing",
          "exit_code": 0,
          "notes": "0 errors, 17 warnings (all unused-var and no-img-element warnings); May 2026 doc reported 51 problems/16 errors — resolved since"
        }
      },
      {
        "id": "start",
        "program": "pnpm",
        "args": ["run", "start"],
        "working_dir": "/var/home/mythos/Eregion/wgtt",
        "timeout_seconds": 60,
        "verified": null
      },
      {
        "id": "dev",
        "program": "pnpm",
        "args": ["run", "dev"],
        "working_dir": "/var/home/mythos/Eregion/wgtt",
        "timeout_seconds": 60,
        "verified": null
      }
    ],
    "checks": [
      {
        "id": "lint",
        "command": "lint",
        "status": "passing",
        "verified_at": "2026-09-17",
        "notes": "17 warnings remain; 0 errors"
      },
      {
        "id": "build",
        "command": "build",
        "status": "passing",
        "verified_at": "2026-09-17"
      },
      {
        "id": "test",
        "command": "test",
        "status": "none_suite",
        "notes": "package.json test script runs pnpm lint — not a real test suite; gap recorded"
      }
    ],
    "artifacts": [
      {
        "id": "build-output",
        "path": ".next/",
        "type": "build-output",
        "verified": {
          "exists_at": "2026-09-17",
          "notes": "present from prior build"
        }
      },
      {
        "id": "database-schema",
        "path": "database/schema.sql",
        "type": "schema",
        "verified": {
          "exists_at": "2026-09-17"
        }
      },
      {
        "id": "database-rls",
        "path": "database/rls_policies.sql",
        "type": "security-policy",
        "verified": {
          "exists_at": "2026-09-17"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": true,
        "notes": "WeTravel API, Travefy API, Crossbar webhooks, Resend, Supabase, OpenRouter all require outbound network"
      },
      "filesystem": {
        "write": true,
        "notes": "build output writes to .next/; pnpm-lock.yaml updates possible"
      },
      "secrets": {
        "env_names": [
          "WETRAVEL_PARTNER_API_KEY",
          "WETRAVEL_WEBHOOK_SIGNING_SECRET",
          "CROSSBAR_WEBHOOK_SECRET",
          "SUPABASE_SERVICE_ROLE_KEY",
          "TRAVEFY_API_KEY",
          "OPENROUTER_API_KEY",
          "RESEND_API_KEY"
        ],
        "notes": "declared names only from wgtt/.env.local; values not read or reproduced. NEXT_PUBLIC_* vars are public client vars, not secrets. Admin_ALLOWED_EMAILS is config, not secret."
      }
    },
    "protected_paths": [
      {
        "path": ".env.local",
        "policy": "read_only",
        "notes": "contains real secret values (19 keys); do not mutate"
      },
      {
        "path": ".env.example",
        "policy": "read_only",
        "notes": "template file; reference only"
      }
    ],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17; no dirty work to preserve"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md",
        "/var/home/mythos/Eregion/wgtt/SETUP.md",
        "/var/home/mythos/Eregion/wgtt/PROJECT_STATUS.md",
        "/var/home/mythos/Eregion/wgtt/docs/ADMIN_OPERATIONS_PLAN.md",
        "/var/home/mythos/Eregion/wgtt/docs/AMAZON-AWS-AMPLIFY-CHECKLIST.md"
      ],
      "plans": [
        "/var/home/mythos/Eregion/wgtt/docs/ADMIN_OPERATIONS_PLAN.md",
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md"
      ],
      "notes": "PROJECT_STATUS.md is stale (says Next.js 14, March 2026; actual is Next.js 16.0.5). ADMIN_OPERATIONS_PLAN.md has open ownership questions (trip/booking/itinerary/payment/roster) that affect acceptance criteria."
    },
    "acceptance": [
      "pnpm build passes",
      "pnpm lint passes (0 errors; warnings tracked)",
      "deployment target configured (operator to specify — AWS Amplify checklist exists but not completed)",
      "Supabase schema/migrations/RLS verified in actual target project",
      "WeTravel sync/admin flows, webhook ingestion, and email/contact flows verified",
      "ADMIN_OPERATIONS_PLAN.md open questions settled (trip/booking/itinerary/payment/roster ownership)",
      "PROJECT_STATUS.md updated to match current code (Next.js 16.0.5, current integrations)"
    ],
    "audit_integrated": true,
    "audit_refs": [
      "/var/home/mythos/Eregion/audits/wgtt/"
    ],
    "audit_status": "audit_folder_exists_no_files_2026-03-26",
    "audit_notes": "wgtt/audits/ folder exists but contains no audit files as of 2026-03-26; PROJECTS_REPORT.md lists wgtt as 'Modern & Well-Structured, Low priority' but no per-category audits completed. Audit gap relative to CoverCoINC."
  },
  "approval_id": "",
  "proposal_id": "f3-wgtt-contract",
  "idempotency_key": "f3-wgtt-20260917"
}
```

## Contract 3 — SkylightPros

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "sp-20260917-001",
      "name": "skylightpros",
      "kind": "web-app",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/skylightpros",
      "repository": {
        "remote": "https://github.com/dward1502/skylightpros.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "pnpm",
      "node_version": ">=20"
    },
    "commands": [
      {
        "id": "build",
        "program": "pnpm",
        "args": ["run", "build"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 300,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passed",
          "exit_code": 0,
          "notes": "next build --no-lint produced .next/ output; May 2026 doc reported 300s timeout — resolved since"
        }
      },
      {
        "id": "test",
        "program": "pnpm",
        "args": ["run", "test"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 120,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passing",
          "exit_code": 0,
          "notes": "32 tests, 8 test files passed; May 2026 doc reported SES Region + agenda failures — resolved since"
        }
      },
      {
        "id": "lint",
        "program": "pnpm",
        "args": ["run", "lint"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 120,
        "verified": {
          "ran_at": "2026-09-17",
          "result": "passing",
          "exit_code": 0,
          "notes": "0 ESLint errors or warnings"
        }
      },
      {
        "id": "typecheck",
        "program": "pnpm",
        "args": ["run", "typecheck"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 300,
        "verified": {
          "ran_at": null,
          "result": "not_verified_current_state",
          "notes": "May 2026 doc reported 240s timeout; not re-verified on 2026-09-17. Build passing is a positive signal but typecheck not confirmed."
        }
      },
      {
        "id": "ci",
        "program": "pnpm",
        "args": ["run", "ci"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 600,
        "verified": {
          "ran_at": null,
          "result": "not_verified_current_state",
          "notes": "combines lint + typecheck + test + build; not run on 2026-09-17 due to typecheck uncertainty"
        }
      },
      {
        "id": "prisma:generate",
        "program": "pnpm",
        "args": ["run", "prisma:generate"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 60,
        "verified": null,
        "notes": "prebuild step; generates Prisma client"
      },
      {
        "id": "smoke:e2e",
        "program": "pnpm",
        "args": ["run", "smoke:e2e"],
        "working_dir": "/var/home/mythos/Eregion/skylightpros",
        "timeout_seconds": 120,
        "verified": null
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "passing",
        "verified_at": "2026-09-17",
        "notes": "32 tests, 8 files; SES Region + agenda failures from May 2026 resolved"
      },
      {
        "id": "lint",
        "command": "lint",
        "status": "passing",
        "verified_at": "2026-09-17"
      },
      {
        "id": "build",
        "command": "build",
        "status": "passing",
        "verified_at": "2026-09-17",
        "notes": "previously timing out per May 2026 doc; now passing"
      },
      {
        "id": "typecheck",
        "command": "typecheck",
        "status": "not_verified",
        "notes": "historically timing out (May 2026 doc, 240s); not re-verified 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "build-output",
        "path": ".next/",
        "type": "build-output",
        "verified": {
          "exists_at": "2026-09-17",
          "notes": "present from prior build"
        }
      },
      {
        "id": "prisma-client",
        "path": "node_modules/@prisma/client",
        "type": "generated-library",
        "verified": {
          "exists_at": "2026-09-17"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": true,
        "notes": "Supabase, SES, S3, SNS, Twilio, HubSpot, Google APIs, Anthropic, OpenAI, Agenda postgres backend, Redis/Upstash all require outbound network"
      },
      "filesystem": {
        "write": true,
        "notes": "build output writes to .next/; Prisma client generation writes to node_modules/@prisma/client; pnpm-lock.yaml updates possible"
      },
      "secrets": {
        "env_names": [
          "DATABASE_URL",
          "DIRECT_URL",
          "SUPABASE_SERVICE_ROLE_KEY",
          "NEXT_PUBLIC_SUPABASE_URL",
          "NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY",
          "NEXT_PUBLIC_RECAPTCHA_SITE_KEY",
          "RECAPTCHA_SECRET_KEY",
          "GOOGLE_CLIENT_ID",
          "GOOGLE_CLIENT_SECRET",
          "AWS_REGION",
          "RESEND_API_KEY",
          "TWILIO_ACCOUNT_SID",
          "TWILIO_AUTH_TOKEN",
          "TWILIO_PHONE_NUMBER",
          "HUBSPOT_ACCESS_TOKEN",
          "HUBSPOT_APP_SECRET",
          "HUBSPOT_PRIVATE_APP_TOKEN",
          "LEAD_INGEST_TOKEN",
          "YELP_API_KEY",
          "YELP_CLIENT_ID",
          "ANTHROPIC_API_KEY",
          "ANTHROPIC_MODEL",
          "GOOGLE_CALENDAR_CLIENT_EMAIL",
          "GOOGLE_CALENDAR_ID",
          "GOOGLE_CALENDAR_PRIVATE_KEY",
          "GOOGLE_CALENDAR_TIMEZONE",
          "EMAIL_FROM",
          "ADMIN_EMAILS"
        ],
        "notes": "28 declared env names from skylightpros/.env.example (full set) + 11 currently in .env.local. Values not read or reproduced. Operator to confirm which are required for the contract's scoped authority vs nice-to-have."
      }
    },
    "protected_paths": [
      {
        "path": ".env.local",
        "policy": "read_only",
        "notes": "contains real secret values (11 keys currently active); do not mutate"
      },
      {
        "path": ".env.example",
        "policy": "read_only",
        "notes": "full 28-var template; reference only"
      },
      {
        "path": ".env",
        "policy": "read_only",
        "notes": "exists; contains real values; do not mutate"
      }
    ],
    "dirty_worktree_policy": {
      "mode": "read_only_aware",
      "modified_files": [
        "M PRODUCTION_HARDENING_CHECKLIST.md",
        "M README.md",
        "M docs/CONFIG_INDEX.md",
        "M docs/TESTING_CHECKLIST.md",
        "M middleware.ts",
        "M src/app/(public)/login/page.tsx",
        "M src/app/admin/components/AdminTopBar.tsx",
        "M src/app/admin/layout.tsx",
        "A src/app/admin/users/UsersInviteClient.tsx",
        "A src/app/admin/users/page.tsx",
        "A src/app/api/admin/users/invite/route.ts",
        "M src/lib/access-control.ts",
        "A src/lib/admin/users.ts",
        "M src/lib/field-access.ts"
      ],
      "notes": "14 modified files as of 2026-09-17. Active admin-users feature development in progress (new files + modified files). Arda is aware of this dirty work and should clean it up — meaning Arda should help get the tree to a clean state, not force writes to manufacture test conditions. No writes into this tree until operator approves mutation scope. PRODUCTION_HARDENING_CHECKLIST.md modified but no conflict markers present as of 2026-09-17 (resolved since May 2026 doc flagged them)."
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on master branch; note dirty worktree must be resolved before revert is clean"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md",
        "/var/home/mythos/Eregion/skylightpros/AGENT.md",
        "/var/home/mythos/Eregion/skylightpros/STRUCTURE_AND_SCHEMA.md",
        "/var/home/mythos/Eregion/skylightpros/SUPABASE_FINAL.md",
        "/var/home/mythos/Eregion/skylightpros/ROUTES_AND_ENTRYPOINTS.md",
        "/var/home/mythos/Eregion/skylightpros/PRODUCTION_HARDENING_CHECKLIST.md",
        "/var/home/mythos/Eregion/skylightpros/STAGING_SIGNOFF_CHECKLIST.md",
        "/var/home/mythos/Eregion/skylightpros/progress.txt",
        "/var/home/mythos/Eregion/skylightpros/FIELD_WORKER_BACKLOG.md",
        "/var/home/mythos/Eregion/skylightpros/docs/TASKS.md"
      ],
      "plans": [
        "/var/home/mythos/Eregion/PROJECT_MONETIZATION_STATUS_2026-05-13.md",
        "/var/home/mythos/Eregion/skylightpros/docs/TASKS.md",
        "/var/home/mythos/Eregion/skylightpros/FIELD_WORKER_BACKLOG.md"
      ],
      "notes": "Richest documentation of the three business apps. STAGING_SIGNOFF_CHECKLIST.md incomplete per May 2026 doc (missing sign-off date, approvers, known-issues). PRODUCTION_HARDENING_CHECKLIST.md is comprehensive but was modified in working tree."
    },
    "acceptance": [
      "pnpm test passes (32 tests currently passing)",
      "pnpm lint passes",
      "pnpm build passes",
      "pnpm typecheck completes without timeout (not yet re-verified 2026-09-17 — current gap)",
      "pnpm ci passes (lint + typecheck + test + build combined — blocked until typecheck verified)",
      "dirty worktree cleaned up (admin-users feature work landed or stashed)",
      "Supabase migrations and env contract verified in staging",
      "STAGING_SIGNOFF_CHECKLIST.md completed with sign-off date, approvers, known-issues",
      "PRODUCTION_HARDENING_CHECKLIST.md conflict markers resolved (currently clean) and reviews completed",
      "Demo field worker flow: job list -> open job -> media capture -> scan/outtake -> office media queue"
    ],
    "audit_integrated": true,
    "audit_refs": [
      "/var/home/mythos/Eregion/audits/skylightpros/"
    ],
    "audit_status": "audit_folder_exists_no_files_2026-03-26",
    "audit_notes": "skylightpros/audits/ folder exists but contains no audit files as of 2026-03-26; PROJECTS_REPORT.md lists skylightpros as 'Empty Project, Medium priority' — that assessment is stale (project is now full Next.js 15.5.4 app with 28 env vars, 32 tests, rich docs). Audit gap relative to CoverCoINC; update needed."
  },
  "approval_id": "",
  "proposal_id": "f3-skylightpros-contract",
  "idempotency_key": "f3-skylightpros-20260917"
}
```

## Cross-contract notes

### Command verification summary (all three, live 2026-09-17)

| Project | build | lint | test | typecheck | ci |
|---|---|---|---|---|---|
| CoverCoINC | pass | pass | none defined | not defined | not defined |
| WGTT | pass | pass (17 warnings) | test=lint only (gap) | not defined | not defined |
| SkylightPros | pass | pass | pass (32 tests) | not verified | not verified (blocked on typecheck) |

### Stale May 2026 doc corrections

The `PROJECT_MONETIZATION_STATUS_2026-05-13.md` doc reported these as failing/timeing out. All verified passing on 2026-09-17 except where noted:

- CoverCoINC build: doc said passed — still passing ✓
- CoverCoINC: doc said no test — still no test ✓
- WGTT build: doc said passed — still passing ✓
- WGTT lint: doc said 51 problems, 16 errors — now 0 errors, 17 warnings ✓ (resolved)
- SkylightPros test: doc said failing (SES Region + agenda) — now passing ✓ (resolved)
- SkylightPros build: doc said 300s timeout — now passing ✓ (resolved)
- SkylightPros typecheck: doc said 240s timeout — **not re-verified 2026-09-17** ✗ (still unknown)
- SkylightPros PRODUCTION_HARDENING_CHECKLIST.md conflict markers: doc flagged — **no conflict markers present 2026-09-17** ✓ (resolved)

### Audit integration status

| Project | Audit folder | Audit files | Status |
|---|---|---|---|
| CoverCoINC | `audits/CoverCoINC/audits/` | 3 files (structure, startup, memory) | completed 2026-03-25 |
| WGTT | `audits/wgtt/` | none | folder exists, no files; PROJECTS_REPORT.md lists as low priority but no per-category audits |
| SkylightPros | `audits/skylightpros/` | none | folder exists, no files; PROJECTS_REPORT.md assessment is stale (said empty project, now full app) |

The plan says "integrated and audit is performed and reviewed on each one (some have it done already)." CoverCoINC has 3 audits done. WGTT and SkylightPros have audit folders but no audit files — their PROJECTS_REPORT.md assessments are stale. This is a follow-up item, not a block to contract attachment.

### Dirty work summary

| Project | Dirty work | Policy |
|---|---|---|
| CoverCoINC | README.md modified (unstaged) | read_only; preserve until operator cleans up |
| WGTT | clean | n/a |
| SkylightPros | 14 modified files (active admin-users feature WIP) | read_only_aware; Arda aware, should clean up — help get tree clean, don't force writes to manufacture tests |

### What F3 does not do

- Does not attach these contracts to `data/workbench/projects.json` (F3 attachment requires operator approval with idempotency receipts — the `approval_id` fields above are empty pending that)
- Does not read secret values from `.env` files (env_names are declared names only)
- Does not run typecheck or ci for SkylightPros (blocked on typecheck uncertainty; running ci could mutate state)
- Does not resolve the dirty work in skylightpros (operator said Arda should clean it up — that's a separate action)
- Does not invent deployment targets (operator said "no" to deployment specification)
- Does not define acceptance criteria beyond what's already documented in-project

## Source evidence

- Live command verification 2026-09-17: `npm run lint`, `npm run build` (CoverCoINC); `pnpm lint`, `pnpm build` (WGTT); `pnpm test`, `pnpm lint`, `pnpm build` (SkylightPros)
- `git -C <repo> status --short`, `git -C <repo> log --oneline -5`, `git -C <repo> remote -v`
- `cat <repo>/package.json` for commands, dependencies, package manager
- `cat <repo>/.env`, `<repo>/.env.local`, `<repo>/.env.example` for env key names only (values not read)
- `cat <repo>/README.md`, `audit-notes.md`, `SETUP.md`, `PROJECT_STATUS.md`, `ADMIN_OPERATIONS_PLAN.md`, `PRODUCTION_HARDENING_CHECKLIST.md`, `STAGING_SIGNOFF_CHECKLIST.md`, `progress.txt`, `FIELD_WORKER_BACKLOG.md`, `docs/TASKS.md`, `AGENT.md`, `STRUCTURE_AND_SCHEMA.md`, `SUPABASE_FINAL.md`, `ROUTES_AND_ENTRYPOINTS.md`
- `ls audits/<project>/audits/` and `cat` per audit file for CoverCoINC; `ls audits/<project>/` for WGTT and SkylightPros (no files)
- `PROJECT_MONETIZATION_STATUS_2026-05-13.md` for monetization context and stale-vs-current comparison
