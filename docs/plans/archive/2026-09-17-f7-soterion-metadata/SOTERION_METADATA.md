---
soterion:
  sigil: "SCROLL"
  glyph: "[scroll]"
  code_point: "U+1F4DC"
  role: "metadata"
  owner: "RUMIL"
  status: "completed"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "fabric", "f7", "metadata"]
---

> Soterion: [scroll] | metadata | owner: RUMIL | status: completed | reviewed: 2026-09-17

# F7 -- Soterion Metadata

## Context

Soterion metadata improves discovery without implying permission to change. The
metadata is used to find owned files, then the actual sources are read. Labels
and metadata do not prove runtime, freshness, or approval.

## Metadata Application Principles

### 1. Metadata is for discovery

Metadata helps locate and identify files. It does not replace reading the actual
source. The discovery flow is:

1. Use metadata to find files of interest
2. Identify the file's ownership and purpose from metadata
3. Read the actual source
4. Verify the file is what the metadata claims

### 2. Metadata finds owned files, then sources are read

Metadata answers "what is this file and who owns it?" Source reading answers
"what does this file actually contain?" These are separate steps.

### 3. Labels do not prove runtime, freshness, or approval

A label like status: active or reviewed: 2026-09-17 does not prove the
project is currently running, that the code is fresh, or that the project has
been approved for work. These must be verified separately.

### 4. Preserve language-valid metadata conventions in code

Metadata in code should follow the conventions of the language. For example, Rust
crates use Cargo.toml for dependency and project metadata; TypeScript projects
use package.json. Soterion metadata in plan files should be consistent with
these conventions, not override them.

## Metadata Types

### 1. Project identity metadata

- project_id: unique identifier for the project
- name: human-readable project name
- kind: project type (rust, web-app, git-source, git-documentation, acceptance, fixture)
- owner: project owner
- lifecycle: project lifecycle state (active, etc.)

### 2. Source location metadata

- root: project root directory
- remote: remote repository URL
- language: primary language
- adapter: execution adapter

### 3. Build and run metadata

- commands: build, test, run commands
- artifacts: build artifacts and their paths
- checks: verification checks

### 4. Authority metadata

- authority: permission level (read_only, approval_required, etc.)
- network.allow: whether network access is allowed
- filesystem.write: whether filesystem writes are allowed
- secrets.env_names: declared secret environment variable names

### 5. Soterion metadata in plan files

Plan files use YAML frontmatter for Soterion metadata:

---
soterion:
  sigil: "SCROLL"
  glyph: "[scroll]"
  code_point: "U+1F4DC"
  role: "plan-role"
  owner: "PLAN_OWNER"
  status: "active"
  reviewed: "YYYY-MM-DD"
  tags: ["tag1", "tag2"]
---

This metadata identifies the plan's role, owner, status, and review date. It does
not prove the plan is approved or executable.

## Metadata Usage Flow

### Discovery phase

1. Query metadata to find projects of interest
2. Filter by role, owner, status, tags, or other metadata fields
3. Identify candidate projects

### Identification phase

1. For each candidate, read the metadata to understand ownership and purpose
2. Determine if the project is production or proof/demo
3. Identify the project's root, commands, and authority

### Reading phase

1. Read the actual source code at the project root
2. Verify the source matches the metadata claims
3. Do not rely on metadata alone to understand the code

### Verification phase

1. Verify the project is actually approved for work (separate from metadata)
2. Verify the project's commands work as claimed (run them)
3. Verify the project's freshness and runtime status (check separately)

## Metadata Limitations

- Metadata is not approval: a project entry in the registry does not mean it is approved for work. Approval is separate.
- Metadata is not runtime proof: a project marked status: active is not necessarily running. Runtime state is separate.
- Metadata is not freshness proof: a project marked reviewed: 2026-09-17 may have stale code. Freshness must be checked.
- Metadata can become stale: metadata is updated when projects change, but it can lag behind reality. Always verify against actual source.

## Metadata Storage

Metadata is stored in:

- Plan files: YAML frontmatter in docs/plans/ files
- Registry: data/workbench/projects.json for project contracts
- Project manifests: Cargo.toml, package.json, etc. for project-native metadata

These sources should be consistent with each other, but they serve different
purposes. Plan file metadata identifies the plan's role; registry metadata
identifies the project's contract; manifest metadata identifies the project's
native package information.
