# Storage and retention audit — October 7, 2026

The 311 MB of main Manwe history is manageable as archived evidence, but it
should not grow indefinitely or be scanned on every health request. The
[installed health repair](provider-qualification-2026-10-07.md) removes that
full-history scan from readiness. It does not implement retention.

The more immediate capacity issue is the filesystem: 844 GiB used out of
882 GiB, 39 GiB available (96%). No data was deleted by this audit. This is a
scoped survey of Arda, Hermes and user caches, not an attribution of all used
space. Sizes below are rounded `du` allocated sizes; nested rows overlap, and
shared/reflink extents can make apparent reclaim estimates misleading.

## Largest consumers and existing controls

| Location | Observed size | Assessment |
| --- | ---: | --- |
| Arda `target/` | 86 GiB | Generated builds/evidence: debug 61 GiB, qualification 13 GiB, native-check 8.8 GiB, release 3.5 GiB. Preserve the running release, rollback executable and acceptance evidence when pruning. |
| `target/qualification/s8-fix-release/isolated-target` | 11 GiB | Old isolated build output inside the preceding total; a strong cleanup candidate after checking active process references. Preserve the associated source/receipts. |
| `~/.hermes/` | 63 GiB | Most is the agent repository, not conversation history. |
| `~/.hermes/hermes-agent/.git/objects` | 48 GiB | `git count-objects -vH`: 1,010 packs, 47.61 GiB packed data and 39.93 MiB in three temporary pack/index/reverse-index garbage files. The reported 45,350,827 packed entries can include duplicates; this is not a unique-object count. No Git process was active during inspection. No local `gc.*`, `maintenance.*` or `fetch.*` setting was returned; that does not rule out external maintenance. Do not blindly repack with only 39 GiB free. |
| `~/.hermes/state-snapshots` | 2.8 GiB | One retained pre-update snapshot dated July 16. Expiration/rollback policy needs an explicit owner. |
| `~/.hermes/checkpoints` | 2.7 GiB | Includes 2.4 GiB object storage and a 211 MiB legacy directory dated May 7. Use reference-aware checkpoint collection, not file-age deletion of shared objects. |
| `~/.hermes/sessions` | 1.0 GiB | Existing gateway pruning removes old routing mappings only; its source explicitly retains transcripts. The default mapping age is 90 days. This is not transcript retention. |
| `~/.hermes/logs` | 60 MiB | At least the tool-call logger already rotates at 5 MiB with three backups; do not infer all Hermes logs share that policy. |
| `~/.cache` | 46 GiB | Includes Arda and Annunimas build caches at roughly 14 GiB each, llama.cpp 5.6 GiB, LM Studio 5.3 GiB, Hugging Face 2.8 GiB and browser caches. The two build paths are separate directories; shared extents were not measured. |
| `~/.local/share` | 30 GiB | Containers account for 21 GiB. Check images/volumes and active ownership before any prune. |
| Arda `data/` | About 1 GiB | Canonical data is much smaller than generated builds. No blanket TTL is appropriate here. |
| `data/governance` | 448 MiB | Seven approximately 64 MiB Bacon Lite rotations plus the active file. Existing writer defaults are 64 MiB/file and seven retained files: size retention already exists. Age-based expiry was not observed. |
| `data/manwe` | 364 MiB | Main state and governance histories were initially 311,085,710 bytes combined. The event writer appends through long-lived handles; no rotation was found in that writer. External rename/truncate would be unsafe. |
| `data/prometheus` | 153 MiB | Orders about 74 MiB and escalations about 79 MiB. The inspected writer appends without rotation. Its query-result `truncate` is in-memory limiting, not on-disk retention. Open/unresolved state must survive compaction. |

Source references: `crates/spine/runtime/manwe/src/adaptive/service/full/event_writer.rs`,
`crates/spine/governance/arda-governance/src/bacon_lite.rs`,
`crates/spine/observability/arda-aule/src/prometheus/orders.rs`, and the installed
Hermes `gateway/session_lifecycle.py`, `gateway/run_watchers.py`,
`gateway/run_turn.py` and `gateway/config.py`.

## Integrity findings

The read-only scanner checked 949,088,521 bytes across 18 JSONL/history files
and the empty governance lock file. It counted 639,695 valid terminated records
and two malformed terminated records. No unterminated tail or detected
rotation/truncation made these scans inconclusive. This checks JSON syntax, not
record schema, lineage, semantic correctness or completeness of historical data.

| File | Malformed record byte offset | Bytes | SHA-256 of record |
| --- | ---: | ---: | --- |
| `data/manwe/governance_events.jsonl` | 77,243,220 | 3,621 | `35e30804b6e7ff986630a15c47d0344872646dae87078efb9590e9a722892bb5` |
| `data/manwe/state.jsonl` | 230,604,552 | 593 | `f328234ca90e24853aa158d306f5436d64b46ca2a51a79b694891a632d63c3f8` |

The parser reported a missing comma delimiter and a missing colon delimiter,
respectively. These are completed malformed records inside the histories, not
an active writer's unfinished tail. Their cause has not been established. Keep
the original bytes and provenance when quarantining or repairing them; silent
deletion would hide evidence. Other scanned records parsed successfully.
`PRAGMA quick_check` on `data/arda/objectives.sqlite3`, opened read-only, returned
`ok`. This is not a full integrity check of every database or Git object store.

Reproduce a read-only inventory with:

```bash
python3 scripts/qualification/audit_jsonl_storage.py \
  data/manwe/*.jsonl data/governance/bacon_lite.jsonl* data/prometheus/*.jsonl
python3 -m unittest discover -s scripts/qualification -p test_audit_jsonl_storage.py
```

The scanner reads only each file's opening byte extent, emits counts, offsets
and hashes rather than record contents, and distinguishes an unterminated tail
from completed corruption. Exit status 1 flags malformed complete records or an
inconclusive scan. It assumes append-only writers; inode/size/mtime checks do
not prove absence of every possible concurrent in-place rewrite. Use a stable
snapshot for forensic guarantees. Four regression tests passed.

Detailed local evidence is in `target/qualification/r06-rollout-20261007/`:
`storage-depth.txt`, `jsonl-integrity.json`, `jsonl-integrity-final.json`,
`malformed-summary.json` and
`sqlite-integrity.json`. That directory also contains a private configuration
backup; it must not be bulk-staged, published or archived into public evidence.

## Recommended order of remediation

1. Reclaim obsolete generated build outputs first. Inventory process references,
   preserve release/rollback and acceptance receipts, and prune selected old
   build directories. This offers substantially more benefit than deleting
   311 MB of production history.
2. Investigate Hermes Git pack duplication and failed-maintenance leftovers.
   Establish free scratch space, repository integrity, ref/reflog policy and
   rollback before maintenance; do not treat reachable packs as expendable.
3. Add writer-owned size/age rotation for Manwe, with bounded active history,
   checksummed archives and an explicit archive-expiry policy. Readers must
   retain required recovery/audit semantics. Quarantine malformed records with
   offsets and original hashes through that owner, not an external truncator.
4. Compact Prometheus histories by terminal state while preserving unresolved
   orders/escalations and referenced evidence. Test replay before expiring an
   archive; age alone cannot identify safe-to-delete records.
5. Define snapshot, transcript and checkpoint retention separately. Keep
   pinned/referenced sessions and rollback snapshots; garbage-collect shared
   checkpoint objects only after reference analysis. Add disk-usage and failed
   cleanup alerts so retention failures become visible.

These are follow-up actions, not claims that new retention has been installed.
The current change supplies measured findings and a reusable integrity scanner.
