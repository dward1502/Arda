# Athena — Parse / Ingest / Digest Design Assessment

Grounded review of the arda knowledge agent's architecture and
retrieval quality. Findings are sourced from the actual
`crates/old-arda/arda-varda/src` implementation.

> Status note: `arda-core` (the crate this code imports for
> `Agent`/`LlmProvider`/governance types) has been deleted from disk and
> is being replaced by `crates/spine/governance/arda-core`. Several
> governance/plutus/mnemosyne dependencies are still STUBS. See the
> extraction plan for the migration path. This doc assesses design, not
> the in-flight migration.

---

## 1. How it actually works

### Parse / Ingest — `ingest.rs:666` `ingest()`
- Local, deterministic, cheap. **No LLM call on the ingest path.**
- `build_shallow_analysis()` produces metadata only: `title`, `summary`,
  `language`, `key_dependencies`, `relevance_tags`, `license`,
  `components_available`, `reuse_potential`, plus GitHub / arXiv metadata.
- Append-only JSONL: one "book" file per source (`<source_id>.jsonl`) plus
  a global `digest.jsonl`.
- Idempotent dedup by `source_id` — re-ingest marks `shallow_existing`
  instead of duplicating.
- Concurrency-gated via `try_run_bounded("athena_ingest", ...)`.
- Clean event/interceptor pipeline (`before` / `after` / `DigestEvent`).

### Digest — `deep_analyze()` (the LLM stage, NOT on ingest path)
- **Shallow = metadata only** (no real understanding).
- **Deep = queued, provider-routed LLM call** via
  `run_extraction()` → `extraction::extract_knowledge()`, which fills
  `concepts` / `patterns` / `novel_ideas` / `applicability` /
  `integration_hooks` as free-text fields.
- Layered governance scoring on top: `triad_validate`, `resonance`,
  `joule`, `love_equation` → `confidence_self_report`.

### Query / Retrieve — `index.rs` + `query.rs`
- Lazy in-memory index: rebuilt on first stale read (books-dir mtime +
  300s TTL). Not persisted, not shared across processes.
- Scoring = weighted substring `.contains()` (see `index.rs`
  `score_entry`):
  - title 2.5, concepts 2.0, novel_ideas 1.8, summary 1.5,
    applicability 1.5, patterns 1.4, integration_hooks 1.2,
    deep_summary 1.2, relevance_tags 1.0, comparable_systems 0.8
  - + small deep-confidence bonus when text score > 0.
- No embeddings, no BM25/TF-IDF, no stemming, no synonym handling.

---

## 2. Verdict

- **Ingest — GOOD, keep it.** Append-only books, idempotent dedup,
  shallow (cheap/local) vs deep (LLM/queued) split is the right shape.
  Interceptor/event design is clean; concurrency gate is correct.
- **Digest — PARTIAL.** JSONL storage of deep extraction is fine for
  durability, but the result is free text the query layer can't truly
  use. The governance confidence layer is **currently low-signal**: the
  `arda-governance` / `arda-economics` / `arda-vaire`
  crates it scores against are **stubs**, so `confidence_self_report` is
  decorative until those gates are real.
- **Query — THE WEAK LINK.** Substring scorer has real problems:
  synonyms never match; substring false positives
  (`rust`⊂`trust`, `car`⊂`carbon`); score is uncalibrated additive so
  long docs win by token count; no semantic/vector retrieval; index is
  in-memory + lazy so it doesn't scale and isn't shared.

**Bottom line:** architecture is sound; retrieval is naive substring
matching that mis-ranks and misses. Solid foundation, not a finished
knowledge engine.

---

## 3. Failure modes (concrete)

1. **Substring scoring is lossy and noisy.**
   False positives (`rust`→`trust`, `car`→`carbon`); synonyms never
   connect (`auth` ≠ `login` ≠ `authentication` unless the exact token
   is in stored text); raw additive score means long shallow books beat
   short on-point deep books.
2. **Confidence layer is hollow.** `deep_analyze()` feeds triad/resonance/
   joule/love — all from stub crates. Until those gates are real, the
   "deep beats shallow" tie-break in `score_entry` acts on noise.
3. **No semantic/vector retrieval.** No embedding path at all. A knowledge
   agent meant to connect ideas across sources can't find "similar
   approach used elsewhere."
4. **Index is in-memory + lazy + per-process.** Built on first stale read,
   300s TTL, never persisted, never shared. Two daemons keep separate
   copies; large corpus re-reads all JSONL on every rebuild.
5. **Deep stage degrades silently.** `run_extraction()` returns
   `(None, "no_llm_attached")` if no provider is wired. A corpus with only
   shallow entries has no concepts/patterns/indexable intelligence — query
   falls back to title/summary substring only.

---

## 4. Improvement backlog (priority order)

- **P0 — Fix retrieval fidelity (biggest payoff, lowest risk).** Replace
  the substring scorer with BM25 over tokenized fields, OR add a small
  embedding index (reuse the provider's existing embeddings → local
  HNSW/flat vector store) and do hybrid lexical + semantic ranking. Turns
  Athena from "grep over notes" into a real knowledge retriever.
- **P1 — Make confidence honest.** Gate deep confidence on whether
  governance/plutus are real (stubs today). Until then, don't let deep
  tie-break outrank a lexical match. Port gates to `arda-core` or mark
  confidence `provisional` in the response.
- **P2 — Persist + share the index.** Write `DigestIndex` to disk (or a
  real store) so it survives restarts and is shared across processes;
  rebuild incrementally on append instead of full lazy re-read on mtime.
- **P3 — Normalize ingest text.** Stem/lemmatize tokens at ingest time
  (store normalized concepts) so `running`/`ran` and `auth`/
  `authentication` collapse. Cheap; complements BM25.
- **P4 — Surface deep-status in query.** Return a flag when matches are
  shallow-only so the caller knows the answer rests on metadata, not
  extracted knowledge.

---

## 5. Relationship to extraction

These are improvements to Athena's *logic itself* and are independent of
where the crate lives in the tree. They can be done before, after, or
independently of the `old-annunimas → crates/spine` migration. P0 is the
single highest-value change and does not depend on the migration.
