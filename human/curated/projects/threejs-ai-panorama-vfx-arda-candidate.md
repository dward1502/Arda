---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "athena_curated_intake"
  owner: "ATHENA"
  status: "active"
  last_reviewed: "2026-05-26"
athena:
  source_path: "human/inbox/ideas/creative-media/Untitled.md"
  intake_date: "2026-05-26"
  category: "creative_project_signal"
  promotion_status: "promoted_feasibility_candidate"
  tags: ["threejs", "arda", "webgl", "vfx", "creative-tools"]
  phase7_boundary: "phase_7_not_reopened"
  rationale: "Creative/WebGL signal; ARDA-adjacent only after source inspection and native WebKit/Tauri feasibility pass."
---

> 🜏 Soterion: 📜 athena_curated_intake | owner: ATHENA | status: active | reviewed: 2026-05-26

# Three.js AI panorama and VFX ARDA candidate

## ATHENA classification

- Source path: `human/inbox/ideas/creative-media/Untitled.md`
- Category: `creative_project_signal`
- Promotion status: `promoted_feasibility_candidate`
- Rationale: Creative/WebGL signal; ARDA-adjacent only after source inspection and native WebKit/Tauri feasibility pass.
- Phase boundary: Phase 7 remains closed; this is human-vault intake / Phase 8+ autonomy planning material where applicable.

## Original imported content

Alright, Codex with GPT 5.5 is completely cracked. This is nuts  Basically one-shotted my request to create an app that takes a prompt, creates an equirectangular panorama with GPT image 2 – and then use Apple's ML Sharp to stitch a gaussian splat world together.

Introducing Plume, Niagra for [@threejs](https://x.com/threejs)
[X link](https://x.com/ThetaForgeCo/status/2059280894678966303)
A modern vfx library for your [#threejs](https://x.com/hashtag/threejs?src=hashtag_click) games. - Pure GPU particle systems via WebGPU + TSL. No CPU fallback, no JS per-particle math. Scale to hundreds of thousands of particles without burning the main thread. - each emitter is a SoA storage buffer + two compute kernels (spawn/update) wired together from composable modules — spawn rate, init velocity, gravity, drag, color-over-life, etc. Add modules, get behavior. Remove them, get less. - What makes it different: events. When a particle dies, it atomically appends to an event buffer. Another emitter can consume those events as spawn triggers. Fireworks, debris, compound effects — chained on the GPU with no round-trip. I will be on vacation starting today, I will release upon returning May 2nd so expect a first release first week or so in May.

3d model pipeline forge-mind
the AI to 3D pipeline in 2026: - generate concept art (GPT image, nano banana) - image to 3D mesh (hunyuan3D, tripo, meshy) - cleanup + rig (meshyai, tripo, blender) - auto-animate (mixamo) start to game-ready character in one afternoon.
