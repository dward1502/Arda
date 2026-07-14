# Migration Plan: `annunimas-charon` → `manwe`

> Status: PLANNING / BLOCKED-PENDING-DECISION
> Author: agent (evidence-gathered 2026-07-14)
> Source-of-truth: on-disk `Arda/` workspace, not the charon docs (which are stale).

## 0. Ground-truth findings (read before acting)

These were verified by reading the actual files, not the charon INDEX/README.

1. **charon is NOT a workspace member.**
   `Arda/Cargo.toml` `members` = `[engine, manwe, spine/governance/arda-core,
   spine/governance/arda-tool-harness, spine/interface/arda-onboarding,
   spine/executors/arda-service-registry]`. `crates/old-annunimas/annunimas-charon`
   is excluded → it is not compiled by the Arda build today.
   charon source root: `crates/old-annunimas/annunimas-charon/` (NOT
   `crates/spine/runtime/annunimas-charon` as its own INDEX.md claims).

2. **charon's dependency crates are stubs / absent.**
   `annunimas-charon/Cargo.toml` path-deps on `../annunimas-core`,
   `../annunimas-governance`, `../annunimas-mnemosyne`, `../annunimas-plutus`.
   The local copies under `old-annunimas/` are stub crates (e.g.
   `annunimas_core` resolves to an essentially empty `lib.rs`; several
   `annunimas_*` path targets are not present on disk). charon's
   `service.rs` imports `GovernanceChainConfig`, `JouleWorkUnit`,
   `MnemosyneService`, `load_governance_chain`, etc. — none of which have a
   real impl in this tree. => charon does NOT compile in Arda as-is.

3. **`manwe` is the ACTIVE gateway and is deliberately minimal.**
   `crates/manwe/src/main.rs` (171 LOC): OpenAI-compat proxy on `127.0.0.1:7171`,
   `/v1/chat/completions` + `/v1/models`, static `manwe.toml` provider catalog,
   naive 1:1 forward with `provider/`-prefix strip. Its doc header cites
   `REFACTOR_PLAN.md §0/§2`: manwe is the **frozen contract** local root; the
   `remote` adapter / adaptive routing is an **explicit later step ("NOT before")**.

4. **Dangling broken reference in `engine`.**
   `crates/engine/src/lib.rs:10` has `pub use annunimas_charon as charon;`, but
   `crates/engine/Cargo.toml` declares NO `annunimas_charon` dependency. That line
   cannot resolve; `cargo build -p arda-engine` will fail on it. (Confirm with a
   build before any merge work.) This is a leftover from before charon was moved
   under `old-annunimas`.

5. **charon docs are path-drifted.**
   README references `/var/home/mythos/Annunimas/...` and `crates/annunimas-charon/...`;
   INDEX.md references `crates/runtime/annunimas-charon/...`. None match the
   on-disk Arda layout. The "169 tests pass / 21 providers" baseline was measured
   against the full Annunimas tree, not this workspace.

## 1. Recommended strategy (do NOT do a naive full merge)

A literal "lift service.rs + 20 submodules into manwe" violates the documented
`manwe` frozen contract (#3) and cannot compile because of the stub deps (#2).
Instead, treat this as the **documented next step** — growing manwe from a static
local root into a routing-capable gateway — done behind a feature flag so the
current 7171 behavior is preserved by default.

Two viable shapes:

- **A. In-repo library (recommended):** extract charon's routing engine into
  `crates/manwe-routing` (or `crates/spine/routing/charon`), keep `manwe` a thin
  binary, gate adaptive routing behind `#[cfg(feature = "adaptive")]`. manwe's
  default build stays the frozen local proxy.
- **B. Full in-binary merge:** add the submodules directly under `crates/manwe/src/routing/`.
  Heavier diff, same feature-gate requirement, higher blast radius on the 7171 path.

Both require resolving the 4 `annunimas_*` deps first (see §2).

## 2. Dependency resolution

Shared, already-compatible deps (versions match workspace): `serde`, `serde_json`,
`tokio`, `chrono`, `tracing`, `thiserror`, `toml`, `reqwest`, `axum`, `tower`,
`tokio-stream`, `base64`, `rand`, `regex`, `async-trait`.

Problem deps — the 4 `annunimas_*` crates. Choose ONE path:

- **Option 1 — port to spine:** re-point charon's governance/state calls onto the
  migrated `arda-core` (`crates/spine/governance/arda-core`) where equivalents
  exist (governance gates, contracts, state). Requires mapping each
  `annunimas_*` type to an `arda-core` type and writing adapters.
- **Option 2 — thin trait shims (lowest risk):** define local trait boundaries in
  `manwe-routing` (`RouterGovernance`, `WorkLedger`, `MemoryStore`) with a
  no-op/default impl used by manwe; keep charon's routing math but drop the hard
  Annunimas coupling. This preserves the routing algorithm while making the crate
  buildable and testable in isolation.
- **Option 3 — vendor the real crates:** copy the genuine `annunimas-core/-governance/
  -mnemosyne/-plutus` from `~/Annunimas` into the workspace and fix them. Highest
  fidelity, largest surface, most maintenance.

Recommendation: **Option 2** for a first mergeable artifact, **Option 1** as the
follow-up that actually wires governance. Do NOT use Option 3 without explicit
user direction (it pulls a large external surface into Arda).

## 3. Module restructuring (shape A)

```
crates/manwe-routing/                # new lib crate (workspace member)
  Cargo.toml                         # deps from §2; default-features minimal
  src/lib.rs                         # pub mod routing; pub mod proxy; ...
  src/types.rs                       # CharonRequestEnvelope, ProviderState, RouteDecision
  src/transport/{mod,ipc,http}.rs    # optional HTTP/SSE behind feature
  src/routing.rs                     # normalize_openai_request_payload, metadata attach
  src/route_{policy,scoring,selection,sessions}.rs
  src/state_{mutation,runtime,io}.rs
  src/service.rs                     # RoutingService construct + route() entry
  src/adaptive.rs                    # feature="adaptive" only
crates/manwe/
  Cargo.toml                         # + manwe-routing dep, features=[adaptive]
  src/main.rs                        # unchanged default path; opt-in adaptive router
  src/routing_adapter.rs             # bridges manwe AppState -> manwe-routing
```

Keep `manwe/src/main.rs` default path byte-for-byte compatible (frozen contract).
Add an `adaptive` feature; when off, manwe behaves exactly as today.

## 4. Step-by-step

Phase 0 — unblock the build
  1. `cargo build -p arda-engine` to confirm the `pub use annunimas_charon` failure.
  2. Remove or `#[cfg(...)]`-guard that line in `crates/engine/src/lib.rs` (it is
     dead — nothing in engine consumes `charon`).

Phase 1 — stand up a compilable routing lib
  3. Create `crates/manwe-routing` as a workspace member.
  4. Apply §2 dependency resolution (recommend Option 2 trait shims).
  5. Port `types.rs` + `routing.rs` + `route_*` + `state_*` first (pure logic,
     no transport). Get `cargo test -p manwe-routing` green with unit tests.
  6. Port `service.rs` `RoutingService` over the shims.

Phase 2 — wire behind a feature
  7. Add `features = { adaptive = [] }` to `manwe-routing` and `manwe`.
  8. Implement `src/routing_adapter.rs` translating manwe `ChatRequest` →
     `CharonRequestEnvelope` and the `RouteDecision` back to an upstream call.
  9. Gate the adapter path in `main.rs` on `cfg!(feature = "adaptive")`; default
     build stays the static proxy.

Phase 3 — transport (optional, later)
 10. Port `transport/http.rs` + `transport/ipc.rs` behind `adaptive` so charon's
     `/health`, `/providers`, `/probe`, `/observability` surfaces exist only when
     the feature is on. Do NOT add new ports to the default 7171 contract.

Phase 4 — verify & doc-fix
 11. `cargo build --workspace` and `cargo test --workspace` green.
 12. Correct charon's INDEX.md/README path drift (§0.5) or replace with a pointer
     to this plan. Update REFACTOR_PLAN.md to record that adaptive routing has
     landed behind `manwe`'s `adaptive` feature.

## 5. Risks / conflicts

- **Frozen-contract violation (HIGH):** full merge changes 7171 semantics
  (circuit-breaking, payload normalization, prefix handling). Mitigate: feature
  gate; default build identical to today.
- **Stub deps (HIGH):** 4 `annunimas_*` crates lack real impls in this tree →
  charon won't compile without §2 resolution. Mitigate: Option 2 shims first.
- **Behavioral divergence (MED):** charon strips `provider/` prefix and rewrites
  `model`; manwe already strips prefix but does not normalize payloads. When
  adaptive is on, request/response shapes must still satisfy OpenAI-compat
  callers (Hermes). Add contract tests.
- **Test transfer (MED):** charon's 169 tests rely on the full Annunimas fixture
  and `/var/home/mythos/Annunimas` config paths; they will not run as-is. Re-base
  on manwe's `manwe.toml` shape.
- **Doc drift (LOW):** charon INDEX/README point at non-existent paths; will
  mislead the next merge attempt. Fix in Phase 4.
- **engine dangling use (LOW but build-breaking):** must be removed/guarded before
  any `cargo build` of the workspace succeeds.

## 6. Open questions for the user

- Q1: Port governance to `arda-core` (Option 1) now, or ship shims first (Option 2)?
- Q2: Is `manwe` the intended permanent home, or should routing live in
  `crates/spine/routing/charon` with manwe as one consumer?
- Q3: Vendor the real `annunimas_*` crates from `~/Annunimas` (Option 3)?
  (Not recommended without explicit go-ahead — large external surface.)
