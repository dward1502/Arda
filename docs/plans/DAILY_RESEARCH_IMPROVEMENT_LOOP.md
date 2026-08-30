---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "WARDEN"
  status: "active"
  reviewed: "2026-08-25"
  tags: ["research", "daily", "improvement", "internet", "continuation"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: WARDEN | status: active | reviewed: 2026-08-25

# Daily Research and Improvement Loop

## Outcome

Every day Arda examines active objectives, connected projects, failed work, stale plans, dependencies, runtime health, and relevant current external developments. It turns useful findings into bounded work, implements eligible improvements, verifies and reviews them, and carries unfinished outcomes forward. It stays quiet when evidence supports no change.

This is not a daily news digest and not an agent that continually invents features. Research is driven by operator goals and verified system needs.

## Verified starting point

- Repository systemd units define daily Warden internet research and repository survey.
- Those timers are not installed in the active user unit set.
- The survey unit points to `%h/Arda`, not canonical `%h/Eregion/Arda`.
- Knowledge triage can classify evidence and intentionally blocks queue mutation without a governed pivot.
- The installed read-only autopilot can report, but its current preflight is stale/missing evidence and it does not complete implementation work.
- No installed loop was found that connects research through task creation, execution, verification, review, and later continuation.

## Daily cycle

### 1. Internal scan

Use connected project contracts and Soterion YAML metadata to locate relevant files quickly, then read canonical task, plan, receipt, project, source, and runtime authorities to gather:

- active objectives and acceptance gaps;
- incomplete/failed/deferred queue work;
- stale plans and status claims;
- recent runtime failures and degraded providers;
- dependency, security, build, test, and deployment signals;
- explicit operator interests and previously discussed desired capabilities.

### 2. Research-question generation

Generate a bounded set of questions tied to those needs, such as:

- Is there a safer or simpler current implementation pattern for a failing subsystem?
- Has an upstream dependency fixed a workaround we carry?
- Are new local models better suited to a declared worker/reviewer role on available hardware?
- Is a connected project's product/market/operational assumption stale?
- Which unfinished plan still matters to the operator's original outcome?

Each question names its originating project/objective and the decision it could change.

### 3. External retrieval

Use current internet sources, upstream documentation, releases, advisories, papers, and reputable implementation evidence. Record URL, title, retrieval time, claim, applicability, uncertainty, and contradiction. Prefer primary sources. Never convert search snippets into implementation authority.

### 4. Synthesis and promotion

Compare findings with live local evidence. Produce one of:

- `no_change` with reason;
- `knowledge_update` with cited durable record;
- `task_candidate` with outcome and acceptance criteria;
- `urgent_review` for security, data loss, or material external change;
- `blocked_research` when sources are inadequate.

Policy-safe local candidates may enter the canonical queue automatically. Consequential changes require operator approval. Duplicate and superseded findings do not create new work.

### 5. Execute and continue

The autonomous completion loop schedules and executes eligible candidates. Research does not mark its own recommendation complete. Verification and review determine whether the change closes. Incomplete work retains lineage and resumes on later ticks.

### 6. Daily operator brief

Show only material outcomes:

- verified improvements completed;
- ongoing work and current continuation decision;
- decisions genuinely needed;
- important rejected/no-change findings;
- failures that exhausted bounds.

Do not send recurring “all clear” noise unless requested.

## Cadence

- **Continuous/minute:** resume eligible task nodes and react to failures.
- **Daily:** bounded research, project drift scan, dependency/release/security review, and task promotion.
- **Weekly:** deeper Rúmil project-purpose/plan/source audit and cross-project dependency review.
- **Monthly:** provider/model capability and cost review, dormant-project review, and plan-authority cleanup.

Cadence wakes the same canonical loop; it does not create separate queues.

## Implementation sequence

### D1 — Repair scheduler installation

Correct canonical paths, package/install Warden units through the supported installer, verify timer/service ownership, and expose last successful useful cycle—not merely last process exit.

### D2 — Research topic authority

Replace a static disconnected topic list with generated questions from active objectives and connected project evidence while preserving operator-pinned topics and exclusions.

### D3 — Retrieval receipts

Store source-grounded findings with freshness, citations, content digests where lawful, contradictions, and relevance to a named decision.

### D4 — Governed task pivot

Implement the explicit bridge from triaged finding to canonical outcome/task. Enforce dedupe, authority, project contract, budget, and acceptance coverage.

### D5 — Improvement execution

Run safe candidates through full Workbench verification/review. Support code, docs, config, dependency, test, deployment-plan, and operational-procedure improvements according to project authority.

### D6 — Feedback and learning

Track whether each promoted finding produced a useful accepted change, was rejected, or caused rework. Adjust topic selection and source weighting while preserving operator corrections and provenance.

## Acceptance window

Run seven consecutive daily cycles. Require:

- all cycles survive restart and retain lineage;
- every external claim has a source and freshness time;
- at least one finding correctly results in no change;
- at least one finding becomes a verified improvement in a real connected project;
- at least one unsafe or low-value idea is rejected by policy/review;
- unfinished work resumes without a new operator task;
- the brief accurately distinguishes implemented, in progress, blocked, and proposed;
- the operator judges that the loop reduces—not increases—management burden.

## Done

This plan is complete when daily research reliably changes real outcomes where justified and remains quiet where not. Timer activation, report generation, or a list of links is insufficient.
