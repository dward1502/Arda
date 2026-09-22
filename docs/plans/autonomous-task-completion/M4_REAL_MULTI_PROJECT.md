---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-22"
  tags: [m4, multi-project, concurrency, isolation]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-22

# M4 — Real Multi-Project Execution

## Prerequisites
- M3 complete: live critic rejection and revision proven
- Two operator-reviewed real project contracts from the fabric owner (not target/arda-real-projects/ proof roots)
- Daemon running on port 7878, 37 projects loaded

## Step-by-step

### M4.1 — Bind real project contracts
1. Register two real project contracts with exact roots, authority, acceptance/check commands, artifacts, budgets, and provider constraints
2. Bind each leaf to its exact project, workspace root, and authority class
3. Verify project contracts are operator-reviewed (not synthetic)

### M4.2 — Concurrent independent leaves
1. Create an objective with dependent leaves in both projects
2. Run safe independent leaves with measured overlapping execution intervals (start/end evidence, not receipt timestamps merely within one second)
3. Each project produces a useful human-visible result
4. Each project passes its declared checks

### M4.3 — Dirty work preservation and alias isolation
1. Preserve dirty operator work — fresh mutation on a dirty root must be blocked
2. Prove physical-root aliases cannot double-mutate
3. Use separately authorized disposable isolation fixtures; never force writes into read-only real projects

### M4.4 — Validated join
1. Join only after both project contracts pass
2. Prove one receipt-backed root close
3. Verify no completed-sibling replay after restart

## Evidence required
- Project contracts: exact roots, authority, checks, budgets, provider constraints
- Overlap: measured start/end timestamps proving concurrent execution
- Isolation: dirty root mutation blocked, alias double-mutation prevented
- Join: both receipts validated, root close with receipt chain
- No replay: post-restart no duplicate terminal records

## Exit gate
- One useful result spans two real projects with overlap, isolated authority, per-project checks, and one joined close
- Single-host concurrency acceptable; remote-device execution has separate ownership