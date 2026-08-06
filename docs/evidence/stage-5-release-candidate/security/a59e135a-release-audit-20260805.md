# `a59e135a` release audit

**Audit date:** 2026-08-05
**Scope:** `a59e135a^..a59e135a`, plus corrective HEAD validation
**Disposition:** corrective source changes and local-history sanitation complete; final source verification and reliability gates remain open

## Findings

### 1. Manwe network exposure and development-process launch

`a59e135a` configured canonical Manwe as `cargo run ... --bind 0.0.0.0 --port 7171`. Manwe's HTTP mutation authorization remains optional for local compatibility, while several provider-costing routes do not use that mutation guard. The wildcard listener therefore contradicted the documented loopback trust boundary.

Corrective commit `6f5ba670`:

- changes the canonical service command to `target/release/manwe`;
- binds canonical port `7171` to `127.0.0.1`;
- rejects every non-loopback Manwe HTTP bind in the server itself;
- documents authenticated reverse proxying as the remote-access boundary;
- adds registry and bind-policy tests.

Runtime smoke evidence:

- `target/release/manwe --config manwe.toml --bind 0.0.0.0 --port 17171` exits non-zero with `refusing non-loopback HTTP bind`;
- the same release binary bound to `127.0.0.1:17171` and returned HTTP 200 from `/health`.

### 2. Generated runtime state and raw prompt persistence

`a59e135a` committed generated state under `data/governance`, `data/manwe`, `data/plutus`, and `data/prometheus`. The largest file was `data/plutus/runtime_status.json`, and rotated Bacon-lite records included raw route prompt text and local identity/path material.

Corrective commits `6f5ba670` and `707442d6`:

- ignore these four generated runtime-state directories;
- remove all 23 runtime files from the current tracked tree while preserving local working copies;
- add a Bacon-lite persistence API that evaluates the original task but stores a caller-provided safe description;
- make Manwe route receipts persist `prompt=[redacted]` rather than the request text;
- add scoring-equivalence and non-persistence regression tests.

Current-tree verification:

- `git ls-files data/governance data/manwe data/plutus data/prometheus` returns zero paths;
- representative live files in all four directories are ignored;
- the full `arda-governance` and Manwe library test suites pass.

### 3. Packaged HUD personal-operations transport

The HUD personal-operations client calls the loopback harness from a different webview origin. The harness did not emit CORS headers, so the packaged UI could be blocked even though its CSP allowed the endpoint.

Corrective commit `6f5ba670`:

- allows GET/POST preflight only from packaged Tauri origins and the two HUD development origins;
- allows only the required content, identity, and idempotency headers;
- confirms an unrelated web origin receives no `Access-Control-Allow-Origin` response.

The client still uses the generic `operator-0` identity. This is not accepted as
remote or multi-user authentication. It is bounded by the Stage 5 threat model's
explicit single-user, loopback-only profile; wellness ingestion remains deferred
under the separate Personal Operations privacy review. Remote or multi-user use
would require backend-owned identity rather than a webview-provided value.

### 4. Independent frontend, launcher, and config finding adjudication

Two read-only follow-up reviews raised three additional candidate blockers:

- Workbench rejection uses `decision: policy_safe`. This is intentional at the
  current contract boundary: the decision authorizes the cancellation mutation;
  it does not mark the rejected run node as approved. The backend rejects any
  mutation envelope whose decision is not `policy_safe`, while the durable run
  event and node transitions record `Cancelled` and the operator's rejection
  reason. No source change is required.
- The launcher profile check requires `ID=centos`, `VERSION_ID=10`, a
  `PRETTY_NAME` beginning with `Bluefin LTS`, and `x86_64`. The supported host's
  live `/etc/os-release` reports exactly `ID=centos`, `VERSION_ID=10`, and
  `PRETTY_NAME="Bluefin LTS"`; the existing supported-profile fixture exercises
  the same predicate. The candidate concern is disproven for the declared
  profile.
- Tracked deployment and fleet configuration contains checkout paths, machine
  identities, and private-network endpoints. Most examples predate `a59e135a`;
  the commit primarily migrated already-tracked Manwe endpoints from port 5110
  to 7171. They remain portability and source-publication debt. Final artifact
  lifecycle validation must prove that the signed install does not depend on
  operator-specific repository configuration, and history sanitation must not
  introduce any additional private runtime material.

## Verification performed

- `cargo fmt --all --check` — pass
- `git diff --check` — pass
- `cargo check -p arda-engine --all-targets` — pass
- `cargo test -p arda-engine --lib` — 28 passed
- `cargo test -p arda-engine --test harness_personal_ops` — 9 passed
- `cargo test -p arda-governance --lib` — pass
- `cargo test -p manwe --lib` — pass
- `cargo build -p manwe --release` — pass
- `cargo deny check advisories` — pass (`advisories ok`)
- `pnpm audit --prod` in `apps/arda-hud` — pass (`No known vulnerabilities found`)
- `pnpm test -- PersonalOperationsModule.test.tsx personalOps.test.ts` in `apps/arda-hud` — 100 files / 397 tests passed (Vitest ran the configured full suite)
- `pnpm build` in `apps/arda-hud` — pass

Existing non-fatal frontend test warnings remain visible: React `act(...)` warnings and duplicate Three.js import warnings. They did not fail the suite and were not introduced by the audited fixes.

## History sanitation and remaining gates

The ahead-only lineage was rebuilt from `origin/manwe` on 2026-08-06. The
sanitized branch retains the corrected final tree while omitting the generated
runtime directories and the inherited Annunimas queue archive. `a59e135a` is no
longer reachable from the `manwe` branch. A local backup ref retains the original
lineage for recovery and must never be pushed.

History sanitation removes this source-publication blocker but does not itself
authorize a release freeze. The completed U3 soak is a valid failed diagnostic
snapshot that predates the corrections. Final source still requires focused
verification, a complete-matrix smoke, and a fresh uninterrupted 24-hour soak.
