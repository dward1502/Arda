# Mirromere and RELIC/CITADEL Prototype Provenance Audit

**Date:** 2026-07-30  
**Scope:** read-only provenance and reuse decision for Stage 4 parallel design work  
**Decision:** preserve contracts and evidence; do not copy, migrate, deploy, or promote either prototype

## Evidence method

The audit inspected the current Arda checkout plus the direct external paths named by the product plans. It checked repository identity, manifests, root licensing, source and deployment documentation, representative SHA-256 fingerprints, and available local validation. It did not inspect credentials, mutate either external directory, deploy to CITADEL, or treat historical service claims as live state.

## Mirromere

### Located evidence

A filename search under `/var/home/mythos/Eregion` found no executable Mirromere application or prototype. The only filename containing `mirromere` is the untracked portfolio plan:

- `docs/plans/2026-07-29-mirromere-plan.md`

The canonical design evidence currently consists of:

- `docs/Mirromere_PRD.md`
- `docs/MIRROMERE_RELIC_OUTPOST_VISION.md`
- `docs/plans/2026-07-29-mirromere-plan.md`

The two established design documents are tracked in Arda history; the last commit touching that set is `76df43e02a58c3f7b5c4640f0e1a2a96a33b9a9a`. The portfolio plan is currently untracked in the shared worktree.

### Provenance decision

Mirromere is **design-only in the inspected filesystem**, not a prototype available for migration. Its PRD and outpost vision may remain first-party design inputs, but no implementation provenance, dependency inventory, asset rights, or executable recovery path exists to approve.

Proceed only with independently implemented, versioned contracts behind a disabled-by-default feature flag. Do not claim a Mirromere runtime, beta, or prototype until executable source and its provenance are separately reviewed.

## External RELIC prototype

### Located evidence

Direct source: `/var/home/mythos/Eregion/relic-kiosk`

- The directory exists but is not a Git worktree, so no source remote, author history, tag, or commit can be established from the artifact itself.
- `package.json` identifies private package `annunimas-relic` version `0.1.0`.
- The application is static HTML/CSS/JavaScript with a local `annunimas.relic.scene.v1` file contract.
- The deploy helper resolves fleet identity from the canonical Annunimas root, syncs to a Pi, writes user-systemd units, enables `relic.service`, disables `citadel-companion.service`, and points Chromium at port `8091`. This is operationally consequential and is not an import script.
- The artifact contains 1,175 files, of which 19 are outside `vendor`/`three` paths. The large generated payload is a copied Three.js distribution.
- `.gitignore` classifies `src/vendor/three/` as generated, but that payload is present in the inspected directory. Any distribution must preserve the upstream MIT license and notices and must not attribute the generated dependency to RELIC.
- There is no root license for RELIC-owned source. The copied Three.js tree contains its MIT license, but that does not establish a license for RELIC's own files.
- Historical queue evidence at `core/projects/tasks/queue.jsonl:209` records that the former in-tree RELIC app was extracted to this external path and passed three tests. This establishes the intended relationship to the former private consumer, but it does not recover file-level authorship or Git history.

Representative fingerprints:

| Artifact | SHA-256 |
|---|---|
| `README.md` | `3418aa421bba44f1f3d9681e3e85995df1cfbb37aec65a6d038b3393aa0f2b44` |
| `package.json` | `6cc871b09dddffabbdaf5aca1347f09e446dd82fcece78217f33ccecc33062f4` |
| `src/relic.js` | `ea8ac6bc36e1159853c6b2a346602d56b39c8d69229ea18a167d8432cf8074ef` |
| `src/relicSceneState.js` | `07e8b76fe20bb0b15b2181e54f23d8aac17d24bd6c300f23c4b2be12b6a9f9b3` |
| `public/scene.json` | `8a0892fc3011a47bd8d7fed1cebc21f168e4600ae8c6d46f957496ab8056f9d7` |
| `scripts/deploy_to_citadel.sh` | `67c7696d9b5ef6c33825b52abbe09cb00359f68e8e481493b7f7b02d439ee60b` |

Local validation passed:

```text
npm run validate
3 tests passed; 0 failed
```

This proves the inspected normalization logic is executable. It does not prove current Pi deployment state or runtime integration with Arda.

### Provenance decision

Do not copy RELIC source or assets into Arda while the root license and Git provenance remain unknown. Preserve `annunimas.relic.scene.v1` only as historical compatibility evidence. Define an Arda-owned read-only presence protocol independently, then either:

1. keep the existing prototype as a recoverable external sidecar that consumes the protocol; or
2. perform a clean-room Arda implementation after the protocol and visual requirements stabilize.

Any later direct migration requires explicit operator review plus source ownership/license evidence.

## External CITADEL avatar prototype

### Located evidence

Direct source: `/var/home/mythos/Eregion/citadel-avatar`

- The directory exists but is not a Git worktree.
- It contains 40 files, 37 outside `three` paths, and no root license.
- `package.json` declares only Three.js `^0.183.2`.
- `README.md` describes a local state-polled surface on port `8080`.
- `QUICK_START.txt` claims Git commit `fdfe5e6`, but that claim is not independently recoverable from this non-Git directory.
- `QUICK_START.txt` repeatedly points to `/var/home/mythos/Eregion/citadel-avatar-pure`; that path is absent from the inspected Eregion tree, so those deployment instructions are stale.
- `DEPLOYMENT_TO_PI5.md` claims a historical deployment on port `8000`, but that claim is likewise not independently recoverable from this artifact.
- RELIC documentation describes a later port-`8091` replacement and disabled companion service. These are conflicting historical snapshots, not current operational truth.
- JavaScript syntax checks passed for `scene-runtime.js` and `agent-kinetic-optimized.js`.

Representative fingerprints:

| Artifact | SHA-256 |
|---|---|
| `README.md` | `050445a6c073c045123e0a852ba365f7a94a47d4dfbc815c4b8b4bb0ada058a2` |
| `package.json` | `8d6e30072d1b37ba0b8854c47a27da8d7b0aece0a53fd1255d00cfaf31959fa9` |
| `index.html` | `be55ee48f2be4491756c55d0153bf77d20b52ffc7d8610b508dab96dd075ef44` |
| `scene-runtime.js` | `89cd0680318d80e7749bec05e37a2880da00c3529fff487dad0cbd00acdc22f0` |
| `state.json` | `d97083b6e1785c71776f981fa6d56f0077cb66cf1c819fcddf1384106229aa2a` |
| `run.sh` | `379b0b2bd7359ef8511c4b31ad64507b240dad7323223599ff064e0f4455e36e` |

### Provenance decision

Keep the CITADEL avatar prototype external and disabled as a product dependency. Do not use its deployment documents as live truth. A future CITADEL beta must have one canonical Arda-owned protocol, an explicit feature flag, an independent service/runtime recovery path, and a live deployment verification performed at that time.

## Portfolio gate result

- **Personal Operations:** contract preservation may continue independently.
- **Mirromere:** design-only; no executable prototype approved.
- **RELIC/CITADEL:** external artifacts may inform requirements but are not approved for source migration.
- **Stage 4:** no dependency on these prototypes is justified. Workbench remains recoverable without them.
- **Stage 5+:** any beta activation requires a fresh provenance review, explicit feature flag, and live recovery evidence.
