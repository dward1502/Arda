# Stage 5 local-history sanitation receipt

**Executed:** 2026-08-06
**Scope:** unpublished `manwe` commits ahead of `origin/manwe`
**Disposition:** sanitized branch lineage created; original lineage retained only in a local backup ref

## Authorization and recovery boundary

The operator explicitly authorized committing the pending source fix and continuing with history sanitation. Before rewriting the branch:

- the final pre-rewrite commit was `dd80e5ae2ca2f2a25b96299594ef4c14341af184`;
- its exact tree was `0acecd1c1c602416f2ed32de2048f0e406929d7d`;
- local recovery ref `refs/backup/manwe-pre-sanitize-20260806T053049Z` was created;
- unrelated tracked and untracked working files were preserved in stash `pre-history-sanitation-20260806T053049Z`.

The backup ref and stash are local recovery material and must never be pushed.

## Rebuild result

A new commit was created directly from the exact corrected tree with `origin/manwe` as its sole parent:

- sanitized consolidation commit: `7b21dd54350420d53ef7893bc3d3a2c2c39826ad`;
- parent: `f646245ce5dde26490dfbacb209571ce8f064fc6` (`origin/manwe` at rewrite time);
- rebuilt tree: `0acecd1c1c602416f2ed32de2048f0e406929d7d`;
- tree identity compared with the pre-rewrite corrected tree: exact match;
- ahead-only consolidation commits immediately after rebuild: one.

A follow-up sanitation change removes the inherited Annunimas queue archive from the release tree while retaining its row count and SHA-256 in the queue-retirement receipt. Exact local worktree paths in two preflight records were redacted, and the RELIC sidecar default now uses a remote-home-relative path instead of a named account home.

## Reachability and path verification

Verification against the rebuilt `manwe` branch established:

- `git branch --contains a59e135a` returned no branches;
- `git rev-list origin/manwe..HEAD` contained only the rebuilt lineage;
- `git ls-tree` found no paths under `data/governance`, `data/manwe`, `data/plutus`, or `data/prometheus` in the final tree or any ahead-only tree;
- the inherited `20260805T201411Z_annunimas_queue_legacy.jsonl` runtime archive is absent from the release tree;
- `git diff --check` passes;
- Git object validation reports no corrupt reachable objects.

Unreachable and backup-retained objects can remain in the local object database until recovery retention is explicitly ended. They are not reachable from `manwe` and are not included by the normal branch push refspec.

## Release consequence

The generated/private runtime-history blocker is closed for the `manwe` branch. This receipt does not authorize artifact signing or release: the sanitized final source must still pass focused verification, a complete-matrix reliability smoke, and a new uninterrupted 24-hour soak before artifact lifecycle and independent evaluator gates.
