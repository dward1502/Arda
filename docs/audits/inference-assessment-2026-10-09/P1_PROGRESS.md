---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Baseline and isolated runtime progress

Partial assessment, not promotion or task-level acceptance. Source changes and
evidence are uncommitted. Independent sampler review findings were repaired;
follow-up review found no blocking regressions or new findings.

## Operator scope

Priorities: coding, research, then voice to avoid typing. Operator set no
explicit resource/download/latency limits and authorized temporarily stopping
one server model while preserving Core and Warden. This is experiment authority,
not authority to replace the whole fleet, reboot hosts, or promote an unreviewed
model. No numeric promotion threshold has been claimed.

## Superseding qualification

[CUDA, Saluki and voice continuation](P2_P3_PROGRESS.md) corrects the original
new-runtime GPU claim: CUDA libraries were not discoverable, so it fell back to
CPU. Corrected GPU reruns and partial Saluki/native-tool results are retained
there. PortAudio is now available through a scoped launcher and synthetic STT
works; genuine spoken-input acceptance remains open.

## Executed observations

- [Current-worker smoke](p1-baseline-smoke.json): three synthetic exact-response
  checks per model. Bonsai passed coding and abstention but failed the precise
  citation response; Nanbeige passed research and abstention but failed coding.
  These are deliberately tiny endpoint probes, not representative benchmarks.
- [Rollback capture](p1-server-rollback.json): existing server unit contents,
  launch scripts, GPU identity, Nanbeige weight hash and downloaded archive hashes.
- Upstream llama.cpp b11430 CUDA 12.8 binaries installed side-by-side under
  `/var/home/annunimas-server/inference-assessment/b11430/`.
  Both archive SHA256 values matched GitHub release API digests. Executable
  reports build 11430 / commit 8345f3339. Existing binaries were not overwritten.
- [Historical runtime smoke](p2-runtime-smoke.json): Nanbeige Q4_K_M; GPU0 was requested, but the new runtime actually ran on CPU. Both used
  context 4096, concurrency 1, q4_0 KV, thinking disabled. Old fork passed two
  of three checks; b11430 passed all three. This is insufficient to establish
  a runtime winner, parser correctness, long-context stability or speed gain.
  Exact commands, raw responses and journals are retained. The existing GPU1
  worker remained available; machine-wide load was not isolated.
- [Experiment script](runtime-smoke.py) used a loopback-only listener, bounded
  transient unit and independent rollback timer. Bonsai was stopped only for
  the experiment, then restored. Temporary unit and timer were read back inactive.
- [Restored health](p2-restored-health.json): Core, Warden and both server workers
  returned healthy; API model inventories were read back. Both server services
  were active with zero restart counts. No model weights or routing changes.

## Resource measurement repair

The existing `scripts/benchmark_local_model.py` now records NVIDIA UUIDs and
separate per-device VRAM/GTT observations. Multi-device legacy aggregate values
are null rather than the first GPU or a fictitious combined allocation pool.
GTT is explicitly host-backed and must not be added to system RAM usage.
Whole-device measurements do not claim process attribution or idle headroom.

Ten focused regression tests in `scripts/test_benchmark_local_model.py` pass.
Independent review exposed a false pass when device visibility changed; new RED
fixtures reproduced it before repair. Legacy memory gating now requires one
fully observed device throughout the run. Multi-device or missing observations
are explicitly `unknown` and cannot pass a positive memory threshold; no summed
capacity policy is implied. NVIDIA fixtures are host-independent, with separate
mixed-vendor coverage. [Live server sampling](p1-server-memory.json) observed
both distinct GPU UUIDs before this gate repair. Follow-up independent review
passed all ten tests and additional missing-observation/identity-replacement
probes, with no blocking findings. This qualifies the bounded accounting repair,
not the full benchmark or model allocation.

## Voice path

Installed Hermes audio-io extra through `hermes pm install --extra audio-io`.
Verified using the actual launcher bootstrap, not the obsolete checkout venv.
The configured STT provider is OpenAI through the gateway; provider configuration
is recognized, but transcription has not been exercised.

[Voice preflight](p1-voice-preflight.txt) fails because this immutable Bluefin host
lacks the PortAudio native library. No microphone recording, OS layering/reboot,
STT provider change or claim of working hands-free interaction was made. Native
library availability and a genuine spoken-input round trip remain required.

## Next dependency-ready work

1. Sampler review is resolved; preserve unknown multi-device gating until an
   explicit device-selection/headroom policy is implemented and tested.
2. Extend the retained baseline to native tools/streaming and real Hermes/Arda
   parser paths; pin template provenance and retain required manifest fields.
3. Resolve PortAudio through a supported host or isolated dependency path,
   then verify actual user speech. Do not treat synthetic audio as user acceptance.
4. Only then use the reserved server lane for Saluki text experiments and larger
   coding/research fixtures. P1/P2 remain open; no candidate is promoted.
