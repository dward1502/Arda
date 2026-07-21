# Arda Varda Role Analysis
`docs/arda-varda-role-analysis.md`

## Observed current role

After the Batch 1–5 cleanup passes, `arda-varda` is best understood as:

- **ATHENA runtime shell.** The crate doc comment says it plainly: “Knowledge ingest,
  synthesis, and learning loop agent.” Its `AthenaAgent` binds:
  - capability routing (`ingest`, `query`, `deep`, `deep_analyze`, `research`,
    `code`, `decision`, `general`)
  - system prompt construction per capability
  - single LLM call surface (`ChatRequest` → `LlmProvider.chat()`)
  - task lifecycle (`task.start_execution()` → `task.complete(...)`)
- **Ingest pipeline owner.** The `src/ingest/` tree is the heart of Varda today:
  crawlers, deep extractors, scholarly/GitHub sources, observerability hooks,
  policy/routing layer, uncertainty sampling, remediation, metrics, source
  classification, layout/index writes. These are real production paths, not
  stubs.
- **Legacy human/learning/transport registry.** Varda still exposes
  `pub use arda_human::{...}`, `pub use arda_learning::{...}`.
  Those modules no longer live in this crate, but the re-exports preserve
  backward compatibility while the extraction completes.
- **Test-blocked compile surface.** The in-source `#[cfg(test)]` block was
  archived because it imported `arda_plutus`, which is absent from the workspace
  today. Varda is compile-safe in library mode, but test-local execution still
  depends on restoring or deleting those test paths.

## Boundary vs other crates

| Surface | Still in `arda-varda` | Moved out |
|---|---|---|
| Agent execution / LLM routing | `AthenaAgent` | — |
| Ingest crawl/extract/scholarly | `src/ingest/*` | — |
| Human document ingestion | `pub use arda_human` | `arda-human` owns canonical implementation |
| Knowledge delta emission | `pub use arda_learning` | `arda-learning` owns canonical implementation |
| Transport config / `expand_home` | re-exported | `arda-transport` owns canonical implementation |
| Service registry concepts | re-exported | `arda-service-registry` / `arda-core` own implementations |

## Future possible roles

1. **Thin orchestrator facade.** Varda can shrink to a small `AthenaAgent`
   runtime that composes `arda-human`, `arda-learning`, `arda-transport`,
   `arda-service-registry`, and a future `arda-ingest` crate. The current
   `src/ingest/*` tree would itself become a new crate, leaving Varda as
   capability routing plus LLM execution only.
2. **EVM/MCP execution node.** If Varda is intended to be Hermes’ code/research
   execution backend, it should own explicit MCP/boardroom tool surfaces
   instead of generic “general” capability text.
3. **Learning-loop owner.** If Varda is meant to close the deliberation
   feedback loop, `arda-learning` should stay thin and Varda should drive
   `KnowledgeDelta` emission from executed `Task` results.
4. **Deprecated alias for a new crate name.** The `arda-varda` name is already
   being phased away in docs. A future rename to `arda-athena` or a generic
   `arda-executor` would reduce name-coupling if the executor pattern repeats
   for Hades/Prometheus agents.

## What I would add

- A dedicated `arda-ingest` crate. The `src/ingest/*` module tree is large
  enough to warrant its own package; it has its own config, metrics,
  policy, and route logic today.
- Explicit `MCP`/tool-call capability types in Varda so the agent can invoke
  boardroom/Hermes tools without depending on `OrACLE` stringly-typed routing.
- A small executor contract trait so Varda-style agents can be hosted inside
  `manwe`/`arda-engine` without coupling to `AthenaAgent`.

## What I would remove

- The `arda_plutus` test dependency entirely from this subtree unless
  `arda-plutus` is restored as a sibling crate. The three ad-hoc `PlutusService`
  references in archived test code should stay archived, not live in `src/`.
- Legacy `ArdaError` import that is unused in `lib.rs` (`arda_core::error::ArdaError`).
- The bare `model_used` binding; either log it or drop it. It is dead weight
  and signals incomplete observability in the LLM path.
- Partial “Annunimas → Arda” migration artifacts in INDEX/docs when the rename
  is finished globally.

## Suggested roadmap

1. Extract `src/ingest/*` → `arda-ingest` crate, mirroring the `arda-human`
   / `arda-learning` extraction pattern.
2. Convert `AthenaAgent` into a generic `ArdaAgent<Capabilities,Provider>`
   so the same runtime can host Hades/Prometheus executors.
3. Move LLM routing/prompt-building into a trait so Varda can stay
   provider-agnostic.
4. Resolve `arda_plutus` either by:
   - folding the plutus concepts into `arda-economics`/`arda-core`, or
   - deleting the archived test code and the referencing source paths.
5. Finalize naming/alias strategy: either own `arda-varda` or rename to
   `arda-athena` and remove ambiguity in docs/tests.
