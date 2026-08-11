# Final-source 24-hour soak assessment — S5-R1

**Assessment date:** 2026-08-07
**Receipt:** `soak-24h-final-efd118b5-20260807.json`
**Disposition:** valid pass; satisfies the final-source Stage 5 reliability soak gate

## Source and receipt identity

- Frozen commit: `efd118b5339f42df133fdfb9d3256c64a02b7e59`
- Frozen tree: `f499a4d6f66bbf0a6845bd33f77061ba2f47835c`
- Frozen source fingerprint: `3254da9e964dd488ab6550d79fa21b4f382af2facae8a99ea79678aaaff01c27`
- Launch receipt SHA-256: `1294e021ca2d2a69b409df1dcdb669d258d8bb8ba67973ba80e20cbf5d87870c`
- Smoke receipt SHA-256: `24d5a83bd6079050767e005c83f065f3a654b67579c44fee8f1d440316d81c13`
- Final receipt SHA-256: `cbad4e54f12bede0c1afed937eeecf488e95b25add485348889282f3a7608e40`
- Frozen worktree status after completion: clean

The launch record binds the run to the commit, tree, and source fingerprint above. The final receipt's before/after source fingerprints are byte-identical to the launch fingerprint.

## Gate result

- Requested duration: 86,400 seconds
- Elapsed duration: 86,400.000077 seconds
- Scenario executions: 2,844
- Passed: 2,844
- Failed: 0
- Scenarios represented: all eleven required failure classes
- Scenario latency budgets: preserved for every scenario
- Protected-state growth: 0 files / 0 bytes
- Minimum observed free space: 168,845,426,688 bytes
- Required free-space floor: 68,719,476,736 bytes
- Source integrity: unchanged
- Receipt status/validity: `pass` / `valid`

The receipt therefore closes `REL-SOAK-001` for final source `efd118b5`. It does not close packaging, installed-lifecycle, independent-evaluator, remote-authentication, or adapter-artifact trust gates.
