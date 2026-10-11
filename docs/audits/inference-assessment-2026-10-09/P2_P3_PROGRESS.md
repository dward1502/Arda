---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# CUDA runtime, Saluki and voice experiments

Partial qualification only. Nothing promoted or committed. Only Bonsai was
paused; the other server worker, Core and Warden remained available. Final
[health readback](p3-restored-health.json) returned ok for all four endpoints.

## Correction to the earlier runtime comparison

The original b11430 installation used CUDA release archives, but its launch
library path omitted the extracted cudart directory. `--list-devices` reported
no devices. The earlier `p2-runtime-smoke.json` new-runtime result was CPU
execution despite `--n-gpu-layers 999`. It is NOT a same-backend comparison and
must not support a GPU quality or speed conclusion. The first Saluki trial
likewise ran on CPU: [raw CPU evidence](saluki-cpu-smoke.json).

An identical, digest-verified CUDA 12.8 release was extracted separately under
`~/inference-assessment/b11430-cuda/` on Server. Correct library path includes
both `llama-b11430/` and `cudart-llama-b11430-bin-ubuntu-cuda-12.8-x64/`.
`--list-devices` then found both RTX 2080 SUPER devices. Trial processes exposed
only GPU0. Retained process maps now prove loaded CUDA libraries and actual
executable identity. No production unit binary or environment was changed.

Archives (same hashes as the earlier verified release API digests):

- runtime: `8f79f75093e8fb6d9c65e59167511e9547776794191073e488ce81efd5ade723`
- cudart: `0db934433d96342cf30b2c85b0673d89537b1d0d029a3bf28ccb4db68622e8ca`

Initial CUDA requests exceeded the 30-second request deadline. These remain
failed observations, not model correctness failures or completed measurements:
[control](runtime-cuda-tools.json), [Saluki](saluki-cuda-smoke.json).
A bounded retry with a 120-second request deadline succeeded. Cold-kernel/cache
warmup is a possible explanation, not an established root cause. No numeric
latency acceptance threshold was invented.

## Native tools and streamed output

[Extended runtime control](runtime-cuda-extended.json) and the old-runtime row
in [initial CUDA control](runtime-cuda-tools.json) use identical Nanbeige weights,
4096 context, one slot, q4_0 KV, thinking disabled and GPU0.

- Both pass research and absent-source abstention; both fail the exact coding
  response fixture.
- Both pass single native tool, tool selection and escaped/typed arguments.
- Both fail tool-result continuation: old runtime leaks a reasoning delimiter
  into final content; new runtime emits duplicate initial multiply calls.
- Both stream READY. Raw requests, responses and call IDs are retained.

These are endpoint/schema checks, not real routed Hermes/Arda acceptance and
not a representative coding benchmark. The runtime is not a qualified promotion.
Short two-token output timings are not sustained generation benchmarks; the
runtimes also report generation-rate denominators differently.

## Saluki text-only trial

Pinned artifact: ConwayResearch/Underdog-Saluki-27B-1.0 revision
`1336c0b5d74dfe6ad7f793577092f5effd9f97c6`,
`Underdog-Saluki-27B-1.0-IQ2-mix.gguf`. Download is stored on Server under
`~/inference-assessment/models/`. Exact downloaded hash and size, binary hashes,
embedded template, loaded libraries and arguments are retained in
[extended Saluki evidence](saluki-cuda-extended.json).

Same artifact on the old Nanbeige-compatible fork and b11430 CUDA, 40 GPU
layers, 4096 context, one slot, q4_0 KV, thinking disabled:

- Both pass all three tiny coding/research/abstention checks.
- Both pass all four native tool checks, including continuation.
- Both stream READY.
- b11430 tool checks took approximately 17–31 seconds each; old-runtime checks
  approximately 27–43 seconds. One run per configuration is not a statistically
  qualified latency result.
- Observed GPU0 occupancy was 5415 MiB old / 5435 MiB new. These are snapshots,
  NOT peaks or independently attributed process memory. GPU1 remained occupied
  by the retained production worker.

Saluki is a promising text-only candidate, not a selected deployment. Larger
held-out coding/research packs, resource/thermal sampling, context/concurrency,
thinking modes, vision and actual routing remain open. Dual-GPU comparison
still needs authority to pause both server lanes.

## Voice dependency and provider path

Installed `portaudio` using existing Homebrew, without sudo, OS layering or
reboot. User launcher `/var/home/mythos/.local/bin/hermes-voice` preserves the
managed Hermes executable and adds only scoped library/ALSA environment:

- `LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/opt/portaudio/lib` (preserves prior entries)
- `ALSA_CONFIG_PATH=/usr/share/alsa/alsa.conf`
- `ALSA_PLUGIN_DIR=/usr/lib64/alsa-lib`

The host ALSA configuration is necessary: Homebrew defaults initially exposed
raw hardware and selected the motherboard input rather than PipeWire defaults.
[Final preflight](p3-voice-pipewire.json) verifies default PipeWire-facing input
and output, native input sample rate and 16 kHz output support through the actual
Hermes managed bootstrap. No microphone was recorded.

[Synthetic speech provider test](p3-synthetic-voice.json) successfully transcribed
espeak-ng audio: “Please research local inference, and review the code changes.”
This verifies the existing OpenAI gateway STT path, not human speech acceptance.
A direct three-second silent WAV produced the hallucinated transcript “you”:
[provider/preflight evidence](p3-voice-preflight.json). Existing Hermes
`AudioRecorder.stop()` rejects recordings below its peak-RMS speech floor
(`tools/voice_mode.py`, lines 830–854 at inspection); direct transcription API
callers do not gain that recorder protection automatically.

To perform genuine interactive acceptance in a separate terminal:
`hermes-voice chat --cli`, then `/voice on`. A real operator-spoken request,
correct transcript, response and stop/cancel behavior remain required. Do not
claim the user can stop typing solely from synthetic tests.

## Remaining execution

Read-only independent workers are inspecting live config consumers/routed test
entrypoints and remaining candidate compatibility. Continue the plan from those
verified findings; do not archive it or promote based on this partial report.
