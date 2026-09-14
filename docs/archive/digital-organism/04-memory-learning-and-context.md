---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "VAIRE"
  status: "complete"
  reviewed: "2026-08-22"
---

> 🜏 Soterion: 📜 implementation_plan | owner: VAIRE | status: complete | reviewed: 2026-08-22

# Stage 4 — Organism Memory, Learning, and Context

## Objective

Turn distributed work into governed collective continuity that later nodes can retrieve and behaviorally use without broadcasting private memory or treating correlation as authority.

## Boundary

Vairë remains the one canonical memory abstraction/store. Varda owns evidence/provenance and research evaluation. Hermes profile memory is scoped conversational working memory. Nodes receive references and bounded capsules, not ambient access to all organism memory.

## Work packets

### S4.1 — Persist node/work/placement provenance

Memory and outcome records retain organism, node, worker, role, objective, attempt, route, evidence, correction/revocation, and receipt lineage. Sensitive content uses references/redacted projections.

### S4.2 — Enforce domain and consumer policy

Keep personal, business, and system domains distinct from subsystem scopes. Personal/health context requires authenticated operator-local or explicitly attested consumers. Cross-node broadcast of raw personal memory is prohibited.

### S4.3 — Record memory use

For each capsule/retrieval, record whether a node used, deferred, rejected, superseded, or could not retrieve the item. Storage and retrieval alone are not learning proof.

### S4.4 — Close the outcome learning loop

Compare acceptance conditions with terminal receipts. Produce bounded learning deltas for route quality, capability fit, recurring failure, recovery effectiveness, and context usefulness. Varda evaluates evidence; Vairë preserves approved safe-local deltas; Arandur may propose adaptations but cannot self-authorize them.

### S4.5 — Prove cross-node continuation

Node A begins a task and writes an intermediate receipt. Node B receives only a bounded context capsule, continues accurately, and returns an outcome whose lineage proves what it used. Corrected/revoked memory must not reappear as current truth.

## Verification

- scope/consumer access matrix;
- correction, supersession, revoke, decay, and chain queries;
- memory-use receipts;
- no duplicate canonical store;
- restart continuation;
- no transcript or secret replication;
- workspace-wide compile when shared memory contracts change.

## Exit gate

A fresh node continues another node’s work after restart using a bounded capsule, records exactly which memory/evidence influenced it, honors a correction/revocation, and improves a later placement or plan only through an approved learning receipt.

## Implemented

- Vairë now appends digest-bound context-outcome receipts distinguishing selected context from memory that actually influenced work. Exact restart replay returns the existing receipt; conflicting replay fails.
- Varda evaluates terminal receipts against named acceptance conditions and permits only bounded, fully evidenced safe-local placement learning.
- The engine persists approved placement-learning receipts under the workbench root with a cross-process ledger lock and stable replay identity. Adaptive placement consumes only those Varda/Vairë-linked receipts and explains every applied score adjustment in its placement receipt.
- No second memory store was introduced. Canonical context, scope policy, correction/revocation, and bounded capsule assembly remain owned by Vairë.

## Evidence

- `.hermes/evidence/digital-organism/stage4-memory-learning-restart-receipt.json` records the core-to-Beelink reassignment, exact context influence, Varda evaluation, Vairë learning provenance, placement adjustment, fresh-service restart, and duplicate counts.
- `cargo test -p arda-engine --test stage4_memory_learning_context`: 2 passed.
- `cargo test -p arda-engine --test context_bootstrap_restart`: 1 passed, proving a fresh worker continues after root restart without conversation history.
- `cargo test -p arda-vaire --test context_capsule`: 3 passed, including restart-stable capsule identity and revoked-memory rejection.
- Focused correction-chain and idempotent revoke tests passed; `cargo test -p arda-engine --test knowledge_delta_loop`: 3 passed.
- Stage 3 adaptive-placement regression remained green (3 passed, 1 live-only ignored), and `cargo build -p arda-engine` passed.
