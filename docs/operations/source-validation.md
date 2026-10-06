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
bash scripts/ci/native-check.sh cargo test --workspace --all-targets --all-features --locked --offline --no-fail-fast -- --test-threads=1
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
The container unit/integration suite runs directly: the local Xvfb wrapper stalled
before launching Cargo. GitHub's Ubuntu runner supports Xvfb and uses it there.
The container runs as the host user with an init process, and binds its temporary
directory from the checkout's filesystem (ext4/XFS/Btrfs required for keeper
tests). Proc masking is removed inside this rootless container so nested
bubblewrap can establish its own proc mount. Node and bubblewrap are explicit
test prerequisites. No `--privileged`, host namespace or inference device is used.

The root CI job temporarily permits unprivileged user namespaces on its disposable
Ubuntu VM, verifies a real bubblewrap launch, and restores the prior setting in
an always-run step. This addresses Ubuntu's
[AppArmor namespace restriction](https://documentation.ubuntu.com/release-notes/24.04/)
without modifying the operator host. Ordinary isolated namespace fixtures run;
tests declared ignored for installed authority or additional qualification remain
opt-in.

Both `/tmp` and `/var/tmp` are backed by task-owned directories on the qualified
host filesystem. The latter is explicit because keeper tests intentionally use
`/var/tmp` for durable state and `/dev/shm` for ephemeral endpoints.

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

HUD has narrowly documented lint expectations for its staged, unregistered
personal-operations/research command modules and two existing Tauri commands
whose named IPC argument lists must remain compatible with the frontend.
These expectations do not register commands or expand execution authority.
Unused monitor-registry helpers are compiled only for their existing tests.
The rest of the native warning baseline is enforced, including root/launcher.

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

## October 6, 2026 validation

Source implementation: `ee6d0d1833824b3a163ba96c6711d0a76bc222c4` on
`reliability/r03-native-ci`. The local checkout remains on `f5c8a361` with operator
work preserved; the 38 modified Rust files match the committed validation branch.

- Full root all-target/all-feature compilation passes with launcher included.
- Full root tests, `--workspace --all-targets --all-features --locked --offline
  --no-fail-fast -- --test-threads=1`: **2,378 passed, 0 failed, 61 ignored**,
  across 179 target summaries. The ignored tests retain their declared opt-in
  requirements; no installed provider/keeper qualification is inferred.
- Frontend tests: launcher 20, HUD 623, Mirromere 6, shared UI 11 passed.
  All app builds and shared UI typecheck passed. Existing frontend lint warnings
  remain visible; both available lint scripts return success.
- Full root all-target/all-feature Clippy passes with `-D warnings`, including
  launcher. Root/HUD/Mirromere formatting passes. Local evidence and exact ignored-test
  names are under `target/qualification/reliability-20261006/`, including
  `manifest.json`, `full-workspace-tests-final.log`, and `ignored-tests.txt`.
- [Final remote matrix](https://github.com/dward1502/Arda/actions/runs/37438628947)
  passed all eight jobs on the clean checkout, including root tests/strict Clippy,
  both separate native workspaces and all four frontend/shared-UI jobs. Runner
  namespace policy restoration also passed. The branch is not merged; required
  status checks and installed acceptance remain unchanged.

The baseline repairs include two stale fixture initializers, shutdown tests that
previously polled an already-closed HTTP server, and a crash-recovery fixture that
retained a raw SQLite observer across process interruption. Production changes
are limited to contextual authority-open errors, a typed retirement request with
unchanged authority checks, and admission of read-only projects with no executable
checks. A regression verifies that the last case remains pending approval and
does not allow unchecked execution metadata. No installed binary or historical
authority database was repaired or replaced by this validation task.
