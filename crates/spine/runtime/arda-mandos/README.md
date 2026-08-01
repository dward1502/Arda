# arda-mandos

`arda-mandos` is Arda's bounded, advisory Oracle runtime. It evaluates typed queries through the Aurelius, Bacon, and Sun Tzu gates; records explainable verdicts in a restart-safe integrity chain; and exposes one shared command contract through direct Rust, Unix-socket IPC, and optional HTTP interfaces.

Mandos provides decision support. It does not grant execution, approval, or autonomous consensus authority.

## Public surface

- `OracleEngine`: deterministic query validation, gate evaluation, typed outcomes, conditions, concerns, and low-cardinality status counters.
- `OracleService`: serialized evaluation, idempotent request identity, versioned JSONL persistence, restart hydration, integrity verification, atomic verified export, bounded recent history, and bounded best-effort Plutus telemetry.
- `OracleDaemon`: supervised IPC and optional HTTP listeners with bounded payloads and graceful shutdown.
- `EvidenceRef`, `ReasoningContext`, and `PageIndex`: typed provenance, bounded public reasoning graphs, and stable document references.

## Runtime interfaces

IPC commands:

- `status`
- `evaluate`
- `verdicts`
- `paths`
- `verify_ledger`
- `export_ledger` with `{"destination":"operator/export.jsonl"}`

HTTP routes when the default `http` feature is enabled:

- `GET /status`
- `POST /evaluate`
- `GET /verdicts?limit=N` (capped at 100)
- `GET /paths`
- `GET /ledger/verify`
- `POST /ledger/export` with `{"destination":"operator/export.jsonl"}`
- `GET /events`

HTTP and IPC share typed request parsing, service dispatch, redaction, query-error mapping, and structured error codes. Transport export destinations are relative paths rooted beneath `<ARDA_MANDOS_HOME>/exports`; absolute paths and traversal components are rejected. The direct Rust service API remains available for explicitly trusted operator-selected destinations.

## Persistence and operations

`ARDA_MANDOS_HOME` selects the runtime root. The authoritative files are:

- `runtime_status.json`: atomic operational snapshot.
- `verdict_history.jsonl`: append-only versioned verdict records linked by sequence and SHA-256 digests.

`OracleService::verify_ledger()` validates schema support, sequence continuity, previous-record links, request/verdict/record digests, verdict deserialization, terminated records, and legacy unchained records. `export_verified_ledger()` serializes against concurrent evaluations, refuses corrupt or degraded ledgers, refuses the authoritative path as its destination, and uses temporary-file plus atomic-rename replacement with cleanup on failure.

## Verification

Packet 4 closeout evidence on branch `manwe`:

- `cargo fmt -p arda-mandos -- --check`
- `cargo test -p arda-mandos --all-features`: 75 unit + 2 integration tests passed.
- `cargo test -p arda-mandos --no-default-features`: 68 unit + 2 integration tests passed.
- Strict all-feature and no-default-feature Clippy passed with `--all-targets --no-deps -- -D warnings`.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-mandos --all-features --no-deps` passed.
- Direct consumers passed all-feature tests and checks: `arda-aule` and `arda-orome`.

See `BREAKDOWN.md` for the implementation map, `STATUS.md` for current evidence, `OWNERSHIP.md` for authority boundaries, and `INDEX.md` for navigation.
