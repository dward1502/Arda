# manwe — Current Status

Crate: `crates/spine/runtime/manwe`
Reviewed: 2026-08-04
State: active; single governed runtime converged in source

## Current runtime contract

- One binary process path: governed `ManweService` HTTP transport.
- Default coordinated endpoint: `127.0.0.1:7171`; canonical workstation launch
  binds `0.0.0.0:7171` for verified Tailscale consumers.
- Root owner: `cargo run -p arda -- --no-ui` through `services.toml`.
- Root status: `http://127.0.0.1:7878/v1/status` with lifecycle/readiness,
  required/optional, PID, restart count, bounded backoff, and detail.
- Provider/state ownership: `config/manwe.providers.toml` and `data/manwe/`,
  subject to documented environment overrides.

The former static HTTP modules and standalone gRPC process were removed.
`adaptive` is the default and mandatory binary feature. The hidden `--adaptive`
flag is a compatibility no-op, not a runtime selector; `--grpc` fails clearly.

## Verification observed on 2026-08-04

| Command / probe | Result |
|---|---|
| `cargo test -p manwe --all-features -- --test-threads=1` | PASS: 279 library + 3 binary tests |
| `cargo test -p arda-engine --all-targets -- --test-threads=1` | PASS after registry expectation update |
| `cargo test --test root_daemon -- --test-threads=1` | PASS: 5 tests |
| `systemd-analyze --user verify ...` | PASS for root, Aule and inference-probe units |
| `pnpm --dir apps/arda-hud test -- --run ...` | PASS: 100 files, 392 tests |
| Temporary `target/debug/arda --no-ui` | PASS: root-owned Manwe healthy on `0.0.0.0:7171` |
| Tailscale `http://100.78.138.113:7171/healthz` | PASS during temporary root run |
| Root shutdown | PASS: root and owned Manwe child exited; production `:5110` process preserved |

The temporary canonical topology reported `service_statuses[0]` as required,
healthy, PID-owned by the root, restart count zero, and readiness-probe detail.
A supervisor regression test also forces one child failure, observes one bounded
restart, and proves clean final shutdown.

## Governed routing evidence

The maintained test suite proves:

- task-class and tool-schema classification;
- model/tool/context admission and rejection diagnostics;
- local, free-cloud, subscription, and paid candidate handling;
- lane-fitness, bandit, tool-fit and capability-receipt persistence/consumption;
- route rationale and typed governance metadata;
- bounded resource-group concurrency and independent groups;
- route/provider/model headers and auditable route/governance receipts;
- fallback, cooldown, quota and model-quirk behavior.

The process smoke uses a controlled local upstream and verifies health, models,
capabilities, chat, route headers, provider identity, route-selected state, and
governance receipts without external provider writes.

## Deployment boundary

The live operator-managed `:5110` process remains untouched until the U4 install
and recovery cutover installs the new root binary/unit and coordinates the
active session using that provider. Canonical repository consumers now target
`:7171`; historical status reports retain old `:5110` evidence but no longer
authorize launch configuration.

## Maintained gates

```text
cargo check -p manwe --all-targets --all-features
cargo clippy -p manwe --all-targets --all-features -- -D warnings
cargo test -p manwe --all-features -- --test-threads=1
cargo fmt -p manwe -- --check
python crates/spine/runtime/manwe/tests/process_smoke.py
python crates/spine/runtime/manwe/tests/check_docs.py
cargo test -p arda-engine --all-targets -- --test-threads=1
cargo test --test root_daemon -- --test-threads=1
```
