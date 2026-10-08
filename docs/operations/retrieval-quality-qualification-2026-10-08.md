---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "qualification_evidence"
  owner: "ARDA"
  status: "active"
  reviewed: "2026-10-08"
---

# R07 retrieval quality qualification

Source qualification only, on a dirty shared worktree. No installed binary,
live memory, historical Book, provider route or completed objective was changed
by this slice. Independent review found two benchmark integrity defects; both
were reproduced and repaired; re-review accepted the production fixes and exposed
a default-parallel fixture race, now repaired with test-only serialization. A
subsequent crawler activity fix has no blocking review findings. The review
identified an unbounded fixture-server join; it is now bounded with abort/join
cleanup. This qualifies the bounded measurement scope of the
[R07 task](../plans/CRATE_RELIABILITY_AND_ACCEPTANCE.md#r07-measure-memory-and-research-quality),
not installed rollout or general answer quality.

## Frozen corpus and measurement

The [versioned fixture](../../crates/spine/executors/arda-varda/fixtures/reliability_questions_v1.json)
contains 12 frozen repository-document excerpts and 24 questions. Each excerpt
records its repository path, original line range and content SHA-256. Questions
and relevance labels are agent-authored, not independent human ground truth.
No private operator transcript or memory was copied into this fixture. Frozen
excerpts describe the source at capture time, not perpetual current status.
Fixture SHA-256:
`34c6bc7d16906b482f82076ef5f705286cd06b85990db47824389a3c1279e515`.

The benchmark now evaluates existing Vaire lexical retrieval and Varda production
field-weighted BM25 against the same document IDs, questions and relevance labels
at k=3. It records the input hash, rankings and exact source citation spans.
Top-level Recall@1 is true fractional recall; Hit@1 remains separately named in
the adapter report. The report schema is now v2. Empty labels, repeated IDs,
unknown relevant IDs and colliding source identities fail instead of silently
inflating a score. Canonical source-ID collisions are rejected before ingest;
existing Books, digest/index state or a warmed index reject store reuse. Callers
must exclusively own a fresh isolated store throughout the run. Vaire's evaluator
owns the result cutoff and counts each
relevant document only once, even when an adapter returns duplicates.

| Method | Hit@1 | Recall@3 | MRR@3 |
| --- | ---: | ---: | ---: |
| Vaire lexical, unchanged | 0.916667 | 0.958333 | 0.930556 |
| Varda before source-coverage repair | 0.375000 | 0.354167 | 0.375000 |
| Varda after source-coverage repair | 0.708333 | 0.958333 | 0.826389 |

Varda indexed a truncated title and generic status summary for plain-text input,
leaving most source content unsearchable. The repair keeps a bounded extractive
summary of at most 16,384 Unicode characters for local text. The complete raw
input stays in its ledger; shallow-only provenance is retained. Existing Books
are not rewritten or migrated. New-query results survive isolated-store reopen,
and exact UTF-8 citation offsets are tested.

After repair, all 24 cases have a nonempty top result whose citation offsets,
field contents, source ID and document ID validate. This measures referential
citation integrity, not generated-answer correctness or entailment. The separate
answer experiment below evaluates source-resolved top-three context. First-result ranking still trails lexical
retrieval; the data does not justify introducing a vector database. Do not tune
weights against this small development corpus and then call it held-out quality.

## Generated-answer experiment

A separate Hermes model worker (`gpt-6-astra`) answered the 24 corpus questions
using only the frozen sources resolved from their production top-three document
IDs, plus two unsupported-question controls. No relevance labels or answer rubric
were supplied to that worker. This is an offline retrieval/source-resolution
harness, not the production Vaire capsule or answer-serving path.

Retained artifacts:

- [Exact per-case model input](../../crates/spine/executors/arda-varda/fixtures/reliability_answer_input_v1.json).
- [Rubric frozen before answers were read](../../crates/spine/executors/arda-varda/fixtures/reliability_answer_rubric_v1.json).
- [Actual model answers and quotes](../../crates/spine/executors/arda-varda/fixtures/reliability_answers_v1.json).
- [Per-case judgments, hashes and score totals](../../crates/spine/executors/arda-varda/fixtures/reliability_answer_scores_v1.json).

Parent-agent semantic review scored 23/24 corpus answers correct and both
unsupported controls appropriately abstaining (25/26 overall). All 26 were judged
faithful to their supplied context; programmatic checks verified exact citation
quotes against the correct per-case source and every expected ID exactly once.
These are agent-authored labels and agent judgments, not independent human ground
truth. Exact quote matching is not a semantic entailment oracle.

The failed corpus answer is q08: production retrieval omitted `m5-boundaries`, so
the worker correctly abstained instead of inventing M1/M2 closure. Partial answers
q10/q12 appropriately declined to invent absent storage paths or job names. The
measured miss is retained, not hidden by relabeling or retraining on this corpus.
The two controls ask for energy and answer-faithfulness numbers absent from their
sources; both abstained.

Score counts are reproducible from the retained per-case judgments; re-generating
answers uses the recorded model-input/prompt contract but is nondeterministic.
To recheck exact quotes and totals from the repository root:

```sh
python3 -c 'import json,pathlib; p=pathlib.Path("crates/spine/executors/arda-varda/fixtures"); i=json.loads((p/"reliability_answer_input_v1.json").read_text()); a=json.loads((p/"reliability_answers_v1.json").read_text()); s=json.loads((p/"reliability_answer_scores_v1.json").read_text()); contexts={c["id"]:{d["id"]:d["text"] for d in c["sources"]} for c in i["cases"]}; assert len(a["cases"])==len(contexts)==len({c["id"] for c in a["cases"]}); assert set(contexts)=={c["id"] for c in a["cases"]}; assert all(q["quote"] and q["quote"] in contexts[c["id"]][q["source_id"]] for c in a["cases"] for q in c["citations"]); print("correct",sum(c["correct"] for c in s["cases"]),"faithful",sum(c["faithful"] for c in s["cases"]),"total",len(s["cases"]))'
```

## Checks and retained continuity

Executed on October 8:

- `cargo test --locked -p arda-varda -p arda-vaire --all-features -j 2`:
  242 passed, 0 failed, 2 ignored. Ignored live fixtures are not counted as passing.
- `cargo clippy -p arda-varda -p arda-vaire --all-features --all-targets -j 2 -- -D warnings`:
  passed after repairing a new test lint.
- Both new plain-text retrieval tests first failed on the old generic summary,
  then passed after the source repair. Coverage includes tail-body retrieval,
  UTF-8 offsets, isolated restart and bounded-prefix/raw-ledger preservation.
- A local HTTP table checks 503, malformed JSON, missing markdown and explicit
  success=false. All clear active activity, retain errors and leave partial text
  unindexed. The unsuccessful-result case first failed because activity was marked
  complete; the repair preserves the returned success=false API while recording
  activity failure. This is a simulated transport qualification, not an installed
  external-service acceptance run.
- The full suites retain existing scope, privacy, correction, revocation,
  selection/budget and crawler error regressions. These are source tests, not a
  new live provider or external crawler acceptance run.
- `python3 scripts/qualification/audit_m5_continuity.py --objective operator-objective-c98d83591ddab744 --objective operator-objective-072182cbd75bf44c --output target/qualification/r07-20261008/m5-audit.json`:
  passed retained scope/prompt/terminal-lineage/unique-outcome checks. Its six
  adversarial unit tests also pass under `python3 -O`. This audit reads existing
  acceptance evidence; it does not mint a new context-use receipt or replace
  semantic review of accepted output. The
  [M5 owner](../plans/autonomous-task-completion/M5_VAIRE_CONTINUITY.md) remains
  authoritative for bounded operator acceptance and its limitations.

Reproduce the measurement:

```sh
cargo run -p arda-varda --bin arda-varda-benchmark -- \
  crates/spine/executors/arda-varda/fixtures/reliability_questions_v1.json
```

Local evidence directory: `target/qualification/r07-20261008/` contains
`before.json`, `after.json`, `after-reviewed.json`, source-hash `manifest.json`, final test and Clippy
logs, and `m5-audit.json`. These are local build artifacts, not durable published
acceptance. The tracked fixture and this report preserve the corpus and bounded
result; full private M5 records remain in their existing owners.

## Isolated commit qualification

An exported staged tree, excluding unrelated working-tree changes, passed
237 tests with 1 ignored live test and strict all-target/all-feature Clippy.
The larger 242/2 result above belongs to the dirty working tree. Isolation exposed
a pre-existing context-capsule test initializer missing `excluded_refs`; only its
empty-field initialization is included, not the unrelated capsule implementation
or additional tests. The unrelated crawler transport changes are also excluded.

## Remaining assertions

| Boundary | Existing evidence | Still required |
| --- | --- | --- |
| Retrieval | Same-corpus lexical/BM25 scores and exact spans | Independently labeled broader held-out questions before quality generalization |
| Answer quality | Retained 26-case generated answers, rubric and scores | Broader independently labeled evaluation; no production answer-path claim |
| Continuity | Retained M5 audit plus current source regressions | Preserve M5's accepted scope; do not promote prompt presence into a new consumption receipt |
| Crawler | Four local HTTP degradation cases and repaired failure activity | Installed external-service acceptance is not claimed |
| Delivery | Source tests, strict Clippy and independent code review | Explicit installed qualification if rolling out; no installation occurred |
