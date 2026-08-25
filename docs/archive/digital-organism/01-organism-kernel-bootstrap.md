---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-21

# Stage 1 — Organism Kernel and Bootstrap Context

## Objective

Make organism identity, current objective, authority, topology references, and bounded context available to every participating worker without injecting the entire repository or relying on one long Hermes session.

## Architecture

Use existing `arda-core` identity/task/capability contracts, Vairë recall policy, and engine run lineage. Add the minimum organism manifest/context projection only where Stage 0 proves an owner gap. Hermes consumes the context through the existing operator bridge/plugin or a stable programmatic integration; it does not become the canonical context store.

## Work packets

### S1.1 — Define the organism manifest

Represent stable organism identity, mission, schema versions, canonical authorities, accepted transport families, privacy domains, and current root identity. Keep deployment-specific endpoints in node manifests/configuration.

**Likely files:** `crates/spine/governance/arda-core/src/`, `core/state/contract_registry.json`, contract tests.

### S1.2 — Define the bounded organism context capsule

The capsule must include only:

- organism and node identity;
- active objective and assigned role;
- acceptance conditions;
- allowed/prohibited capabilities and egress;
- relevant memory/evidence references;
- current peer/topology references;
- parent/child task/run/receipt lineage;
- unresolved failures and expiry;
- required return contract.

It must exclude raw unrelated memory, credentials, broad transcripts, and hidden reasoning.

**Task 4 verified state:** The manifest is `running`: Core owns `arda.organism-manifest.v1`, the canonical service loads `config/organism.toml`, and the authenticated root endpoint projects it read-only. The context contract is `root-composed`: Vairë owns `arda.organism-context.v1`, and its lineage, expiry, privacy/egress, role, capability, acceptance, and return-receipt validators compile into the root runtime. [Implementation and runtime evidence](../../../.hermes/evidence/digital-organism/organism-contracts.json)

**Task 5 verified state:** Vairë assembles bounded capsules exclusively from explicitly referenced canonical memory and persists deterministic use receipts. The existing Hermes adapter injects the structured capsule into each newly spawned CLI process and binds Hermes execution plus Oromë handoff receipts to its digest. The public harness restart fixture stops the first root, reopens Vairë and run state, suppresses exact replay, rejects a mismatched capsule, and gives the second worker explicit lineage to the first durable receipt. A live run used two separate real `hermes chat -Q` processes with no resume/continue/session token and completed the declared read-only check after restart. The installed root binary matches the verified release artifact. [Machine-readable implementation, live-worker, restart, and deployment evidence](../../../.hermes/evidence/digital-organism/context-bootstrap.json)

This proves the bounded Stage 1 context-bootstrap vertical. It does not claim arbitrary model migration, gateway-wide restart behavior, or continuity for workflows outside the tested public run route.

### S1.3 — Produce capsules through Vairë policy

Add a context assembly/query path that resolves current-only records, explicit corrections/revocations, personal/business/system scope, freshness, and use receipts. Do not create a new context database.

### S1.4 — Integrate Hermes bootstrap

Extend the existing `arda-operator-bridge` or use Hermes’ documented API/TUI gateway hooks to attach one bounded capsule at the start of an Arda-governed turn or worker run. Preserve prompt caching and Hermes session authority. Ordinary non-Arda conversation remains unchanged.

### S1.5 — Prove restart and model independence

Start an objective in one Hermes session/model, restart the gateway, resume through another supported surface/model, and recover the same objective, constraints, unresolved failure, and next action from Arda authorities.

## Tests and verification

- contract serialization and unknown-field rejection;
- scope/egress denial fixtures;
- stale/revoked memory exclusion;
- idempotent context-use receipt;
- plugin unit tests and fresh gateway load;
- real restart/reopen proof;
- no second memory/queue/transcript store.

## Exit gate

A fresh worker with no prior conversation can receive one capsule, explain its role and boundaries, complete a read-only task, return the required receipt, and another fresh worker can continue from that receipt after restart.
