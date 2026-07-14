//! ARDA asset upgrade pipeline.
//!
//! Locates an existing GLB under the ARDA assets tree, runs a Blender-Python
//! script against it through the [`crate::tools::mcp_bridge::McpBridge`], and
//! writes the upgraded GLB plus an ARDA-contract `metadata.json` sidecar back
//! to disk.
//!
//! Ephemeral by default — every run starts from a fresh Blender scene, so
//! repeated invocations are idempotent. With `persistent: true` the current
//! scene is preserved (matches interactive BlenderMCP usage).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::tools::blender_tools::{fresh_envelope, BlenderToolRegistry};
use crate::tools::mcp_bridge::McpBridge;

// Canonical assets root is defined once in `forge::mod`. Keep this comment as
// the historical pointer in case grep finds it.

/// Caller-provided upgrade request.
#[derive(Debug, Clone)]
pub struct UpgradeSpec {
    pub asset_id: String,
    pub domain: String,
    pub assets_root: PathBuf,
    pub script: String,
    pub script_source: ScriptSource,
    pub scene_binding: String,
    pub material_family: String,
    pub persistent: bool,
    /// If false, skip the wrapper's GLB export step — for templates that
    /// only emit textures (e.g. baking) and shouldn't overwrite the mesh.
    pub export_glb: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSource {
    /// Built-in template (e.g. Grok Prompt 1).
    BuiltIn(&'static str),
    /// Raw user-supplied script file.
    File,
    /// LLM-translated from a natural-language prompt. (Future.)
    PromptTranslation,
}

/// Result of a successful upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetOutput {
    pub asset_id: String,
    pub domain: String,
    pub glb_path: PathBuf,
    pub metadata_path: PathBuf,
    pub script_source: String,
    pub idempotency_key: String,
}

/// Run an upgrade end-to-end.
pub async fn upgrade_asset(bridge: McpBridge, spec: UpgradeSpec) -> anyhow::Result<AssetOutput> {
    let asset_dir = spec.assets_root.join(&spec.domain).join(&spec.asset_id);
    let glb_path = asset_dir.join(format!("{}.glb", spec.asset_id));
    let metadata_path = asset_dir.join("metadata.json");

    if !glb_path.exists() {
        anyhow::bail!(
            "asset GLB not found at {} — forge upgrade requires an existing source mesh",
            glb_path.display()
        );
    }

    std::fs::create_dir_all(&asset_dir)?;

    // Materials output dir (used by baking templates). Always create the
    // canonical materials/<asset_id>/ path even if this run doesn't bake.
    let materials_dir = spec.assets_root.join("materials").join(&spec.asset_id);
    std::fs::create_dir_all(&materials_dir)?;
    let absolute_materials_dir = std::fs::canonicalize(&materials_dir)?;

    // Blender's CWD is wherever blender was launched from, not ours. Always
    // hand it absolute paths so import/export don't silently miss the file.
    let absolute_glb = std::fs::canonicalize(&glb_path)?;
    let full_script = wrap_script(&spec, &absolute_glb, &absolute_materials_dir);

    let registry = BlenderToolRegistry::with_defaults(bridge);
    let envelope = fresh_envelope("forge-mind", &format!("forge.upgrade.{}", spec.asset_id));
    let idempotency_key = envelope
        .idempotency_key
        .clone()
        .unwrap_or_else(|| "forge-unknown".into());

    let response = registry
        .invoke(
            "blender.execute_code",
            serde_json::json!({ "code": full_script }),
            &envelope,
        )
        .await?;

    tracing::info!(
        target: "forge.upgrade",
        asset_id = %spec.asset_id,
        domain = %spec.domain,
        "Blender execute_code returned: {}",
        response
    );

    write_metadata_sidecar(&metadata_path, &spec, &idempotency_key)?;

    Ok(AssetOutput {
        asset_id: spec.asset_id,
        domain: spec.domain,
        glb_path,
        metadata_path,
        script_source: script_source_label(spec.script_source).into(),
        idempotency_key,
    })
}

fn script_source_label(s: ScriptSource) -> &'static str {
    match s {
        ScriptSource::BuiltIn(name) => name,
        ScriptSource::File => "user-script",
        ScriptSource::PromptTranslation => "llm-prompt",
    }
}

/// Wrap the user script with ephemeral-scene setup, GLB import, and optional export.
fn wrap_script(spec: &UpgradeSpec, glb_path: &Path, materials_dir: &Path) -> String {
    let glb_str = glb_path.to_string_lossy().replace('\\', "\\\\");
    let mat_str = materials_dir.to_string_lossy().replace('\\', "\\\\");
    let asset_id_py = spec.asset_id.replace('"', "\\\"");
    let export_block = if spec.export_glb {
        format!(
            r#"
# --- forge-mind: export upgraded GLB back over the source ---
for obj in bpy.data.objects:
    obj.select_set(obj.type == 'MESH')

bpy.ops.export_scene.gltf(
    filepath=r"{glb_path}",
    use_selection=True,
    export_format='GLB',
    export_apply=True,
)
"#,
            glb_path = glb_str
        )
    } else {
        String::from(
            r#"
# --- forge-mind: GLB export skipped (template wrote textures/sidecars only) ---
"#,
        )
    };
    let reset_block = if spec.persistent {
        String::new()
    } else {
        // Idempotent reset without touching window/area state: delete all
        // existing data objects directly. wm.read_homefile resets the
        // context the blender-mcp addon was running under and breaks
        // subsequent bpy.ops calls.
        r#"
# --- forge-mind: ephemeral reset (data-only, preserves window context) ---
for _coll in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.images, bpy.data.armatures, bpy.data.cameras, bpy.data.lights):
    for _item in list(_coll):
        _coll.remove(_item)
"#
        .to_string()
    };

    format!(
        r#"
import bpy

# --- forge-mind: template inputs ---
FORGE_ASSET_ID = "{asset_id}"
FORGE_MATERIALS_DIR = r"{mat_dir}"
FORGE_GLB_PATH = r"{glb_path}"

{reset_block}

# --- forge-mind: import existing GLB ---
bpy.ops.import_scene.gltf(filepath=r"{glb_path}")

# --- user/built-in upgrade body ---
{user_script}

{export_block}
"#,
        reset_block = reset_block,
        glb_path = glb_str,
        mat_dir = mat_str,
        asset_id = asset_id_py,
        user_script = spec.script,
        export_block = export_block,
    )
}

fn write_metadata_sidecar(
    path: &Path,
    spec: &UpgradeSpec,
    idempotency_key: &str,
) -> anyhow::Result<()> {
    let source_tag = match spec.script_source {
        ScriptSource::BuiltIn(name) => format!("forge_mind_builtin_{name}"),
        ScriptSource::File => "forge_mind_user_script".into(),
        ScriptSource::PromptTranslation => "forge_mind_prompt_translation".into(),
    };

    let metadata = serde_json::json!({
        "id": format!("{}_{}", spec.domain, spec.asset_id),
        "domain": spec.domain,
        "scene_binding": spec.scene_binding,
        "material_family": spec.material_family,
        "source": source_tag,
        "license": "Internal",
        "forge": {
            "idempotency_key": idempotency_key,
            "persistent": spec.persistent,
        }
    });

    let pretty = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(path, pretty)?;
    Ok(())
}

/// Grok Prompt 1 — geometry cleanup, bevels, UVs.
pub const PROMPT_1_GEOMETRY_CLEANUP: &str = include_str!("templates/prompt_1_geometry_cleanup.py");

/// Grok Prompt 2 — Cycles texture baking to materials/.
///
/// Writes 4K Albedo/Normal/Roughness/Metalness/Emissive PNGs and skips the
/// wrapper's GLB export. Use with `--no-export-glb`.
pub const PROMPT_2_BAKING_PREP: &str = include_str!("templates/prompt_2_baking_prep.py");

/// Grok Prompt 3 — monitor-specific bezel + screen treatment.
pub const PROMPT_3_MONITOR_TREATMENT: &str =
    include_str!("templates/prompt_3_monitor_treatment.py");

/// Built-in template lookup. Returns (template_body, default_export_glb).
pub fn builtin_template(name: &str) -> Option<(&'static str, bool)> {
    match name {
        "prompt_1_geometry_cleanup" | "prompt_1" | "default" => {
            Some((PROMPT_1_GEOMETRY_CLEANUP, true))
        }
        "prompt_2_baking_prep" | "prompt_2" | "baking" => Some((PROMPT_2_BAKING_PREP, false)),
        "prompt_3_monitor_treatment" | "prompt_3" | "monitor" => {
            Some((PROMPT_3_MONITOR_TREATMENT, true))
        }
        _ => None,
    }
}
