# arda-vaire status

Crate: `crates/spine/memory/arda-vaire`
Current state: active
Branch: `manwe`
Test evidence: `cargo test -p arda-vaire` (29 unit + 5 integration passing, 2026-07-25)
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md`.

Evidence level: see `CRATE_PLAN.md` and `CHECKLIST.md`.
Memory promotion is receipt-backed and regression-tested. IPC forwarding covers round-trip, unreachable, malformed-response, and local-default behavior. The transport is provider-free, so a missing-provider state does not exist.
Recall results expose confidence/trust. In-process observability reports recall fidelity/latency, IPC dispatch queue latency, consolidation depth, and promotion receipt totals.
Governance scoring has crate-local duplicate, overload, and novel-checkpoint regression coverage. Knowledge-delta integration covers all four memory-scope archetypes.

Additional evidence: no-default-feature tests pass (27 unit + 5 integration); the controlled recall benchmark completed 600 queries at Hit@1 1.000 and 63.46 µs/query on this host.

Open risk: benchmark fixtures establish a repeatable local baseline but are not an apples-to-apples comparison with Mem0, MemX, Zep, or other public systems. Observability counters are process-local and reset on restart.
