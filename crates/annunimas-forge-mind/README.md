---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# annunimas-forge-mind

Spawned from the Annunimas sovereign crate blueprint. Workflow + execution
surface for 3D asset production (Blender, texturing, slicing, ARDA scenes).

- Realm: `operations`
- Productizable: `true`
- Role: blueprint workflow + execution surface
- Required exports: `core/state/annunimas-forge-mind.json`
- Required hooks: task ledger, ARDA visibility, Soterion trace, governance validators, memory checkpoint capture

## Baseline

- crate contract in `src/contract.rs`
- workflow planning primitives in `src/workflow.rs`
- service status in `src/service.rs`
- governance smoke test in `tests/contract_smoke.rs`
- BlenderMCP bridge + governed tool registry in `src/tools/` (feature-gated)

## Usage

```rust
use annunimas_forge_mind::service::workflow_plan;
use annunimas_forge_mind::workflow::{ArtifactPolicy, EngineeringDomain, ForgeWorkItem};

let item = ForgeWorkItem {
    domain: EngineeringDomain::SoftwareSystems,
    description: "demo".into(),
    has_research: true,
    has_build_artifact: false,
    target_output: vec![],
};

let plan = workflow_plan(&item);
assert_eq!(plan.artifact_policy, ArtifactPolicy::PrototypeAllowed);
```

## BlenderMCP bridge (`mcp-bridge` feature)

A TCP bridge to the [ahujasid/blender-mcp](https://github.com/ahujasid/blender-mcp)
Blender addon lives under `src/tools/`:

- `mcp_bridge.rs` — raw async transport (`tokio::net::TcpStream`) speaking the
  addon's `{"type": "...", "params": {...}}` wire format on `127.0.0.1:9876`.
- `blender_tools.rs` — `BlenderToolRegistry` that pairs each command with an
  `annunimas-tool-harness` `ToolMetadata` so every call passes through the
  governed-invocation gate (idempotency for mutating tools, operator review for
  Critical risk, soterion-trace validator set, etc.).

Default registry tools:

| tool id                              | risk    | side effect | notes |
|--------------------------------------|---------|-------------|-------|
| `blender.get_scene_info`             | Low     | ReadOnly    | scene graph snapshot |
| `blender.get_object_info`            | Low     | ReadOnly    | per-object inspection |
| `blender.get_viewport_screenshot`    | Low     | ReadOnly    | PNG bytes |
| `blender.get_polyhaven_status`       | Low     | ReadOnly    | addon flag |
| `blender.execute_code`               | High    | Mutating    | arbitrary Python in Blender — needs idempotency key |
| `blender.download_polyhaven_asset`   | Medium  | Mutating    | needs idempotency key |

Native `blr` is still the preferred path; the bridge is the fallback when you
need anything the native crate doesn't cover (Polyhaven, Hyper3D, addon-side
operators).

### Build

```bash
cargo build -p annunimas-forge-mind --features mcp-bridge
# or both at once
cargo build -p annunimas-forge-mind --features "native-blender mcp-bridge"
```

The bridge has no extra dependencies beyond what forge-mind already pulls in
(tokio, serde, anyhow, annunimas-tool-harness).

### Calling a tool

```rust
use annunimas_forge_mind::tools::blender_tools::{fresh_envelope, BlenderToolRegistry};
use annunimas_forge_mind::tools::mcp_bridge::McpBridge;
use serde_json::json;

let registry = BlenderToolRegistry::with_defaults(McpBridge::from_env());

let envelope = fresh_envelope("forge-mind", "trace-abc"); // supplies idempotency key
let scene = registry
    .invoke("blender.get_scene_info", json!({}), &envelope)
    .await?;

let _ = registry
    .invoke(
        "blender.execute_code",
        json!({ "code": "import bpy; bpy.ops.mesh.primitive_cube_add()" }),
        &envelope,
    )
    .await?;
```

`McpBridge::from_env()` reads:

- `BLENDER_MCP_ADDR` — `host:port` (default `127.0.0.1:9876`)
- `BLENDER_MCP_TIMEOUT_SECS` — per-call timeout (default `30`)

## ARDA Boardroom Model-Build Runbook

Use this flow for high-fidelity ARDA boardroom assets such as
`upper_monitor_1`, `desk_left_surface`, `center_console`, and `presence_rig`.

### Machine split

The laptop is suitable for:

- reading specs and references
- normalizing metadata stubs
- editing prompts
- reviewing generated GLBs after sync
- running frontend/Tauri builds in `moria`

The workstation / `annunimas-server` is required for the full build flow:

- ComfyUI + Hunyuan3D-2 text-to-3D generation at `http://annunimas-server:8188`
- vision feedback endpoint at `http://annunimas-server:8081`
- Blender with the BlenderMCP addon listening on `127.0.0.1:9876`
- enough GPU/VRAM for model generation and render/vision loops

Do not run the full generate/iterate loop from the laptop unless those
endpoints route to the workstation and the output path is mounted/synced.

### Source pack

Current boardroom source pack:

```text
human/inbox/arda_boardroom_spec_pack_v0_2/
```

Important files:

- `ARDA_BOARDROOM_MASTER_ASSET_SPEC.md`
- `ARDA_BOARDROOM_BUILD_WORKSHEET.md`
- `ARDA_BOARDROOM_TOOL_PIPELINE.md`
- `ARDA_BOARDROOM_ASSET_INTAKE_INDEX.md`
- `ARDA_BOARDROOM_ASSET_TRACKER.csv`
- `metadata_stubs/`

The first recommended production target is `upper_monitor_1`, using:

- `monitors.jpg`
- `dualArmmonitorL+R.jpg`
- `closeupMonitorJoints.jpg`

### Workstation startup checklist

On the workstation:

1. Start or verify `annunimas-server` DNS/host routing.
2. Start ComfyUI with the Hunyuan3D workflow dependencies loaded.
3. Start the vision LLM server:

   ```bash
   curl -s http://annunimas-server:8081/v1/models
   ```

4. Start Blender, enable the BlenderMCP addon, and click **Connect to MCP server**.
5. Verify the bridge:

   ```bash
   nc 127.0.0.1 9876 <<< '{"type":"get_scene_info","params":{}}'
   ```

6. From the Annunimas repo, check Forge-Mind endpoint resolution:

   ```bash
   source scripts/runtime_build_env.sh
   cargo run -p annunimas-cli -- forge status
   ```

### Generate a first candidate GLB

Use `forge generate` when you want one text-to-3D candidate and optional
Blender cleanup.

Recommended first target:

```bash
source scripts/runtime_build_env.sh
cargo run -p annunimas-cli -- forge generate \
  "isolated premium cyber-noir articulated monitor arm and rugged graphite display housing, compact rotary joints, cyan accent strips, refined beveled hard-surface design, centered product render, 3D model, single object, no humans, no environment" \
  --asset-id upper_monitor_1 \
  --domain world \
  --scene-binding upper_monitor_1 \
  --material-family boardroom_monitor_bezel \
  --comfyui-addr http://annunimas-server:8188 \
  --hy3d-octree 384 \
  --post-cleanup
```

Expected output:

```text
apps/arda-hud/src/assets/scene/world/upper_monitor_1/
  upper_monitor_1.glb
  upper_monitor_1_reference.png
  metadata.json
```

### Run the vision-feedback iterate loop

Use `iterate` when you want ComfyUI generation, Blender angle renders, vision
comparison against a reference image, and best-candidate promotion.

Example:

```bash
source scripts/runtime_build_env.sh
cargo run -p annunimas-cli -- iterate \
  human/inbox/arda_boardroom_spec_pack_v0_2/monitors.jpg \
  --asset-id upper_monitor_1 \
  --domain world \
  --scene-binding upper_monitor_1 \
  --material-family boardroom_monitor_bezel \
  --comfyui-addr http://annunimas-server:8188 \
  --vision-addr http://annunimas-server:8081 \
  --budget-iters 3 \
  --accept-threshold 0.82 \
  "isolated premium cyber-noir articulated monitor arm and rugged graphite display housing, compact rotary joints, cyan accent strips, refined beveled hard-surface design, centered product render, 3D model, single object, no humans, no environment"
```

Expected output includes:

```text
apps/arda-hud/src/assets/scene/world/upper_monitor_1/
  upper_monitor_1.glb
  iterate_summary.json
  iterations/
```

### Upgrade or clean an existing GLB through BlenderMCP

Use `forge upgrade` after a GLB exists.

For monitor-specific treatment:

```bash
source scripts/runtime_build_env.sh
cargo run -p annunimas-cli -- forge upgrade upper_monitor_1 \
  --domain world \
  --template prompt_3 \
  --scene-binding upper_monitor_1 \
  --material-family boardroom_monitor_bezel
```

For generic geometry cleanup:

```bash
cargo run -p annunimas-cli -- forge upgrade upper_monitor_1 \
  --domain world \
  --template prompt_1 \
  --scene-binding upper_monitor_1 \
  --material-family boardroom_monitor_bezel
```

### Promotion rule

Generated assets are allowed to land in the ARDA runtime asset tree only when
all of these are true:

- the asset id and domain match the boardroom tracker
- `metadata.json` uses a material family from `MATERIAL_CONTRACT.md`
- the reference source is captured in metadata or the source pack index
- the asset builds with `npm run build` in `apps/arda-hud`
- the Tauri build still succeeds in `moria` or on the workstation

For AI-generated GLBs, keep `license: "Internal"` unless a separate release
review decides otherwise.

## Setting up the Blender side

You only need the **Blender addon** from upstream — not the Python MCP server,
since forge-mind talks to the addon's socket directly.

1. Clone or download <https://github.com/ahujasid/blender-mcp>.
2. In Blender: `Edit → Preferences → Add-ons → Install…`, pick
   `addon.py` from the repo, and enable **Interface: Blender MCP**.
3. In the 3D Viewport, open the side panel (press `N`), find the **BlenderMCP**
   tab, and click **Connect to MCP server**. This opens the listening socket on
   `127.0.0.1:9876`.
4. (Optional) Enable Polyhaven/Hyper3D toggles in the same panel if you want
   those tools to succeed.

Verify with:

```bash
nc 127.0.0.1 9876 <<< '{"type":"get_scene_info","params":{}}'
```

You should get a JSON object back with `"status": "success"`.

## Extension Points

- Add concrete artifact builders only after an owning runtime path exists.
- Preserve research before build and verification after build.
- Keep governance validators attached to workflow plans before promotion.
- New blender-mcp commands: add a typed wrapper to `McpBridge`, then register
  it in `BlenderToolRegistry::register_defaults` with the right risk/side-effect
  class.
