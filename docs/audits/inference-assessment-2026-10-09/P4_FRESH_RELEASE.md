---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Fresh release and bounded Hermes acceptance

Bundle: `/var/home/mythos/.local/state/arda/deployments/manwe-tools-20261010T062339Z`.
Binary SHA256: `fce5727e2bf66d3f821e904c2f98ee75cba936b3ebed1708b35965bc5f58b774`.
The frozen tree preserves the previous qualified candidate inputs and overlays
only current Manwe source/tests. Fresh isolated-target release build, full release
package tests and all-target Clippy (`-D warnings`) passed. Qualification manifest
and hashed logs are retained in the bundle. Independent artifact review passed
within scope: declared overlays match the five changed Manwe files, candidate
and staged executable hashes agree, gate-log and recorded dependency hashes
match, and inspected dependencies do not escape to the dirty workspace or prior
release. Parent also rechecked candidate, manifest and gate-log hashes.

Provenance limits remain explicit: installed-source equivalence is unknown;
registry/generated contents are not a hermetic input attestation. Review found
23 omitted build-script dependency-info files; their hashes are now retained in
`build-script-dep-info-supplement.json` in the bundle as a post-qualification
inventory, not retroactive contemporaneous gate binding. Unmanifested runtime
outputs exist in frozen trees but were absent from inspected dependency inputs.
The review confirmed the non-thinking failure below; it did not review the
subsequent thinking-enabled repeatability evidence.

The unchanged non-thinking Hermes smoke still refused without reading the fixture:
[preserved failure](p4-hermes-qualified-release-1.json). The simpler explicit native
tool request/continuation passed on that same binary.

A controlled isolated Hermes named provider sets
`extra_body.chat_template_kwargs.enable_thinking: true`. No prompt weakening,
forced tool choice or production configuration change was made. Upstream captures
confirm this setting survives the candidate proxy. Three fresh Hermes sessions
then each performed exactly one real read_file call, consumed the successful
result and answered `otter, 43219`. Each fixture remained unchanged and every
candidate process stopped. See [aggregate](p4-thinking-repeatability.json) and
raw `p4-hermes-qualified-thinking-{1,2,3}.json`; reproduction is
[manwe-thinking-smoke.py](manwe-thinking-smoke.py).

This establishes repeatability for the bounded synthetic read-only task under
thinking-enabled mode, not general tool reliability, full-profile acceptance,
coding safety or deployment acceptance. Some runs emit planning text before the
tool; this is not a clean-output or hidden-reasoning qualification. The CLI warns
that isolated config lacks a model section; the explicit CLI provider/model
selects the test route and upstream captures verify it. Non-thinking failures
remain retained; the experiment does not retrospectively qualify that mode.
Production defaults, running Core/Warden models and installed Manwe are unchanged.
