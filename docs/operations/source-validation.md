# Source validation

The `Source Validation` workflow checks Rust and UI code independently of
documentation health and release signing. It runs on pull requests, main,
`reliability/**` branches and manual dispatch. Jobs have timeouts, two Rust build
jobs, serial Rust test execution and cancellation of superseded branch runs.

## Linux prerequisites

The supported CI build environment is Ubuntu 24.04, Rust 1.98.0, Node 22 and
pnpm 10.8.1. Run `sudo bash scripts/ci/install-linux-deps.sh` on that environment.
The package list includes GTK **3**, WebKitGTK **4.1**, OpenSSL, librsvg,
AppIndicator, xdo, protoc, a C/C++ compiler and Xvfb. GTK 4/WebKit 6 does not
provide the pkg-config interfaces required by this source tree.
See [Tauri's upstream prerequisites](https://v2.tauri.app/start/prerequisites/).

For Bluefin/immutable hosts, use the reusable development container:

```sh
podman build -t localhost/arda-native-check:ubuntu24.04 -f scripts/ci/Containerfile scripts/ci
bash scripts/ci/native-check.sh
bash scripts/ci/native-check.sh xvfb-run -a cargo test --workspace --all-targets --all-features --locked --offline -- --test-threads=1
bash scripts/ci/native-check.sh cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

The wrapper uses the installed Rust toolchain and existing Cargo registry/git
cache. Fetch locked dependencies before an offline run if they are absent. It
mounts the checkout and caches for compilation, uses `target/native-check`, and
does not expose GPU devices, host service sockets or a network. It does not
install or start Arda or inference. The image installs actual native libraries;
it does not bypass pkg-config or exclude launcher targets. Debug information and
incremental compilation are disabled to bound disk use. Tests must continue to
use temporary state; ignored installed qualification is not enabled here.

## Coverage and opt-in work

| Job | Compilation / analysis | Execution |
| --- | --- | --- |
| Root Rust | All workspace packages including launcher, all targets/features; strict Clippy | All non-ignored Rust targets, serially under Xvfb |
| HUD native | Separate Cargo workspace, all targets/features; strict Clippy | Non-ignored native tests under Xvfb |
| Mirromere native | Separate Cargo workspace, all targets/features; strict Clippy | Non-ignored native tests under Xvfb |
| Rust formatting | Root, HUD and Mirromere `cargo fmt --check` | No rewriting in CI |
| Launcher frontend | Frozen install, production build, oxlint | Vitest, two workers |
| HUD frontend | Frozen install, production build, oxlint | Vitest, two workers |
| Mirromere frontend | Frozen install and production build | Vitest, two workers; no lint script exists |
| Shared Mirromere UI | Frozen install and TypeScript check | Vitest, two workers; no lint script exists |

Each frontend uses its own lockfile. Existing oxlint warnings remain warnings;
errors fail the job. Clippy warnings are errors. Compilation, tests and Clippy
are separate steps so one failure does not hide the remaining checks. Ignored
tests retain their source-declared reasons; no global `--ignored` is used.

Real providers/credentials, installed systemd/keeper authority, namespace and
device qualification, browser-runtime cases marked ignored, operator-scale
soaks and consequential real-project tests remain explicit owner-run acceptance.
Fixture subprocesses invoked by ordinary parent tests are still exercised.
CI does not close M4/M5, operator acceptance, macOS/Windows packaging or deployment.

## Enforcement

On 2026-10-06, GitHub `main` protection required one approval, code-owner review
and conversation resolution, with no required status-check contexts; the ruleset
API returned an empty array. No protection settings were changed. Require checks
only after inspecting actual job names and successful remote runs. Local YAML or
a passing root build cannot stand in for native/UI jobs or remote execution.
