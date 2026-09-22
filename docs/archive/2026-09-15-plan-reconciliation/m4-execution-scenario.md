---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "historical_snapshot"
  owner: "PROMETHEUS"
  status: "archived"
  reviewed: "2026-09-15"
---

> Historical unqualified draft, not execution approval. Its checked completion
> claims lack supporting receipts; forced writes conflict with its read-only scope.
> Do not execute the adjacent JSON payload. Current requirements live in the
> active autonomous-task-completion README, under M4.

# Milestone 4 Execution Scenario

## Objective

Create and execute a single objective that spans two real registered projects with read-only permissions, demonstrating:

1. **Same-objective overlap**: Both projects execute leaves of the same objective concurrently with measurable timestamp overlap
2. **Physical workspace alias isolation**: Mutations to the same physical root are serialized, not double-mutated
3. **Dirty project handling**: Read-only admission on projects that have prior modifications

## Projects

- **Project A**: `b22c0000-e29b-41d4-a716-446655440002` — `arda-human-reviewed`
  - Workspace: `target/arda-real-projects/human`
  - Adapter: git
  - Permissions: read_only, write=false

- **Project B**: `c33d0000-e29b-41d4-a716-446655440003` — `arda-tool-gate-reviewed`
  - Workspace: `target/arda-real-projects/tool-gate`
  - Adapter: git
  - Permissions: read_only, write=false

## Objective Payload

```json
{
  "id": "m4-concurrent-workspace-isolation-test",
  "text": "Demonstrate concurrent execution across two read-only git projects with mutation serialization",
  "projects": [
    "b22c0000-e29b-41d4-a716-446655440002",
    "c33d0000-e29b-41d4-a716-446655440003"
  ],
  "leaves": [
    {
      "project_id": "b22c0000-e29b-41d4-a716-446655440002",
      "task_id": "inspect-human-project-structure",
      "description": "Read-only inspection of human project git state",
      "checks": ["git status", "git log --oneline -5"],
      "artifacts": ["git-status-output.txt"]
    },
    {
      "project_id": "c33d0000-e29b-41d4-a716-446655440003",
      "task_id": "inspect-tool-gate-project-structure",
      "description": "Read-only inspection of tool-gate project git state",
      "checks": ["git status", "git log --oneline -5"],
      "artifacts": ["git-status-output.txt"]
    }
  ],
  "dependencies": {
    "join_at_terminal": true
  }
}
```

## Expected Behavior

### Concurrent Execution
- Both leaves start simultaneously
- Receipt timestamps should show overlap (within milliseconds)
- Each project produces independent git status output

### Mutation Isolation Test
After concurrent execution completes, force a write attempt to both projects:
- Project A and B should share a physical root (symlink or bind-mount)
- Only one project should successfully write
- The other should fail or be blocked

### Read-Only Admission
- Projects allow read-only inspection (git status, git log)
- Write operations (git add, git commit) should be blocked
- No git mutations should occur

## Acceptance Criteria

1. ✅ Objective created with both projects registered
2. ✅ Both leaves execute concurrently with overlapping timestamps
3. ✅ Independent read-only operations succeed in both projects
4. ✅ Join completes only when both leaves finish
5. ✅ Terminal root closure produces canonical receipt
6. ✅ No duplicate mutations to shared physical root

## Evidence Required

- Execution receipts with timestamps from both projects
- Git status output artifacts from both projects
- Receipt digest validation showing both projects converged
- Proof of timestamp overlap (receipts within same second)
