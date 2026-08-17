---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "archived"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: archived | reviewed: 2026-08-17

# Phase 2: Hermes Continuity and Surface Handoff Implementation Plan

> **For Hermes:** Use the `subagent-driven-development` skill to implement this plan task-by-task. Re-read current Hermes documentation before changing the plugin because upstream extension APIs evolve quickly.

**Goal:** Preserve one authenticated operator relationship and durable commitment context while a conversation moves between Hermes phone/messaging, desktop, HUD, and future room surfaces.

**Architecture:** Hermes remains session and conversation runtime. A repository-owned Hermes plugin emits bounded session/surface lifecycle events to Arda. Arda links those events to durable intent and Vairë context, returns references rather than copied transcripts, and records explicit handoffs. The existing command bridge is extended or paired, not replaced by the transient `hermes chat` subprocess adapter.

**Tech stack:** Hermes Gateway hooks/plugins/sessions, Python plugin tests, Rust Axum harness, Oromë interface types, Vairë memory policy, strict JSON/Serde contracts.

---

## Current source baseline

- `adapters/hermes-operator-bridge/` is a tracked Hermes plugin using `pre_gateway_dispatch` for authenticated Discord `arda ...` command interception and durable loopback delivery.
- It persists pending events with mode `0600`, retries boundedly, and treats Arda event identity as replay authority.
- Its current scope is commands, not general session continuity; it intercepts and skips prefixed messages.
- `crates/engine/src/adapters/hermes.rs` launches bounded transient Hermes CLI jobs. Keep that adapter only for explicitly bounded worker jobs; it is not the main operator-session architecture.
- Vairë is the memory authority; Hermes retains its native session store and surface delivery semantics.

## Handoff contract

Create `arda.surface-handoff.v1` with:

- stable operator identity reference, never a platform display name alone;
- Hermes session lineage id and current session id;
- source and destination surface/outpost ids;
- conversation topic/commitment references, not an unrestricted transcript dump;
- Vairë memory-scope references and data-domain classification;
- privacy class: `public_room | shared_room | private_room | personal_device`;
- consent state and requesting actor;
- `requested | prepared | accepted | active | declined | expired | failed`;
- issued/expiry/accepted times;
- replay/idempotency key;
- bounded reason/error;
- receipts linking the transition.

A handoff does not grant new tools, data scopes, or action authority.

## Task 1: Audit the live Hermes extension surface

**Files:**
- Modify after evidence: `adapters/hermes-operator-bridge/README.md`
- Create: `docs/operations/hermes-continuity-extension-audit.md`

**Steps:**
1. Check installed `hermes --version` and current authoritative docs for gateway hooks, plugin manifests, session-store access, event hooks, messaging source identity, and delivery APIs.
2. Inspect the installed plugin API source only as secondary evidence.
3. Record supported hook names, arguments, thread/topic semantics, session identifiers, and lifecycle constraints.
4. Identify whether continuity belongs in the existing plugin or a sibling `adapters/hermes-continuity-bridge/`; choose the existing plugin unless isolation is required by hook lifecycle or deployment.
5. Run existing bridge tests before edits: `python -m unittest -v adapters/hermes-operator-bridge/test_plugin.py`.
6. Commit the audit separately: `docs(hermes): record continuity extension surface`.

**Task 1 evidence (2026-08-17):** Completed in `9b5e74da`. The audit pins
Hermes Agent `v0.20.3`, the public hook/session/delivery contracts, thread/topic
semantics, gateway-injection limits, and the decision to extend the existing
bridge. Baseline bridge tests passed 3/3, `py_compile` and HADES passed, and
independent specification and quality reviews approved the result.

## Task 2: Define strict continuity and handoff types

**Files:**
- Create: `crates/spine/interface/arda-orome/src/surface_handoff.rs`
- Modify: `crates/spine/interface/arda-orome/src/lib.rs`
- Test: module unit tests and JSON fixtures under the crate's existing fixture convention

**Steps:**
1. Write failing strict-deserialization tests for valid request, unknown fields, missing operator/session identity, expired request, domain escalation, and altered replay key.
2. Implement versioned Serde types with bounded strings/collections and explicit enums.
3. Add pure transition validation; illegal state skips must fail.
4. Round-trip canonical fixtures.
5. Run `cargo test -p arda-orome surface_handoff` and Clippy.
6. Commit: `feat(orome): define surface handoff contract`.

**Task 2 evidence (2026-08-17):** Completed in `faae1543`. Oromë exposes the
strict `arda.surface-handoff.v1` contract with bounded identities/references,
domain and privacy policy, consent, expiry, replay protection, and legal state
transitions. RED failed on missing symbols; GREEN passed 8 focused tests and the
full crate suite. Strict default-feature Clippy passed; all-feature Clippy is
blocked only by pre-existing `arda-outpost-protocol` debt.

## Task 3: Add Arda continuity endpoints

**Files:**
- Modify: the existing operator HTTP transport under `crates/engine/src/harness/` after tracing `/v1/operator/messages`
- Create: focused transport module if the current file would become oversized
- Test: existing harness/router integration-test location

Endpoints:

- `POST /v1/continuity/events` — authenticated local plugin event intake;
- `POST /v1/handoffs` — create/request a transfer;
- `POST /v1/handoffs/{id}/accept` — operator/surface acceptance;
- `GET /v1/handoffs/{id}` — bounded state;
- `GET /v1/continuity/sessions/{lineage}` — references and safe summary only.

**Steps:**
1. Write failing tests for loopback enforcement, replay, unknown schema, expired event, unauthorized operator, cross-domain leakage, and valid transitions.
2. Reuse the current authenticated operator identity boundary; do not trust plugin-supplied `authenticated: true` without transport/source policy.
3. Persist idempotency and transition receipts before acknowledging success.
4. Return typed terminal/retryable errors.
5. Run focused engine tests and direct-consumer checks.
6. Commit: `feat(engine): accept governed continuity events`.

**Task 3 evidence (2026-08-17):** Implemented in `d752a7ce` and hardened in
`e54ff446`, `04785dae`, and `d12b3337`. The five loopback endpoints enforce the
configured canonical operator, strict schemas, expiry/domain policy, exact
replay, append-only receipts, atomic handoff snapshots, safe lineage reads, and
lineage-bound HUD actions. Review gaps for missing replay receipts, unknown
fields, invalid authentication methods, and cross-lineage handoffs were fixed.
Focused engine integration tests pass 3/3 and strict package Clippy passes.

## Task 4: Extend the Hermes plugin without intercepting normal conversation

**Files:**
- Modify: `adapters/hermes-operator-bridge/__init__.py` and `plugin.yaml`, or create the audited sibling plugin
- Modify: associated README
- Test: associated Python test module

**Steps:**
1. Write failing tests showing ordinary messages continue through Hermes while bounded session/surface metadata is emitted out-of-band.
2. Add lifecycle-event payload construction using the audited public hook context; never call undocumented private gateway methods if a public plugin API exists.
3. Preserve the current command path and durable retry semantics.
4. Persist only minimal continuity events; do not create a second transcript store.
5. Add tests for gateway restart, duplicate event, thread/topic transition, private/shared destination, Arda unavailable, and malformed source identity.
6. Run unit tests and `python -m py_compile`.
7. Commit: `feat(hermes): emit Arda continuity events`.

**Task 4 evidence (2026-08-17):** Completed in `809d8801` and identity-hardened
in `d12b3337`. Plugin `0.3.0` returns ordinary messages to Hermes while
asynchronously emitting transcript-free metadata, persists pending events with
mode `0600`, survives gateway restart, and treats duplicate delivery as
terminal. Platform user provenance remains separate from canonical
`operator:mythos`. Nine tests and `py_compile` pass; independent review passed.

## Task 5: Link continuity to Vairë without duplicating Hermes sessions

**Files:**
- Modify: the public Vairë service/schema modules identified by tracing current informant-event intake under `crates/spine/memory/arda-vaire/src/`
- Test: Vairë policy/retrieval tests

Store durable records for:

- session lineage reference;
- topic/commitment reference;
- active surface history;
- explicit handoff receipts;
- relevant memory scope links;
- privacy/domain policy;
- expiry for transient surface state.

**Steps:**
1. Write failing tests for personal/business partition, explicit overlap, expired surface state, absent transcript, and provenance retrieval.
2. Add a continuity record that references Hermes session ids rather than copying entire transcripts.
3. Ensure retrieval returns only scope-authorized context references.
4. Confirm Vairë never becomes a parallel live chat-session scheduler.
5. Run focused Vairë tests and direct-consumer check.
6. Commit: `feat(vaire): preserve surface continuity lineage`.

**Task 5 evidence (2026-08-17):** Completed in `34fcab31` and wired into accepted
handoffs in `3d72cebc`. Vairë stores reference-only lineage, commitment, surface,
receipt, memory-scope, domain, expiry, and provenance records with replay safety.
Personal/business partition, explicit overlap, expired surface removal, absent
transcript, and provenance retrieval tests pass. The full Vairë suite and strict
Clippy pass; independent specification review passed.

## Task 6: Provide the HUD/Launcher continuity projection

**Files:**
- Add a read-only projection to the existing HUD backend/source path after tracing current `ardaSource` contracts
- Modify: `apps/arda-hud/src/lib/ardaSource.ts` and types only when the backend contract exists
- Test: backend projection and frontend unavailable/stale/privacy tests

**Steps:**
1. Write failing tests for no active session, active phone session, prepared handoff, accepted desktop session, expired handoff, and private content withheld from a shared surface.
2. Expose session lineage, surface, privacy, freshness, and action ids—not raw transcript text by default.
3. Add a bounded `Continue here` request in the native HUD workstation; mutating handoff still requires explicit operator action.
4. Ensure the World View remains display-only.
5. Run focused backend tests and HUD tests/build.
6. Commit: `feat(hud): expose governed session handoff`.

**Task 6 evidence (2026-08-17):** Completed in `d6034f9b` and lineage-hardened in
`04785dae`. The backend publishes one safe continuity projection; the continuity
workstation shows source surface, privacy, freshness, handoff state, and the
explicit `Continue here` action without changing World View. Shared surfaces
withhold private references. HUD verification passed 143 test files/583 tests,
TypeScript, production Vite build, and lint with pre-existing warnings.

## Task 7: Phone-to-desktop continuity acceptance

**Run:**
1. Start from a real authenticated Hermes phone/messaging conversation.
2. Create a durable commitment and verify it is present in the native Hermes session.
3. Open Launcher/HUD through Phase 1.
4. Observe a safe active-session projection without leaking private transcript content.
5. Select `Continue here`.
6. Verify the same Hermes session lineage becomes active on desktop and relevant Vairë context is available.
7. Reply on desktop; verify the phone surface sees continuity through Hermes delivery semantics.
8. Test Arda restart, Hermes Gateway restart, duplicate event replay, expired handoff, and shared-room privacy denial.
9. Record exact session/handoff ids only in bounded private operational evidence; redact message content.

**Task 7 evidence (2026-08-17):** **Accepted.** A genuine phone-originated
Discord commitment entered the existing authenticated Hermes session. The live
bridge projected the same lineage with shared-room references withheld. The
operator selected `Continue here` in the native HUD; Arda recorded prepared then
accepted states, and Vairë retained the commitment reference and desktop surface
without transcript text. Hermes' native handoff watcher completed delivery back
to Discord with prior history loaded. Arda and Gateway restarts preserved state;
exact event replay returned `replayed=true`; an expired handoff failed with
`409 conflict`; all four Phase 1 units remained active. Exact identifiers remain
only in owner-only runtime stores and a mode-`0600` operator-local receipt. See the
[operations acceptance record](../operations/hermes-continuity-handoff-acceptance.md).

## Phase gate

Phase 2 is **proven** on the declared personal local profile. One genuine
phone-originated Hermes conversation continued through the native desktop/HUD
with the same authenticated lineage and durable commitment context after Arda
and Gateway restarts. Hermes completed delivery back to the phone surface. No
copied summary, synthetic fixture, or bounded `hermes chat` worker was used as
acceptance evidence. This implementation plan is archived; the operations
acceptance record remains the live evidence authority.
