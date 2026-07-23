# arda-economics — implementation checklist

Source: `BREAKDOWN.md`, live source review, and test evidence.

- [x] Verified crate metadata, public surface, and data contracts
- [x] `cargo check -p arda-economics` passes
- [x] `cargo test -p arda-economics` passes
- [x] Added `INDEX.md` and crate-level `README.md`
- [x] Replaced duplicated root discovery with `arda_core::layout::arda_root_from()`; `ARDA_ROOT` canonical, `ANNUNIMAS_ROOT` fallback
- [x] Made `EnergyMeter::estimate()` async while retaining sync estimator path
- [x] Added v1-to-v2 `runtime_status.json` migration with rejection of unknown future schemas
- [x] Added ROI, LoveEquation restore, tariff validation/reload, input validation, schema migration, and budget-threshold tests
- [x] Added warning/critical budget threshold hooks and finite zero-budget behavior
- [x] Added `AffordabilityPolicy` + `AffordabilityDecision` in `arda-core`; `EconomicsEngine` implements it and `dispatch_full_with_affordability()` enforces it
- [x] Kept transport in this crate after review; HTTP already feature-gated; extraction triggers documented
- [x] Added shared `arda_core::ledger::AppendOnlyLedger`; `PlutusService` writes `arda.plutus.event.v1` on successful mutations
- [x] Added `arda-cli plutus export` with human/`--json`/`--path` output and missing-state handling
- [x] IPC/HTTP transport tests pass: configurable timeouts, Unix socket daemon, HTTP/SSE daemon
- [x] Tariff TOML loads from shipped file and disk reload paths
- [ ] Add integration coverage for failed hardware backends and estimator fallback combinations
- [ ] Add observability hooks for budget pressure, tariff table staleness, and IPC/HTTP queue latency
- [ ] Add finance metrics export for `PlutusRuntimeEvent` streams
- [ ] Validate JouleWork unit multipliers and source-provenance invariants across long-running sessions
