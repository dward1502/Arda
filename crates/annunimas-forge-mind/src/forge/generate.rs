//! Prompt → asset generation via ComfyUI + Hunyuan3D-2.
//!
//! Single general-purpose entry point `generate_asset`. Caller supplies a
//! prompt, asset_id, and domain. We POST a workflow to ComfyUI, download the
//! resulting GLB (and the reference image SDXL produced), and write both into
//! ARDA's canonical asset layout with a metadata.json sidecar matching
//! `apps/arda-hud/src/assets/scene/ASSET_PIPELINE_CONTRACT.md`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::forge::RemoteWorkspaceConfig;
use crate::tools::blender_tools::{fresh_envelope, BlenderToolRegistry};
use crate::tools::comfyui::{ComfyUiClient, WorkflowOutputFile};
use crate::tools::mcp_bridge::McpBridge;

pub const DEFAULT_NEGATIVE_PROMPT: &str = concat!(
    "people, character, person, gingerbread, hand, body, face, ",
    "text, watermark, signature, blurry, low quality, low resolution, ",
    "multiple objects, scene, environment, room, cluttered background"
);

const TEMPLATE_JSON: &str = include_str!("templates/text_to_3d_hunyuan.json");

/// Caller-provided generation request.
#[derive(Debug, Clone)]
pub struct GenerateSpec {
    pub asset_id: String,
    pub domain: String,
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub assets_root: PathBuf,
    pub scene_binding: String,
    pub material_family: String,
    /// Optional override: harden these workflow knobs from the CLI.
    pub overrides: GenerateOverrides,
    /// When true (default), run topology cleanup (Prompt 1) over the produced
    /// GLB via the Blender bridge afterwards. Set false if Blender isn't up.
    pub post_cleanup_blender: bool,
}

/// Optional numeric/sampler overrides for the workflow. None ⇒ use template defaults.
#[derive(Debug, Clone, Default)]
pub struct GenerateOverrides {
    pub sdxl_seed: Option<i64>,
    pub sdxl_steps: Option<u32>,
    pub sdxl_cfg: Option<f64>,
    pub sdxl_width: Option<u32>,
    pub sdxl_height: Option<u32>,
    pub hy3d_seed: Option<i64>,
    pub hy3d_steps: Option<u32>,
    pub hy3d_guidance: Option<f64>,
    pub hy3d_octree_resolution: Option<u32>,
}

/// Result of a successful generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAsset {
    pub asset_id: String,
    pub domain: String,
    pub glb_path: PathBuf,
    pub metadata_path: PathBuf,
    pub reference_image_path: Option<PathBuf>,
    pub prompt_id: String,
    pub elapsed_secs: f64,
    pub idempotency_key: String,
}

pub async fn generate_asset(
    comfy: ComfyUiClient,
    spec: GenerateSpec,
) -> anyhow::Result<GeneratedAsset> {
    let started = Instant::now();
    let envelope = fresh_envelope("forge-mind", &format!("forge.generate.{}", spec.asset_id));
    let idempotency_key = envelope
        .idempotency_key
        .clone()
        .unwrap_or_else(|| "forge-unknown".into());

    let asset_dir = spec.assets_root.join(&spec.domain).join(&spec.asset_id);
    std::fs::create_dir_all(&asset_dir)?;

    let asset_prefix = format!("forge_{}_{}", spec.asset_id, idempotency_key);
    let ref_prefix = format!("{}_ref", asset_prefix);

    let workflow = build_workflow(&spec, &asset_prefix, &ref_prefix)?;
    tracing::info!(
        target: "forge.generate",
        asset_id = %spec.asset_id,
        domain = %spec.domain,
        addr = %comfy.base_url(),
        "submitting workflow"
    );

    let result = comfy.run(&workflow).await?;
    if !result.completed || result.status != "success" {
        let err = result.error.unwrap_or_else(|| {
            format!(
                "workflow ended with status='{}', completed={}",
                result.status, result.completed
            )
        });
        anyhow::bail!("comfyui workflow failed: {err}");
    }
    tracing::info!(target: "forge.generate", prompt_id = %result.prompt_id, "workflow complete");

    // The GLB output: Hy3DExportMesh writes directly to disk and does NOT
    // register with ComfyUI's history outputs map (unlike SaveImage). Since
    // we put a unique idempotency_key in `asset_prefix`, ComfyUI's filename
    // counter for that prefix is always `00001`. Predict the filename
    // deterministically and fetch it via `/view`.
    let predicted_glb_name = format!("{}_00001_.glb", asset_prefix);
    let glb_output = result
        .outputs
        .iter()
        .find(|(node_id, f)| node_id == "23" || f.filename.ends_with(".glb"))
        .map(|(_, f)| f.clone())
        .unwrap_or_else(|| WorkflowOutputFile {
            filename: predicted_glb_name.clone(),
            subfolder: String::new(),
            r#type: "output".to_string(),
        });

    let img_output = result
        .outputs
        .iter()
        .find(|(node_id, f)| node_id == "16" || f.filename.ends_with(".png"))
        .map(|(_, f)| f.clone());

    // Download GLB and write it to the canonical path.
    let glb_bytes = comfy.download_output(&glb_output).await.map_err(|e| {
        anyhow::anyhow!(
            "failed to download GLB '{}' from comfyui: {e} (history outputs were: {:?})",
            glb_output.filename,
            result
                .outputs
                .iter()
                .map(|(n, f)| (n.as_str(), f.filename.as_str()))
                .collect::<Vec<_>>()
        )
    })?;
    let glb_path = asset_dir.join(format!("{}.glb", spec.asset_id));
    std::fs::write(&glb_path, &glb_bytes)?;
    tracing::info!(target: "forge.generate", path = %glb_path.display(), bytes = glb_bytes.len(), "wrote GLB");

    // Save the reference image alongside as `<asset_id>_reference.png` for debugging.
    let reference_image_path = if let Some(img) = img_output {
        let bytes = comfy.download_output(&img).await?;
        let path = asset_dir.join(format!("{}_reference.png", spec.asset_id));
        std::fs::write(&path, bytes)?;
        Some(path)
    } else {
        None
    };

    let metadata_path = asset_dir.join("metadata.json");
    write_sidecar(&metadata_path, &spec, &idempotency_key, &result.prompt_id)?;

    if spec.post_cleanup_blender {
        if let Err(e) = run_blender_cleanup(&glb_path).await {
            tracing::warn!(target: "forge.generate", "post-generation Blender cleanup skipped: {e}");
        }
    }

    if crate::forge::should_materialize_arda_monitor(
        &spec.asset_id,
        &spec.scene_binding,
        &spec.material_family,
        &spec.positive_prompt,
    ) {
        if let Err(e) = crate::forge::materialize_arda_monitor(&glb_path).await {
            tracing::warn!(target: "forge.generate", "ARDA monitor materialization skipped: {e}");
        }
    }

    Ok(GeneratedAsset {
        asset_id: spec.asset_id,
        domain: spec.domain,
        glb_path,
        metadata_path,
        reference_image_path,
        prompt_id: result.prompt_id,
        elapsed_secs: started.elapsed().as_secs_f64(),
        idempotency_key,
    })
}

fn build_workflow(
    spec: &GenerateSpec,
    asset_prefix: &str,
    ref_prefix: &str,
) -> anyhow::Result<Value> {
    let mut wf: Value = serde_json::from_str(TEMPLATE_JSON)?;

    // Positive / negative text and filename prefixes — set unconditionally.
    set_input(
        &mut wf,
        "12",
        "text",
        Value::String(spec.positive_prompt.clone()),
    )?;
    set_input(
        &mut wf,
        "13",
        "text",
        Value::String(spec.negative_prompt.clone()),
    )?;
    set_input(
        &mut wf,
        "16",
        "filename_prefix",
        Value::String(ref_prefix.to_string()),
    )?;
    set_input(
        &mut wf,
        "23",
        "filename_prefix",
        Value::String(asset_prefix.to_string()),
    )?;

    // Optional overrides.
    let o = &spec.overrides;
    if let Some(v) = o.sdxl_seed {
        set_input(&mut wf, "14", "seed", Value::from(v))?;
    }
    if let Some(v) = o.sdxl_steps {
        set_input(&mut wf, "14", "steps", Value::from(v))?;
    }
    if let Some(v) = o.sdxl_cfg {
        set_input(&mut wf, "14", "cfg", Value::from(v))?;
    }
    if let Some(v) = o.sdxl_width {
        set_input(&mut wf, "11", "width", Value::from(v))?;
    }
    if let Some(v) = o.sdxl_height {
        set_input(&mut wf, "11", "height", Value::from(v))?;
    }
    if let Some(v) = o.hy3d_seed {
        set_input(&mut wf, "21", "seed", Value::from(v))?;
    }
    if let Some(v) = o.hy3d_steps {
        set_input(&mut wf, "21", "steps", Value::from(v))?;
    }
    if let Some(v) = o.hy3d_guidance {
        set_input(&mut wf, "21", "guidance_scale", Value::from(v))?;
    }
    if let Some(v) = o.hy3d_octree_resolution {
        set_input(&mut wf, "22", "octree_resolution", Value::from(v))?;
    }

    Ok(wf)
}

fn set_input(wf: &mut Value, node_id: &str, field: &str, value: Value) -> anyhow::Result<()> {
    let node = wf
        .get_mut(node_id)
        .ok_or_else(|| anyhow::anyhow!("workflow template missing node {node_id}"))?;
    let inputs = node
        .get_mut("inputs")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("node {node_id} has no inputs object"))?;
    inputs.insert(field.to_string(), value);
    Ok(())
}

fn write_sidecar(
    path: &Path,
    spec: &GenerateSpec,
    idempotency_key: &str,
    prompt_id: &str,
) -> anyhow::Result<()> {
    let metadata = serde_json::json!({
        "id": format!("{}_{}", spec.domain, spec.asset_id),
        "domain": spec.domain,
        "scene_binding": spec.scene_binding,
        "material_family": spec.material_family,
        "source": "forge_mind_comfyui_hunyuan3d",
        "license": "Internal",
        "forge": {
            "pipeline": "comfyui_text_to_3d_hunyuan",
            "positive_prompt": spec.positive_prompt,
            "negative_prompt": spec.negative_prompt,
            "idempotency_key": idempotency_key,
            "comfyui_prompt_id": prompt_id,
        }
    });
    std::fs::write(path, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

async fn run_blender_cleanup(glb_path: &Path) -> anyhow::Result<()> {
    let bridge = McpBridge::from_env();
    let registry = BlenderToolRegistry::with_defaults(bridge);
    let envelope = fresh_envelope("forge-mind", "forge.generate.post_cleanup");

    let staged = RemoteWorkspaceConfig::from_env()
        .stage_for_blender(glb_path)
        .await?;
    let glb_str = staged.blender_path.to_string_lossy().replace('\\', "\\\\");
    let code = format!(
        r#"
import bpy
for _coll in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.images, bpy.data.armatures, bpy.data.cameras, bpy.data.lights):
    for _item in list(_coll):
        _coll.remove(_item)
bpy.ops.import_scene.gltf(filepath=r"{glb}")
{cleanup}
for obj in bpy.data.objects:
    obj.select_set(obj.type == 'MESH')
bpy.ops.export_scene.gltf(filepath=r"{glb}", use_selection=True, export_format='GLB', export_apply=True)
"#,
        glb = glb_str,
        cleanup = crate::forge::PROMPT_1_GEOMETRY_CLEANUP,
    );

    registry
        .invoke(
            "blender.execute_code",
            serde_json::json!({ "code": code }),
            &envelope,
        )
        .await?;
    staged.sync_back().await?;
    Ok(())
}
