# Canonical Project Task Queue

`queue.jsonl` is Arda's global, append-only project-task evidence ledger.
Aule, queue projections, task selection, and append-only validation resolve this
path through `ARDA_PROJECT_TASK_QUEUE_PATH`; relative overrides are resolved
from the repository root. The default is:

`core/projects/tasks/queue.jsonl`

The ledger was restored byte-for-byte from its last authoritative pre-relocation
revision after repository cleanup removed both this path and its temporary
`docs/plans/projects/tasks/` relocation.

The ledger at
`crates/spine/executors/arda-varda/core/projects/tasks/queue.jsonl` is
component-local ATHENA/Varda state. It is not a fallback global queue and must
not be merged into this ledger implicitly. Any future federation requires an
explicit producer contract and provenance-preserving import receipts.

Mutations to the canonical ledger remain append-only. Run
`scripts/check_task_queue_append_only.sh` after appending records.
