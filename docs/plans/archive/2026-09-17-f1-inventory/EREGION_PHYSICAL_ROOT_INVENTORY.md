---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "inventory"
  owner: "RUMIL"
  status: "approved"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "inventory", "fabric", "f1"]
---

> 🜏 Soterion: 📜 inventory | owner: RUMIL | status: approved | reviewed: 2026-09-17

# Eregion/ Physical Root Inventory — F1 deliverable

Deduplicated physical roots under `/var/home/mythos/Eregion/`, classified, with
explicit in-scope set and exclusion reasons. Compiled read-only from filesystem
and git state on 2026-09-17; no project source mutated.

## Scope of inventory

All immediate subdirectories of `/var/home/mythos/Eregion/` were inspected.
Roots are physical directories that are either a git repository root or a
non-repo project tree. Symlinks and nested subtrees were followed only where a
directory is a known worktree checkout.

## Deduplication result

No physical-root duplicates were found at Eregion top level: every directory is
a distinct physical root. The duplicates the plan warns about (worktree copies,
fixture shims counted as extra projects) live in `data/workbench/projects.json`
and `target/`, not here. This inventory intentionally excludes those runtime
registries and treats them as a separate F2 reconciliation target.

## Classification legend

- **core** — part of the Arda system itself
- **business** — existing business/product the operator needs to work on
- **internal** — internal project related to Arda or Citadel, not a client business
- **infra** — infrastructure, worktree, or ops checkout
- **experiment** — scratch or throwaway
- **excluded** — not a project; documented with reason

## Explicit in-scope set (approved)

### Core — 8 roots

Parted-out Arda crates/apps promoted for eventual public release.

| Root | Kind | Remote | Stack |
|---|---|---|---|
| `Arda-Agent-Loop-Contract/` | crate | `github.com/dward1502/Arda-Agent-Loop-Contract.git` | Rust |
| `Arda-Council/` | crate | `github.com/dward1502/Arda-Council.git` | Rust |
| `Arda-Forge-Mind/` | crate | `github.com/dward1502/Arda-Forge-Mind.git` | Rust |
| `Arda-HUD/` | app | `github.com/dward1502/arda-hud.git` | React/TS |
| `Arda-Human/` | crate | `github.com/dward1502/Arda-human.git` | Rust |
| `Arda-Service-Registry/` | crate | `github.com/dward1502/Arda-Service-Registry.git` | Rust |
| `Arda-Signal-Grid/` | crate | `github.com/dward1502/Arda-Signal-Grid.git` | Rust |
| `Arda-Tool-Gate/` | crate | `github.com/dward1502/Arda-tool-gate.git` | Rust — already in registry |

`Arda-Tool-Gate/` is already represented in the registry as
`arda-tool-gate-source-review` (read_only). The remaining 7 core roots are not
yet in the registry.

### Business — 6 roots

Existing businesses the operator needs to work on.

| Root | Kind | Remote | Stack | Monetization priority (2026-05-13 doc) |
|---|---|---|---|---|
|| `CoverCoINC/` | web app | `github.com/dward1502/CoverCoINC.git` | Next.js 15.5.15, React 19.1, Supabase, Resend | Fastest win (#1) |
|| `wgtt/` | web app | `github.com/dward1502/wgtt.git` | Next.js 16.0.5, React 19.2, Supabase, WeTravel/Travefy/Crossbar | Launch readiness (#2) |
|| `skylightpros/` | web app | `github.com/dward1502/skylightpros.git` | Next.js 15.5.4, React 19.1, Prisma, Supabase, pnpm | Highest value, most risk (#3) |
|| `ravensnestweb/` | web app | `github.com/RavensNestInc/ravensnestweb.git` | Vite+React web, Node.js+TS API | RavensNest 3D-print business platform |
|| `wakita/` | web app (Express) | `github.com/RavensNestInc/wakita.git` (original: `github.com/shun1000/wakitaRE.git`) | Express.js 2.0.0 + TypeScript, Azure Static Web Apps exportable (`dist-static/`), MongoDB (live mode) / static (Azure mode), Jest tests + coverage present | Real estate website — Express-hosted or Azure-static; ownership boundary: RavensNestInc remote (different org than dward1502), authors Daniel Ward + Shun Wakita, Apache-2.0; included by explicit operator direction |
|| `filamentDB/` | web app | `github.com/dward1502/filamentDB.git` | JS (package.json + README.md + (deeper structure TBD in F2)) | Included by explicit operator direction; deeper structure to be surveyed in F2 |

`skylightpros`, `wgtt`, and `CoverCoINC` are covered by the
`PROJECT_MONETIZATION_STATUS_2026-05-13.md` context document. `ravensnestweb`,
`wakita`, and `filamentDB` are additional existing businesses added by operator
direction; `ravensnestweb` and `wakita` are RavensNestInc-owned (different GitHub
org remote) and are included here by explicit operator approval.

### Internal — 2 roots

Internal projects related to Arda/Citadel, not client businesses.

| Root | Kind | Stack | Relationship |
|---|---|---|---|
| `relic-kiosk/` | prototype | JS (no git — stub) | RELIC = first CITADEL geometry-avatar prototype; references Arda scene adapter in test code; intentionally separate from `apps/citadel-companion` |
| `citadel-avatar/` | runtime | JS (no git — stub) | Canonical Pi-facing Three.js app for the round Waveshare display; serves locally on Pi; Arda-adjacent projection surface |

Both are non-repo trees (no `.git`); they are Amplify/stub surfaces or Pi-local
runtimes, not standalone git projects. They are in-scope as internal Arda/Citadel
work but are not treated as promotion candidates.

### Infra — 7 live worktrees

Active Arda worktrees under `Arda-worktrees/`, confirmed live via `git worktree
list` from the `Arda/` checkout. These are not separate promoted projects; they
are operation/pruning-stage branches of the Arda repo.

| Worktree | Branch | Purpose (branch name read) |
|---|---|---|
| `Arda-worktrees/adapter-artifact-pinning` | `stage5/adapter-artifact-pinning` | |
| `Arda-worktrees/appimage-relr` | `stage5/appimage-relr` | |
| `Arda-worktrees/glib-0185-backport` | `stage5/glib-0185-backport` | |
| `Arda-worktrees/personal-operations` | `optional/personal-operations` | |
| `Arda-worktrees/stage5-final-efd118b5` | detached `efd118b5` | |
| `Arda-worktrees/stage5-rc1` | `release/stage5-rc1` | |
| `Arda-worktrees/tauri-gtk-security` | `stage5/tauri-gtk-security` | |

Also present in `Arda/` git worktree list (all marked prunable):
`/tmp/arda-acceptance-exact-tree`, `/tmp/arda-closeout-tree`,
`/tmp/arda-main-integration`, `/tmp/arda-objective-review-*`,
`/tmp/arda-objective-staged-tree`, `/tmp/arda-plan-closeout/*` (multiple review
and dirty-worktree variants), `/var/home/mythos/.cache/t7-dirty-worktree-v4-*`.
These /tmp and .cache trees are prunable and are treated as transient
acceptance/fixture artifacts, not promoted roots.

## Excluded — with reasons

| Root / path | Reason |
|---|---|
| `Arda-worktrees/` subdirs beyond the 7 live ones | No additional live subtrees; the 7 above are the complete live set |
| `/tmp/arda-*` and `~/.cache/t7-*` worktrees | Transient acceptance/fixture worktrees marked prunable; not promoted roots |
|| `samsy-ninja-test/` | Throwaway test repo (README-only); experiment/scratch |
| `ventures/` | Empty directory; not a project |
| `templates/` | Empty directory; not a project |
| `audits/` | Internal audit artifacts; not a project root |
| `Arda/` (the daemon checkout itself) | This is the workbench root where the daemon runs; cross-project fabric attaches to the *parted-out* crates, not to the monorepo checkout as an extra project. The monorepo remains the system under test, not an additional fabric entry. |

## Cross-reference to existing registry

The existing `data/workbench/projects.json` (20 entries) was reviewed against this
inventory during F1 read-only inspection. Its entries that survive as distinct
in-scope physical roots:

- `arda-tool-gate-source-review` → `Arda-Tool-Gate/` ✓ (read_only, already attached)
- M4 acceptance fixtures (v2–v5 a/b), autonomous acceptance a/b, stage-4 research,
  living-mesh, installed-retained-local-acceptance, human-reviewed, tool-gate-
  reviewed, provider-route-audit → these are either subdirectory shims inside
  `Arda/`, generated fixture trees under `target/`, or detached worktree copies.
  They are **not** counted as additional physical roots by this inventory. F2 will
  reconcile each registry identity against the real roots above and retire/merge
  proof-only entries.

## Decisions recorded during F1

1. `wakita/` and `filamentDB/` — operator confirmed include; both added to the in-scope business set above.
2. Internal relic/citadel — confirm they stay internal (current classification) or
   get promoted to business later.
3. Core 7 not yet in registry, wakita, and filamentDB — operator approval to draft contracts for them in
   F2 (the plan requires explicit approval/idempotency receipts, not discovery-as-
   approval).

## Source evidence

- Filesystem inspection: `ls`, `git -C <d> rev-parse --show-toplevel`,
  `git -C <d> remote -v`, manifest probes (`Cargo.toml`, `package.json`,
  `amplify.yml`, `components.json`, `pnpm-lock.yaml`, `README.md`) per root.
- `git -C Arda worktree list` for live worktree confirmation.
- `PROJECT_MONETIZATION_STATUS_2026-05-13.md` for business readiness context.
- `relic-kiosk/README.md` and `citadel-avatar/README.md` for internal project
  purpose.
- `ravensnestweb/README.md` for business-platform classification.
