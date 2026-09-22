---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "RUMIL"
  status: "draft"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "contract", "fabric", "f3", "remaining-14"]
---

> 🜏 Soterion: 📜 contract | owner: RUMIL | status: draft | reviewed: 2026-09-17

# F3 — Remaining 14 Projects Contracts (draft)

Fourteen project contracts drafted for the Workbench registry, reconciled against
actual source and commands during F2. All are `approval_required`. None attached yet —
attachment requires operator approval with idempotency receipts per the plan.

Commands and check results below were **not** live-verified on 2026-09-17 for the
14 remaining projects (deferred to F3 attachment phase). Env names are listed as
declared names only where known; no secret values were read or reproduced.

---

## Contract 1 — Arda-Agent-Loop-Contract

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "c440f21c-14f1-4183-9bd8-8ee320a50bb1",
      "name": "arda-agent-loop-contract",
      "kind": "rust-library",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Agent-Loop-Contract",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-Agent-Loop-Contract.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17; inferred from crate shape"
      },
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17; inferred from crate shape"
      },
      {
        "id": "doc",
        "program": "cargo",
        "args": ["doc", "--no-deps"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "library",
        "path": "target/debug/libagent_loop_contract.rlib",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "library crate; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Agent-Loop-Contract/README.md",
        "/var/home/mythos/Eregion/Arda-Agent-Loop-Contract/Cargo.toml"
      ],
      "notes": "README describes portable inspect-act-verify loop contract and local validator"
    },
    "acceptance": [
      "cargo build passes",
      "cargo test passes",
      "cargo doc --no-deps generates documentation",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Cargo.toml references repository as github.com/dward1502/agent-loop-contract (name differs from physical remote Arda-Agent-Loop-Contract) — operator to clarify"
      ],
      "next_objective": "operator to confirm public release intent and command verification"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Portable inspect-act-verify operating loop contract and local validator. Public release candidate. Commands and check results not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-agent-loop-contract",
  "idempotency_key": "f3-arda-agent-loop-contract-20260917"
}
```

---

## Contract 2 — Arda-Council

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "c9d2682b-4311-4d5c-8b74-d2b36203bb9c",
      "name": "arda-council",
      "kind": "rust-library",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Council",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-Council.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17; README documents cargo test"
      },
      {
        "id": "doc",
        "program": "cargo",
        "args": ["doc", "--no-deps"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "not live-verified 2026-09-17; README documents cargo doc --no-deps"
      },
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "library",
        "path": "target/debug/libannunimas_council.rlib",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "library crate; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "target/ deletion tracked in git (build artifacts removed from tracking); no source file modifications. No active source dirt."
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Council/README.md",
        "/var/home/mythos/Eregion/Arda-Council/Cargo.toml"
      ],
      "notes": "Blueprint-stage Rust crate for multi-agent boardroom records and consensus surfaces"
    },
    "acceptance": [
      "cargo test passes",
      "cargo doc --no-deps generates documentation",
      "path dependencies on Annunimas workspace crates resolved or waived",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Path dependencies on ../../Annunimas/crates/annunimas-core and ../../Annunimas/crates/annunimas-mnemosyne — external workspace references, build in isolation may require those paths",
        "target/ was previously committed and is now deleted from tracking — indicates prior build-in-commit history",
        "Status: blueprint-stage per README — early development, not production-ready"
      ],
      "next_objective": "operator to confirm public release intent, resolve path dep scope, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Multi-agent boardroom deliberation and consensus building. Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-council",
  "idempotency_key": "f3-arda-council-20260917"
}
```

---

## Contract 3 — Arda-Forge-Mind

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "775697ca-a8fa-4465-8ebc-cc16d0c6921b",
      "name": "arda-forge-mind",
      "kind": "rust-application",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Forge-Mind",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-Forge-Mind.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 600,
        "verified": null,
        "notes": "not live-verified 2026-09-17; heavy dependencies (tokio full, sha2) — longer build expected"
      },
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "binary",
        "path": "target/debug/annunimas-forge-mind",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "local 3D asset forge; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "target/ deletion tracked in git (build artifacts removed from tracking); no source file modifications. No active source dirt."
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Forge-Mind/README.md",
        "/var/home/mythos/Eregion/Arda-Forge-Mind/Cargo.toml"
      ],
      "notes": "Sovereign 3D asset forge for Annunimas — Blender, texturing, slicing, and ARDA scene production"
    },
    "acceptance": [
      "cargo build passes",
      "cargo test passes",
      "path dependency on annunimas-tool-harness resolved or waived",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Path dependency on annunimas-tool-harness (local path) — external workspace reference",
        "Heavy dependencies (tokio full features, sha2) — build may be resource-intensive",
        "target/ was previously committed and is now deleted from tracking"
      ],
      "next_objective": "operator to confirm public release intent, resolve path dep scope, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Sovereign 3D asset forge for Annunimas. Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-forge-mind",
  "idempotency_key": "f3-arda-forge-mind-20260917"
}
```

---

## Contract 4 — Arda-HUD

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "c4a9c86e-8298-4985-87b0-e391bbcc5828",
      "name": "arda-hud",
      "kind": "desktop-app",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-HUD",
      "repository": {
        "remote": "https://github.com/dward1502/arda-hud.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "pnpm",
      "node_version": ">=20.19.0",
      "pnpm_version": "10.8.1"
    },
    "commands": [
      {
        "id": "build",
        "program": "pnpm",
        "args": ["run", "build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "tsc && vite build; not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "pnpm",
        "args": ["run", "test"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "vitest run; not live-verified 2026-09-17"
      },
      {
        "id": "dev",
        "program": "pnpm",
        "args": ["run", "dev"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "vite dev server; not live-verified 2026-09-17"
      },
      {
        "id": "tauri:build:stable",
        "program": "pnpm",
        "args": ["run", "tauri:build:stable"],
        "working_dir": "./",,
        "timeout_seconds": 600,
        "verified": null,
        "notes": "Tauri no-bundle build with env setup; requires PKG_CONFIG_PATH, __NV_DISABLE_EXPLICIT_SYNC, WEBKIT_* , GDK_BACKEND=x11"
      },
      {
        "id": "tauri:dev:stable",
        "program": "pnpm",
        "args": ["run", "tauri:dev:stable"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "Tauri dev with env setup; same env requirements as tauri:build:stable"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "vitest run not executed 2026-09-17"
      },
      {
        "id": "build",
        "command": "build",
        "status": "not_verified",
        "notes": "tsc && vite build not executed 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "dist",
        "path": "dist/",
        "type": "build-output",
        "verified": {
          "exists_at": "2026-09-17",
          "notes": "present from prior build"
        }
      },
      {
        "id": "tauri-binary",
        "path": "src-tauri/target/release/arda_hud",
        "type": "native-binary",
        "verified": null,
        "notes": "produced by tauri:build:stable; not verified 2026-09-17"
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "desktop app; no outbound needs declared in contract"
      },
      "filesystem": {
        "write": true,
        "notes": "build writes to dist/; Tauri build writes to src-tauri/target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files found; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "read_only",
      "modified_files": [
        "scripts/node_monitor.sh",
        "src-tauri/Cargo.toml",
        "src/lib/ardaBundleTypes.ts",
        "src/lib/ardaSource.ts"
      ],
      "notes": "4 source files modified (unstaged). Preserve as-is until operator cleans up or approves mutation scope. Do not force writes to manufacture test conditions."
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-HUD/README.md",
        "/var/home/mythos/Eregion/Arda-HUD/package.json",
        "/var/home/mythos/Eregion/Arda-HUD/INDEX.md",
        "/var/home/mythos/Eregion/Arda-HUD/ARDA_CONTRACTS_MANIFEST.md"
      ],
      "notes": "Operator-facing frontend/desktop shell for Annunimas. Comprehensive docs including contract manifests, scene contracts, integration docs."
    },
    "acceptance": [
      "pnpm build passes (tsc + vite build)",
      "pnpm test passes (vitest run)",
      "Tauri build passes with stable env setup (PKG_CONFIG_PATH, __NV_DISABLE_EXPLICIT_SYNC, WEBKIT_* , GDK_BACKEND=x11)",
      "4 dirty files resolved (landed, stashed, or operator-approved mutation scope)",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Dirty worktree: 4 modified source files — active development in progress",
        "Tauri build requires specific environment variables and may need distrobox/lothlorien for native proof",
        "scripts/node_monitor.sh modified — shell script, review for correctness",
        "src-tauri/Cargo.toml modified — Tauri Rust config changed",
        "src/lib/ardaBundleTypes.ts + src/lib/ardaSource.ts modified — core TypeScript source changes"
      ],
      "next_objective": "operator to confirm public release intent, resolve dirty work, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Operator-facing frontend/desktop shell for Annunimas (React 19 + TypeScript + Vite + Tauri 2). Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-hud",
  "idempotency_key": "f3-arda-hud-20260917"
}
```

---

## Contract 5 — Arda-Human

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "fe173a72-231d-4329-92d5-416c0e5b4089",
      "name": "arda-human",
      "kind": "rust-library",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Human",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-human.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "library",
        "path": "target/debug/libannunimas_human.rlib",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "library crate; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Human/README.md",
        "/var/home/mythos/Eregion/Arda-Human/Cargo.toml"
      ],
      "notes": "Human tenant layer for Annunimas. Minimal README (424 bytes). Path deps on Annunimas workspace crates."
    },
    "acceptance": [
      "cargo build passes",
      "cargo test passes",
      "path dependencies on Annunimas workspace crates resolved or waived",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Path dependencies on ../../Annunimas/crates/annunimas-core and ../../Annunimas/crates/annunimas-mnemosyne — external workspace references",
        "publish = false in Cargo.toml — not intended for crates.io",
        "Minimal README — acceptance criteria need operator definition"
      ],
      "next_objective": "operator to confirm public release intent, resolve path dep scope, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Human tenant layer for Annunimas. Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-human",
  "idempotency_key": "f3-arda-human-20260917"
}
```

---

## Contract 6 — Arda-Service-Registry

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "25fd53a6-ac10-4978-b89b-78f1dde8a305",
      "name": "arda-service-registry",
      "kind": "rust-library",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Service-Registry",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-Service-Registry.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "library",
        "path": "target/debug/libannunimas_service_registry.rlib",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "library crate; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Service-Registry/README.md",
        "/var/home/mythos/Eregion/Arda-Service-Registry/Cargo.toml"
      ],
      "notes": "Service registry for Annunimas. Substantial README (15560 bytes). src/, tests/, target/ directories."
    },
    "acceptance": [
      "cargo build passes",
      "cargo test passes",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Acceptance criteria from substantial README need operator confirmation"
      ],
      "next_objective": "operator to confirm public release intent, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Service registry for Annunimas. Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-service-registry",
  "idempotency_key": "f3-arda-service-registry-20260917"
}
```

---

## Contract 7 — Arda-Signal-Grid

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "30e180e1-05f5-4db7-be59-328cfe364717",
      "name": "arda-signal-grid",
      "kind": "rust-library",
      "class": "core-arda-part",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/Arda-Signal-Grid",
      "repository": {
        "remote": "https://github.com/dward1502/Arda-Signal-Grid.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "cargo",
      "language": "rust",
      "package_manager": "cargo"
    },
    "commands": [
      {
        "id": "build",
        "program": "cargo",
        "args": ["build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "cargo",
        "args": ["test"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "cargo test not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "library",
        "path": "target/debug/libannunimas_signal_grid.rlib",
        "type": "build-output",
        "verified": null
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "library crate; no outbound needs declared"
      },
      "filesystem": {
        "write": true,
        "notes": "cargo build writes to target/"
      },
      "secrets": {
        "env_names": [],
        "notes": "no .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/Arda-Signal-Grid/README.md",
        "/var/home/mythos/Eregion/Arda-Signal-Grid/Cargo.toml"
      ],
      "notes": "Signal grid for Annunimas. Lightweight crate — minimal dependencies (serde, serde_json, chrono). README (3133 bytes)."
    },
    "acceptance": [
      "cargo build passes",
      "cargo test passes",
      "public release scope confirmed by operator"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17"
      ],
      "next_objective": "operator to confirm public release intent, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Signal grid for Annunimas. Public release candidate. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-arda-signal-grid",
  "idempotency_key": "f3-arda-signal-grid-20260917"
}
```

---

## Contract 8 — wakita

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "43a2bbb3-fc34-4acb-8080-a2f653781c9a",
      "name": "wakita",
      "kind": "web-app",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/wakita",
      "repository": {
        "remote": "https://github.com/RavensNestInc/wakita.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "npm",
      "node_version": ">=18"
    },
    "commands": [
      {
        "id": "start",
        "program": "npm",
        "args": ["start"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "node server.js; not live-verified 2026-09-17"
      },
      {
        "id": "dev",
        "program": "npm",
        "args": ["run", "dev"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "nodemon server.js; not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "npm",
        "args": ["run", "test"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "jest --coverage; not live-verified 2026-09-17"
      },
      {
        "id": "build:static",
        "program": "npm",
        "args": ["run", "build:static"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "node scripts/build-static.js → dist-static/; not live-verified 2026-09-17"
      },
      {
        "id": "ts:check",
        "program": "npm",
        "args": ["run", "ts:check"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "tsc --noEmit; not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "jest --coverage not run 2026-09-17"
      },
      {
        "id": "ts:check",
        "command": "ts:check",
        "status": "not_verified",
        "notes": "tsc --noEmit not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "dist-static",
        "path": "dist-static/",
        "type": "build-output",
        "verified": {
          "exists_at": "2026-09-17",
          "notes": "present from prior static build"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": true,
        "notes": "Express app with OAuth (Facebook, Google), AWS (S3, Secrets Manager), MongoDB — needs outbound network"
      },
      "filesystem": {
        "write": true,
        "notes": "build:static writes to dist-static/; node_modules updates possible"
      },
      "secrets": {
        "env_names": [
          "MONGODB_URI",
          "JWT_SECRET",
          "FACEBOOK_APP_ID",
          "FACEBOOK_APP_SECRET",
          "FACEBOOK_CALLBACK_URL",
          "GOOGLE_CLIENT_ID",
          "GOOGLE_CLIENT_SECRET",
          "GOOGLE_CALLBACK_URL",
          "AWS_REGION",
          "AWS_ACCESS_KEY_ID",
          "AWS_SECRET_ACCESS_KEY",
          "SESSION_SECRET",
          "PORT",
          "NODE_ENV",
          "USE_LOCAL_PREVIEW_DATA",
          "SIMPLYRETS_BASE_URL",
          "SIMPLYRETS_USERNAME",
          "SIMPLYRETS_PASSWORD"
        ],
        "notes": "Declared names from wakita/.env.example (listed in README). Values not read or reproduced. Operator to confirm which are required for scoped authority vs nice-to-have."
      }
    },
    "protected_paths": [
      {
        "path": ".env",
        "policy": "do_not_create_or_touch",
        "notes": "no .env file in checkout; if created, operator must supply values"
      }
    ],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/wakita/README.md",
        "/var/home/mythos/Eregion/wakita/package.json",
        "/var/home/mythos/Eregion/wakita/AZURE_DEPLOY.md"
      ],
      "notes": "Real estate website — Wakita & Associates. Two modes: local Express preview and Azure Static Web Apps. Authors: Daniel Ward, Shun Wakita. License: Apache-2.0."
    },
    "acceptance": [
      "npm test passes (jest --coverage)",
      "npm run ts:check passes (tsc --noEmit)",
      "npm run build:static produces dist-static/",
      "deployment mode confirmed (Express-hosted vs Azure Static Web Apps)",
      "ownership boundary confirmed (RavensNestInc remote — operator to confirm mutation authority)"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Ownership boundary: RavensNestInc org (not dward1502) — operator to confirm mutation authority",
        "No .env file present in checkout — operator must supply values if running",
        "Two deployment modes (Express-hosted vs Azure Static) — contract needs operator confirmation of which mode is in scope",
        "Original upstream: shun1000/wakitaRE — external author fork; license Apache-2.0"
      ],
      "next_objective": "operator to confirm ownership boundary, deployment mode, supply env values, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Real estate website — Wakita & Associates (Express.js 2.0.0 + TypeScript). Two operating modes: local Express preview and Azure Static Web Apps. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-wakita",
  "idempotency_key": "f3-wakita-20260917"
}
```

---

## Contract 9 — filamentDB

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "7294038f-6aee-4fa1-b1bd-38b1010c60b4",
      "name": "filamentdb",
      "kind": "web-app",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/filamentDB",
      "repository": {
        "remote": "https://github.com/dward1502/filamentDB.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "yarn",
      "node_version": ">=18"
    },
    "commands": [
      {
        "id": "dev",
        "program": "npm",
        "args": ["run", "dev"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "next dev; not live-verified 2026-09-17"
      },
      {
        "id": "build",
        "program": "npm",
        "args": ["run", "build"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "next build; not live-verified 2026-09-17"
      },
      {
        "id": "start",
        "program": "npm",
        "args": ["run", "start"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "next start; not live-verified 2026-09-17"
      },
      {
        "id": "lint",
        "program": "npm",
        "args": ["run", "lint"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "next lint; not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "lint",
        "command": "lint",
        "status": "not_verified",
        "notes": "next lint not run 2026-09-17"
      },
      {
        "id": "build",
        "command": "build",
        "status": "not_verified",
        "notes": "next build not run 2026-09-17"
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
        "verified": null,
        "notes": "next build output; not verified 2026-09-17"
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": true,
        "notes": "AWS Amplify integration — needs outbound for AWS services"
      },
      "filesystem": {
        "write": true,
        "notes": "next build writes to .next/; node_modules updates possible"
      },
      "secrets": {
        "env_names": [],
        "notes": "Operator to specify from .env (148 bytes) and .env.production — values not read or reproduced. Deeper survey needed to identify env var names."
      }
    },
    "protected_paths": [
      {
        "path": ".env",
        "policy": "read_only",
        "notes": "contains real values (148 bytes); do not read values into contract"
      },
      {
        "path": ".env.production",
        "policy": "read_only",
        "notes": "contains real values; do not read values into contract"
      }
    ],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/filamentDB/README.md",
        "/var/home/mythos/Eregion/filamentDB/package.json"
      ],
      "notes": "Next.js 14.0.4 + React 18 + TypeScript web app with AWS Amplify. Only surface-surveyed (package.json + README.md). Deeper structure TBD — src/, amplify/, .graphqlconfig.yml not yet examined."
    },
    "acceptance": [
      "npm run build passes (next build)",
      "npm run lint passes (next lint)",
      "deeper survey completed (src/ structure, amplify/ config, env var identification)",
      "package manager confirmed (yarn.lock present — operator to confirm yarn vs npm)"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Only surface survey done — deeper structure not examined",
        ".env and .env.production exist with real values — operator to identify env names",
        "Package manager ambiguous: yarn.lock present but no packageManager field — operator to confirm",
        "Next.js 14.0.4 — older than other business apps (15.x/16.x)",
        "Boilerplate README — acceptance criteria need operator definition",
        "No test script defined — quality gap"
      ],
      "next_objective": "operator to direct deeper survey, confirm package manager, identify env vars, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Next.js 14 web app with AWS Amplify. Deeper purpose TBD from full survey. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-filamentdb",
  "idempotency_key": "f3-filamentdb-20260917"
}
```

---

## Contract 10 — ravensnestweb

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "dc3424bb-d7fb-4344-ac4f-f322b9a1e58f",
      "name": "ravensnestweb",
      "kind": "web-platform",
      "class": "business",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/ravensnestweb",
      "repository": {
        "remote": "https://github.com/RavensNestInc/ravensnestweb.git",
        "fetch": true,
        "push": true
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "typescript",
      "package_manager": "pnpm",
      "node_version": ">=20 <23",
      "pnpm_version": "10.17.1"
    },
    "commands": [
      {
        "id": "build",
        "program": "pnpm",
        "args": ["run", "build"],
        "working_dir": "./",,
        "timeout_seconds": 600,
        "verified": null,
        "notes": "pnpm -r build (mono-repo); not live-verified 2026-09-17"
      },
      {
        "id": "build:web",
        "program": "pnpm",
        "args": ["run", "build:web"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "pnpm --filter web build; not live-verified 2026-09-17"
      },
      {
        "id": "build:api",
        "program": "pnpm",
        "args": ["run", "build:api"],
        "working_dir": "./",,
        "timeout_seconds": 300,
        "verified": null,
        "notes": "pnpm --filter api build; not live-verified 2026-09-17"
      },
      {
        "id": "lint",
        "program": "pnpm",
        "args": ["run", "lint"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "pnpm -r lint; not live-verified 2026-09-17"
      },
      {
        "id": "lint:web",
        "program": "pnpm",
        "args": ["run", "lint:web"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "pnpm --filter web lint; not live-verified 2026-09-17"
      },
      {
        "id": "lint:api",
        "program": "pnpm",
        "args": ["run", "lint:api"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "pnpm --filter api lint; not live-verified 2026-09-17"
      },
      {
        "id": "dev:web",
        "program": "pnpm",
        "args": ["run", "dev:web"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "pnpm --filter web dev; not live-verified 2026-09-17"
      },
      {
        "id": "dev:api",
        "program": "pnpm",
        "args": ["run", "dev:api"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "pnpm --filter api dev; not live-verified 2026-09-17"
      },
      {
        "id": "start",
        "program": "pnpm",
        "args": ["run", "start"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "pnpm --filter api start; not live-verified 2026-09-17"
      },
      {
        "id": "db:migrate",
        "program": "pnpm",
        "args": ["run", "db:migrate"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "pnpm --filter api db:migrate; not live-verified 2026-09-17"
      },
      {
        "id": "db:seed",
        "program": "pnpm",
        "args": ["run", "db:seed"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "pnpm --filter api db:seed; not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "lint",
        "command": "lint",
        "status": "not_verified",
        "notes": "pnpm -r lint not run 2026-09-17"
      },
      {
        "id": "build",
        "command": "build",
        "status": "not_verified",
        "notes": "pnpm -r build not run 2026-09-17"
      },
      {
        "id": "test",
        "command": "test",
        "status": "none_defined",
        "notes": "no test script at root level; may exist in sub-packages (apps/web, apps/api). Gap recorded — deeper survey needed."
      }
    ],
    "artifacts": [
      {
        "id": "dist",
        "path": "dist/",
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
        "notes": "3D print business platform with Node.js API — likely needs outbound; operator to confirm"
      },
      "filesystem": {
        "write": true,
        "notes": "build writes to dist/ and sub-package outputs; pnpm-lock.yaml updates possible"
      },
      "secrets": {
        "env_names": [],
        "notes": "No .env files found in surface survey. Operator to identify from deeper survey (may exist in apps/api/ or apps/web/)."
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "clean",
      "notes": "working tree clean as of 2026-09-17"
    },
    "rollback": {
      "strategy": "git_revert",
      "notes": "standard git revert on main branch"
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/ravensnestweb/README.md",
        "/var/home/mythos/Eregion/ravensnestweb/package.json",
        "/var/home/mythos/Eregion/ravensnestweb/PROJECT_PLAN.md",
        "/var/home/mythos/Eregion/ravensnestweb/TASKS.md"
      ],
      "notes": "3D print business platform — customer/operator frontend (Vite+React) + Node.js+TypeScript API. pnpm mono-repo with apps/web and apps/api. Deeper per-package survey needed."
    },
    "acceptance": [
      "pnpm build passes (pnpm -r build)",
      "pnpm lint passes (pnpm -r lint)",
      "deeper survey completed (apps/web/package.json, apps/api/package.json inspected)",
      "env var identification from deeper survey",
      "ownership boundary confirmed (RavensNestInc remote — operator to confirm mutation authority)"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "Mono-repo with filtered commands — scope of 'build' vs 'build:web' vs 'build:api' needs operator clarification",
        "Deeper survey needed: apps/web/package.json, apps/api/package.json for full stack/command detail",
        "No .env files in surface survey — may exist in sub-packages",
        "Ownership boundary: RavensNestInc org (not dward1502) — operator to confirm mutation authority",
        "No test script at root level — may exist in sub-packages; gap recorded"
      ],
      "next_objective": "operator to direct deeper survey, confirm command scope, identify env vars, verify commands"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "3D print business platform — customer/operator frontend (Vite+React) + Node.js+TypeScript API. pnpm mono-repo. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-ravensnestweb",
  "idempotency_key": "f3-ravensnestweb-20260917"
}
```

---

## Contract 13 — relic-kiosk

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "634f4e2d-842a-4354-a917-9c29fcdf0d04",
      "name": "relic-kiosk",
      "kind": "prototype",
      "class": "internal",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/relic-kiosk",
      "repository": {
        "remote": null,
        "fetch": false,
        "push": false,
        "notes": "No git repository — internal prototype, not a standalone git project"
      }
    },
    "runtime": {
      "adapter": "node",
      "language": "javascript",
      "package_manager": "npm"
    },
    "commands": [
      {
        "id": "serve",
        "program": "npm",
        "args": ["run", "serve"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "npm run prepare:vendor && python3 -m http.server 8091 --bind 127.0.0.1; not live-verified 2026-09-17"
      },
      {
        "id": "test",
        "program": "npm",
        "args": ["run", "test"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "node --test test/*.test.mjs; not live-verified 2026-09-17"
      },
      {
        "id": "validate",
        "program": "npm",
        "args": ["run", "validate"],
        "working_dir": "./",,
        "timeout_seconds": 60,
        "verified": null,
        "notes": "node --check src/relic.js && node --check src/relicSceneState.js && npm run test; not live-verified 2026-09-17"
      },
      {
        "id": "prepare:vendor",
        "program": "npm",
        "args": ["run", "prepare:vendor"],
        "working_dir": "./",,
        "timeout_seconds": 30,
        "verified": null,
        "notes": "rsync three.js from Arda-HUD node_modules; requires ARDA_HUD_APP_DIR or HOME/Eregion/arda-hud; not live-verified 2026-09-17"
      },
      {
        "id": "deploy:citadel",
        "program": "npm",
        "args": ["run", "deploy:citadel"],
        "working_dir": "./",,
        "timeout_seconds": 120,
        "verified": null,
        "notes": "bash scripts/deploy_to_citadel.sh; not live-verified 2026-09-17"
      }
    ],
    "checks": [
      {
        "id": "test",
        "command": "test",
        "status": "not_verified",
        "notes": "node --test not run 2026-09-17"
      },
      {
        "id": "validate",
        "command": "validate",
        "status": "not_verified",
        "notes": "node --check not run 2026-09-17"
      }
    ],
    "artifacts": [
      {
        "id": "served-content",
        "path": "index.html",
        "type": "static-content",
        "verified": {
          "exists_at": "2026-09-17"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "Local-only serve on 127.0.0.1:8091; no external network exposure"
      },
      "filesystem": {
        "write": true,
        "notes": "prepare:vendor writes to src/vendor/; deploy:citadel may write to Citadel target"
      },
      "secrets": {
        "env_names": [],
        "notes": "No .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "n/a",
      "notes": "No git repository — dirty work concept does not apply. Manual cleanup only."
    },
    "rollback": {
      "strategy": "none-read-only",
      "notes": "No git — no reverts. Manual cleanup only."
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/relic-kiosk/README.md",
        "/var/home/mythos/Eregion/relic-kiosk/package.json"
      ],
      "notes": "RELIC = first CITADEL geometry-avatar prototype. References Arda scene adapter in test code. Intentionally separate from apps/citadel-companion."
    },
    "acceptance": [
      "npm run test passes (node --test)",
      "npm run validate passes (node --check)",
      "npm run serve serves on 127.0.0.1:8091",
      "prepare:vendor successfully vendors three.js from Arda-HUD",
      "operator to confirm prototype scope and Citadel deployment intent"
    ],
    "risks": {
      "current": [
        "Commands not live-verified 2026-09-17",
        "No git repository — not a standalone project; treated as internal prototype",
        "prepare:vendor depends on Arda-HUD node_modules/three/ — cross-project dependency; Arda-HUD must be available",
        "deploy:citadel script exists — deployment to Citadel hardware; operator to confirm scope",
        "No rollback strategy (no git) — manual recovery only"
      ],
      "next_objective": "operator to confirm prototype scope, verify commands, confirm Citadel deployment intent"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "RELIC = first CITADEL geometry-avatar prototype. Internal prototype, not for public release. References Arda scene adapter. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-relic-kiosk",
  "idempotency_key": "f3-relic-kiosk-20260917"
}
```

---

## Contract 14 — citadel-avatar

```json
{
  "contract": {
    "schema_version": "arda.project-contract.v1",
    "identity": {
      "project_id": "55f96128-4107-4744-9b39-9b1454e03230",
      "name": "citadel-avatar",
      "kind": "runtime-projection",
      "class": "internal",
      "owner": "mythos",
      "lifecycle": "active"
    },
    "workspace": {
      "root": "/var/home/mythos/Eregion/citadel-avatar",
      "repository": {
        "remote": null,
        "fetch": false,
        "push": false,
        "notes": "No git repository — internal runtime, not a standalone git project"
      }
    },
    "runtime": {
      "adapter": "none",
      "language": "javascript",
      "package_manager": "none",
      "notes": "Runtime projection surface — not a buildable npm project. Served locally on Pi."
    },
    "commands": [],
    "checks": [],
    "artifacts": [
      {
        "id": "index-html",
        "path": "index.html",
        "type": "static-content",
        "verified": {
          "exists_at": "2026-09-17"
        }
      },
      {
        "id": "scene-js",
        "path": "agent-kinetic-visualization.js",
        "type": "runtime-script",
        "verified": {
          "exists_at": "2026-09-17"
        }
      },
      {
        "id": "scene-js-optimized",
        "path": "scene-runtime-optimized.js",
        "type": "runtime-script",
        "verified": {
          "exists_at": "2026-09-17"
        }
      }
    ],
    "permissions": {
      "authority": "approval_required",
      "network": {
        "allow": false,
        "notes": "Local-only serve on Pi at 127.0.0.1:8080; no external network exposure"
      },
      "filesystem": {
        "write": false,
        "notes": "Runtime projection — no build output. Manual edits only."
      },
      "secrets": {
        "env_names": [],
        "notes": "No .env files; no secrets"
      }
    },
    "protected_paths": [],
    "dirty_worktree_policy": {
      "mode": "n/a",
      "notes": "No git repository — dirty work concept does not apply. Manual cleanup only."
    },
    "rollback": {
      "strategy": "none-read-only",
      "notes": "No git — no reverts. Manual recovery only."
    },
    "requirements": {
      "source": [
        "/var/home/mythos/Eregion/citadel-avatar/README.md",
        "/var/home/mythos/Eregion/citadel-avatar/package.json",
        "/var/home/mythos/Eregion/citadel-avatar/DEPLOYMENT_TO_PI5.md",
        "/var/home/mythos/Eregion/citadel-avatar/SCENE_STATE.md"
      ],
      "notes": "Canonical Pi-facing Three.js app for the round Waveshare display. Arda-adjacent projection surface. Deployment docs exist (DEPLOYMENT_TO_PI5.md, DEPLOYMENT_SUMMARY.md)."
    },
    "acceptance": [
      "Serves locally on Pi at http://127.0.0.1:8080",
      "Three.js scene renders correctly on round Waveshare display",
      "operator to confirm runtime projection scope and deployment status"
    ],
    "risks": {
      "current": [
        "No npm scripts — not a buildable project; runtime is served directly",
        "No git repository — not a standalone project; treated as internal runtime",
        "No declared checks — runtime verification is manual/operational",
        "No rollback strategy (no git) — manual recovery only",
        "Deployment status on Pi not verified 2026-09-17"
      ],
      "next_objective": "operator to confirm runtime projection scope, deployment status, and acceptance criteria"
    },
    "provenance": {
      "declared_by": "arda-f3-reconciliation",
      "declared_at": "2026-09-17T00:00:00Z",
      "purpose": "Canonical Pi-facing Three.js app for the round Waveshare display. Arda-adjacent projection surface. Not a buildable project — runtime is served directly. Commands not verified live 2026-09-17 — deferred to F3 attachment phase."
    }
  },
  "approval_id": "",
  "proposal_id": "f3-citadel-avatar",
  "idempotency_key": "f3-citadel-avatar-20260917"
}
```

---

## Cross-contract notes

### Command verification summary (14 projects, all deferred)

None of the 14 remaining projects had commands live-verified on 2026-09-17. Verification is deferred to the F3 attachment phase (operator-approved, idempotency receipted).

### Stale assessment corrections

- Arda-Council and Arda-Forge-Mind: `target/` deletion tracked in git — not source dirt, but indicates prior build-in-commit history
- Arda-HUD: 4 modified source files — active development, not stale
- filamentDB: PROJECTS_REPORT.md assessment likely stale (only surface-surveyed)
- ravensnestweb: deeper per-package assessment needed

### Dirty work summary

| Project | Dirty work | Policy |
|---|---|---|
| Arda-HUD | 4 modified source files | read_only; preserve until operator cleans up |
| All others | clean | n/a |

### What F3 does not do

- Does not attach these contracts to `data/workbench/projects.json` (attachment requires operator approval with idempotency receipts — the `approval_id` fields above are empty pending that)
- Does not read secret values from `.env` files (env_names are declared names only where known)
- Does not run build/test/lint commands against these projects during drafting (deferred to attachment phase)
- Does not resolve the dirty work in Arda-HUD (operator action)
- Does not perform deeper surveys for filamentDB and ravensnestweb (deferred to F3 execution phase)
- Does not invent env names, deployment targets, or acceptance criteria

### Source evidence

- `git -C <repo> status --short`, `git -C <repo> log --oneline -1`, `git -C <repo> remote -v` for each project
- `cat <repo>/Cargo.toml` or `cat <repo>/package.json` for stack and commands
- `cat <repo>/README.md` for purpose and documentation state
- `ls <repo>/.env*` for env file presence (values not read)
- Per-project file listings for top-level structure
