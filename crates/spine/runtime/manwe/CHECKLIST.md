# manwe — Action Checklist

Owner: hades | Sigil: 🜏 SCROLL | Status: active
Source: ARCHITECTURE.md + BREAKDOWN.md + STATUS.md
Last reviewed: 2026-07-22

## Baseline / health
- [x] Verify `cargo check -p manwe` remains passing.
- [x] Verify `cargo test -p manwe` remains passing.
- [x] Verify `cargo fmt -p manwe -- --check` remains passing.

## Adaptive subtree
- [x] Fix `src/adaptive/service/service_events.rs` unresolved telemetry references; gate telemetry helper under `adaptive + telemetry`.
- [x] Fix `crates/spine/governance/arda-governance/src/triad.rs` missing `policy_version` and `veto` fields in `GovernanceChainResult`/`TriadResult`.
- [x] Fix `crates/spine/governance/arda-governance/src/resonance.rs` fixtures to populate new `policy_version`/`veto` fields.
- [x] Fix `crates/spine/interface/arda-orome/src/lib.rs` duplicate `InboundMessage`/`OutboundMessage` re-imports.
- [x] Verify `cargo test -p manwe --features adaptive` after adaptive fixes.
- [x] Confirm `src/types.rs` remains the single canonical domain type surface and adaptive modules re-export rather than duplicate.
- [x] Keep gRPC gated behind the `grpc` feature only.
- [x] gRPC service is implemented and compiles; make it explicitly opt-in via `--features grpc` + `--grpc` instead of retiring.

## Consumer alignment
- [x] Confirm runtime bind stays `127.0.0.1:7171` unless a coordinated consumer change occurs.
- [x] Confirm `arda-engine` supervision + `/v1/models` proxy wiring remains intact.
- [x] Confirm `arda-hud` reads `/v1/models` from manwe.
- [x] Confirm `arda-launcher` hardcoded `:7171` assumptions are intentional or updated together.
- [x] Confirm `services.toml` remains `name = "manwe"` gateway classification.

## Runtime / reliability
- [x] Add compile-time validation for provider credentials/bind to reduce runtime-only surprises.
- [x] Surface config/bind validity through `/healthz` for static and adaptive modes.
- [x] Add unit-level default forward path coverage in `main.rs` for missing-provider, malformed model, upstream non-JSON, and upstream unreachable.
- [x] Verify `cargo test -p manwe --features adaptive`; passing after arda-economics `service.rs` helper restoration.
- [x] Replace `Arc<AdaptiveRoutingAdapter>` not-wired placeholder with a real scoring pipeline or keep gated until implemented.
- [x] Confirm resource group defaults and env override knobs remain documented and usable.
- [x] Make governance/quota-mesh policy the authority used by the stable HTTP dispatch path when intended.
- [x] Streaming response quality/token receipts parity implemented in `proxy_fleet_provider` via final body receipt on streamed responses.
- [x] Confirm resource group defaults and env override knobs remain documented and usable.

## Architecture
- [x] Keep merged lib + binary surface for now; split decision is recorded.
- [x] Explicitly document gRPC as inactive by default.

## Artifact hygiene
- [x] Update `BREAKDOWN.md` `last_reviewed` after completed checklist items.
- [x] Update `STATUS.md` and `ARCHITECTURE.md` with architecture split/gRPC decision docs.
