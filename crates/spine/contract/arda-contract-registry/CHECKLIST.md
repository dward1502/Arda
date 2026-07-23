CHECKLIST
=========
last_reviewed: 2026-07-23

completed
---------
- [x] workspace membership
      evidence: added `arda-contract-registry` to root `Cargo.toml` `members`
- [x] verification evidence in docs
      evidence: `cargo check/test` pass; values copied into STATUS.md + PLAN.md
- [x] public surface docs
      evidence: README.md + INDEX.md present with lib/registry entries
- [x] olfactory git boundary for doc hygiene
      evidence: `BREAKDOWN.md`/`STATUS.md`/`README.md`/`INDEX.md`/`PLAN.md` present

deferred / not pursued
----------------------
- awaiting `core/state/contract_registry.json` artifact; smoke tests currently validate schema/paths when state exists.
- no runtime wiring path identified yet; this crate is currently a contract/schema source of truth only.
