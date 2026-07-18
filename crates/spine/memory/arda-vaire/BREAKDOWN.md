---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "memory_service"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-vaire
Memory service for Arda agents: significance-weighted episodic records,
identity state, recall/recent/relevant, knowledge-seed recall, consolidation,
Obsidian sync, and daemon transport.
Owner: hades | Sigil: 📜 SCROLL | Status: active

## Summary
`arda-vaire` is the continuous memory/identity persistence layer for the
Mnemosyne agent. It stores significance-weighted memory events with hash-
chained episodic integrity, provides scoped recall, synthesizes identity
state, and exposes an optional HTTP/SSE + Unix-socket daemon interface.

This crate is **blocked in tests** by a missing test dependency:
`service.rs:303` imports `arda_plutus::PlutusService` but `arda-plutus`
is not declared in `Cargo.toml`. Compile-time check passes because
`cargo check` does not build test-only code by default; `cargo test`
fails on this unresolved import.

- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/memory/arda-vaire`
- Data roots: `data/mnemosyne/*`, env-overridable via `ARDA_MNEMOSYNE_HOME`
- Contract dual-write opt-in: `ARDA_CONTRACT_MEMORY_ROOT`

## Verification status

- `cargo check -p arda-vaire`: OK
- `cargo test -p arda-vaire`: 24 passed, 0 failed
- Doc tests: 0
- Import alias fixed: test-only `arda_plutus::PlutusService` renamed to `arda_economics::PlutusService` in `src/service.rs:303`
- Nested legacy test tree removed: `crates/spine/memory/arda-vaire/crates/annunimas-mnemosyne/` was a superseded carryover from the Annunimas port; deleted after confirming tests were already promoted to top-level `tests/`

## Port status from Annunimas

Source: `~/Annunimas/crates/annunimas-mnemosyne/`

Ported correctly:
- All `annunimas_*` prefixes → `arda_*`
- `AnnunimasError` → `ArdaError`
- `annunimas_root()` → `arda_root()`, `ANNUNIMAS_ROOT` → `ARDA_ROOT`
- `annunimas_plutus::JouleWorkUnit::Reasoning` → `arda_economics::JouleWorkUnit::Reasoning`
- All transport/http + transport/ipc error paths
- All service submodules: `store.rs`, `retrieval.rs`, `promotion.rs`, `status.rs`
- Top-level tests promoted: `tests/knowledge_deltas.rs`, `tests/public_flows.rs`

Intentionally different in Arda:
- `lib.rs` public exports are simplified: Arda exports only `InformantEvent` + `MnemosyneService`; Annunimas exported transport daemon types as well. This is a conscious Arda simplification, not missing port work.

Superseded source:
- `~/Annunimas/crates/annunimas-mnemosyne/` is the legacy source tree. Do not modify it from Arda. Use it only as a reference for historical context or missing pieces.

## Agentic-OS abstractions
- **Memory encoding**: `encode(InformantEvent)` -> optional recall entry
  - significance scoring from joulework, love equation, triad, bacon-lite
  - classification into 5 tiers with sigil:
    - MNEME_CORE / MNEME_ACTIVE / MNEME_PERIPHERAL / MNEME_TRANSIENT / MNEME_RELEASED
  - low-significance events go to noise ledger, not episodic storage
- **Hash chain integrity**: SHA256 chain over previous head + event +
  significance; stored in `chain_head`
- **Episodic store**: monthly JSONL files under `episodic/YYYY-MM/`
  - header + body record format
  - malformed-record tolerance in read path
- **Consolidation**: tag-clustered semantic patterns + procedural skills
  - thresholds: min cluster size 2, avg significance >= 0.4
  - archive log with sweep metadata
- **Recall surfaces**:
  - `recall_recent(hours, crate_filter)`
  - `recall_recent_scoped(hours, crate_filter, scope_filter)`
  - `recall_relevant(query, hours, crate_filter, scope_filter, limit)`
  - `recall_knowledge_seeds(query, limit)` from triage registry + Athena deep books
  - `identity_state()` -> counts/mission focus/recent events
- **Memory checkpoint policy**:
  - derives recall window, checkpoint interval, consolidation bias
  - driven by recent memory pressure and priority tags
- **Obsidian sync**:
  - indexes `.md`/`.canvas` files, writes obsidian_index.jsonl
  - encodes each note as an `InformantEvent` with 0.65 confidence hint
- **Status/observability**:
  - counts by sigil class, malformed record counts
  - last/next consolidation timestamps
  - checkpoint policy reporting
- **Daemon transport**:
  - IPC Unix socket + optional HTTP/SSE
  - configurable timeouts via env
- **Phase 1 contract dual-write**:
  - optional `MemoryRecord` write to `<contract_root>/episodic/<id>.json`
  - atomic tmp+rename; primary write never poisoned by dual-write failure
  - env-controlled with `ARDA_CONTRACT_MEMORY_ROOT=auto` defaulting to
    `core/state/memory`

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Public exports: `InformantEvent`, `MnemosyneService` |
| `service.rs` | Core store: encode, recall, identity, consolidate, obsidian sync, dual-write |
| `service/store.rs` | File append helpers, hash chain, path defaults, dual-write plumbing |
| `service/retrieval.rs` | Recent/relevant/knowledge-seed recall + scoring |
| `service/status.rs` | Stats, status, malformed-record accounting |
| `service/promotion.rs` | Consolidation + Obsidian sync |
| `significance.rs` | Significance scoring, tier classification, bonuses/penalties |
| `error.rs` | Canonical error type |
| `transport/mod.rs` | Daemon config + runner |
| `transport/ipc.rs` | Unix socket server |
| `transport/http.rs` | Optional HTTP/SSE server |
| `README.md` | Sigil/metadata overview |

## Consumer wiring
- Used by:
  - `arda-orome` context enrichment
  - `arda-varda` interceptor pipeline
  - likely `engine`/CLI via `from_default_or_fallback()`
- Depends on:
  - `arda-core`
  - `arda-governance`
  - `arda-economics`

## Ideas for improvement
1. Fix test compile error: add missing `arda-plutus` dependency or
   remove unused `PlutusService` import from tests
2. Make recall queries filter by `memory_scope`, sigil class, and
   significance threshold in `read_episodic_records` instead of in-memory
3. Replace full-disk `read_episodic_records()` reads with indexed month
   metadata so large stores don’t scan everything
4. Add async store methods or a bounded async gateway so daemon transport
   doesn’t compete with sync encode paths
5. Make Obsidian sync incremental: track `last_synced_at_utc` per note
   instead of re-reading everything
6. Add explicit retention policy for noise/transient episodic files so
   disk usage doesn’t grow unbounded
7. Use shared ledger/append trait from `arda-core` instead of custom
   `append_jsonl`
8. Surface memory pressure/readiness metrics through `engine`/HUD
9. Add schema-versioned episodic records with migration support
10. Replace hard-coded sigil multipliers with governance-configurable weights
