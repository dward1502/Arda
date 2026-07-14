//! Procedural ARDA style materialization passes for generated GLB assets.
//!
//! Text-to-3D gives Forge-Mind a coarse silhouette. These passes enforce the
//! ARDA visual language after generation, before render/vision scoring.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::forge::RemoteWorkspaceConfig;
use crate::tools::blender_tools::{fresh_envelope, BlenderToolRegistry};
use crate::tools::mcp_bridge::McpBridge;

const ARDA_MONITOR_MATERIALIZATION: &str =
    include_str!("templates/arda_monitor_materialization.py");

pub const ARDA_DESK_MATERIALIZATION: &str = include_str!("templates/arda_desk_materialization.py");
const MATERIALIZATION_SCHEMA_VERSION: u32 = 1;
const MONITOR_MATERIALIZER: &str = "arda_monitor_materialization";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlenderExecutionBackend {
    BlenderMcp,
    RemoteBlenderCliFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlbMaterializationInspection {
    pub node_count: usize,
    pub mesh_count: usize,
    pub material_count: usize,
    pub primitive_count: usize,
    pub emissive_material_count: usize,
    pub arda_procedural_nodes: usize,
    pub duplicate_arda_procedural_nodes: BTreeMap<String, usize>,
    pub overlay_alignment: Option<ArdaOverlayAlignmentInspection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArdaOverlayAlignmentInspection {
    pub screen_node_present: bool,
    pub bezel_node_count: usize,
    pub trace_node_count: usize,
    pub inferred_depth_axis: Option<String>,
    pub max_overlay_plane_separation: Option<f64>,
    pub screen_thin_axis: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializationReport {
    pub materializer: String,
    pub backend: BlenderExecutionBackend,
    pub fallback_reason: Option<String>,
    pub glb_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
    pub source_sha256: String,
    pub output_sha256: String,
    pub remote_host: Option<String>,
    pub blender_path: PathBuf,
    pub inspection: GlbMaterializationInspection,
}

/// Apply the ARDA monitor house style to a generated monitor GLB.
///
/// The pass is intentionally procedural and conservative: import the GLB,
/// convert existing mesh materials to dark graphite, then add cyan emissive
/// screen/edge-strip geometry and compact mount hardware when the generator did
/// not provide recognizable named subparts.
pub async fn materialize_arda_monitor(glb_path: &Path) -> anyhow::Result<MaterializationReport> {
    let source_sha256 = sha256_file(glb_path)?;
    let bridge = McpBridge::from_env();
    let registry = BlenderToolRegistry::with_defaults(bridge);
    let envelope = fresh_envelope("forge-mind", "forge.materialize.arda_monitor");

    let staged = RemoteWorkspaceConfig::from_env()
        .stage_for_blender(glb_path)
        .await?;
    let glb_str = staged.blender_path.to_string_lossy().replace('\\', "\\\\");
    let code = monitor_materialization_script(&glb_str);

    let invoke_result = registry
        .invoke(
            "blender.execute_code",
            serde_json::json!({ "code": code }),
            &envelope,
        )
        .await;

    let mut backend = BlenderExecutionBackend::BlenderMcp;
    let mut fallback_reason = None;
    if let Err(err) = invoke_result {
        let msg = err.to_string();
        if staged.has_remote_sync() {
            tracing::warn!(
                target: "forge.materialize",
                error = %msg,
                "BlenderMCP unavailable or unable to export GLB; falling back to remote Blender CLI"
            );
            backend = BlenderExecutionBackend::RemoteBlenderCliFallback;
            fallback_reason = Some(msg);
            staged
                .run_remote_blender_script("arda_monitor_materialization.py", &code)
                .await?;
        } else {
            return Err(err);
        }
    }

    staged.sync_back().await?;
    let output_sha256 = sha256_file(glb_path)?;
    let inspection = inspect_glb_materialization(glb_path)?;
    ensure_clean_arda_materialization(&inspection)?;
    let metadata_path = metadata_path_for_glb(glb_path);
    if metadata_path.exists() {
        update_materialization_sidecar(
            &metadata_path,
            MONITOR_MATERIALIZER,
            &backend,
            fallback_reason.as_deref(),
            &source_sha256,
            &output_sha256,
            staged.sync_host(),
            &staged.blender_path,
            &inspection,
        )?;
    }

    Ok(MaterializationReport {
        materializer: MONITOR_MATERIALIZER.to_string(),
        backend,
        fallback_reason,
        glb_path: glb_path.to_path_buf(),
        metadata_path: metadata_path.exists().then_some(metadata_path),
        source_sha256,
        output_sha256,
        remote_host: staged.sync_host().map(str::to_string),
        blender_path: staged.blender_path,
        inspection,
    })
}

fn monitor_materialization_script(glb_str: &str) -> String {
    format!(
        r#"
import bpy
for _coll in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.images, bpy.data.armatures, bpy.data.cameras, bpy.data.lights):
    for _item in list(_coll):
        _coll.remove(_item)
bpy.ops.import_scene.gltf(filepath=r"{glb}")
{materialize}
for obj in bpy.data.objects:
    obj.select_set(obj.type == 'MESH')
    if obj.type == 'MESH':
        bpy.context.view_layer.objects.active = obj
bpy.ops.export_scene.gltf(filepath=r"{glb}", use_selection=True, export_format='GLB', export_apply=True)
"#,
        glb = glb_str,
        materialize = ARDA_MONITOR_MATERIALIZATION,
    )
}

fn metadata_path_for_glb(glb_path: &Path) -> PathBuf {
    glb_path
        .parent()
        .map(|parent| parent.join("metadata.json"))
        .unwrap_or_else(|| PathBuf::from("metadata.json"))
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open {} for sha256: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| anyhow::anyhow!("failed to read {} for sha256: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn inspect_glb_materialization(path: &Path) -> anyhow::Result<GlbMaterializationInspection> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read GLB {}: {e}", path.display()))?;
    inspect_glb_materialization_bytes(&bytes)
}

pub fn inspect_glb_materialization_bytes(
    bytes: &[u8],
) -> anyhow::Result<GlbMaterializationInspection> {
    let json = glb_json(bytes)?;
    Ok(inspect_gltf_json(&json))
}

fn glb_json(bytes: &[u8]) -> anyhow::Result<Value> {
    if bytes.len() < 20 {
        anyhow::bail!("GLB too small to contain header and JSON chunk");
    }
    if &bytes[0..4] != b"glTF" {
        anyhow::bail!("GLB magic mismatch");
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into()?);
    if version != 2 {
        anyhow::bail!("unsupported GLB version {version}");
    }
    let declared_len = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    if declared_len != bytes.len() {
        anyhow::bail!(
            "GLB length mismatch: declared {declared_len}, actual {}",
            bytes.len()
        );
    }
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let chunk_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        offset += 8;
        if offset + chunk_len > bytes.len() {
            anyhow::bail!("GLB chunk length exceeds file boundary");
        }
        if chunk_type == b"JSON" {
            return serde_json::from_slice(&bytes[offset..offset + chunk_len])
                .map_err(|e| anyhow::anyhow!("failed to parse GLB JSON chunk: {e}"));
        }
        offset += chunk_len;
    }
    anyhow::bail!("GLB JSON chunk not found")
}

fn inspect_gltf_json(json: &Value) -> GlbMaterializationInspection {
    let node_names = json
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|node| node.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut arda_counts = BTreeMap::new();
    for name in node_names.iter().filter(|name| name.starts_with("arda_")) {
        *arda_counts.entry(name.clone()).or_insert(0) += 1;
    }
    let duplicate_arda_procedural_nodes = arda_counts
        .iter()
        .filter_map(|(name, count)| (*count > 1).then_some((name.clone(), *count)))
        .collect::<BTreeMap<_, _>>();
    let primitive_count = json
        .get("meshes")
        .and_then(Value::as_array)
        .map(|meshes| {
            meshes
                .iter()
                .map(|mesh| {
                    mesh.get("primitives")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0);
    let emissive_material_count = json
        .get("materials")
        .and_then(Value::as_array)
        .map(|materials| {
            materials
                .iter()
                .filter(|material| material.get("emissiveFactor").is_some())
                .count()
        })
        .unwrap_or(0);

    GlbMaterializationInspection {
        node_count: json
            .get("nodes")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        mesh_count: json
            .get("meshes")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        material_count: json
            .get("materials")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        primitive_count,
        emissive_material_count,
        arda_procedural_nodes: arda_counts.values().sum(),
        duplicate_arda_procedural_nodes,
        overlay_alignment: inspect_arda_overlay_alignment(&node_names, json),
    }
}

fn inspect_arda_overlay_alignment(
    node_names: &[String],
    json: &Value,
) -> Option<ArdaOverlayAlignmentInspection> {
    let nodes = json.get("nodes").and_then(Value::as_array)?;
    let mut overlay_positions = Vec::new();
    let mut screen_thin_axis = None;
    let mut screen_node_present = false;
    let mut bezel_node_count = 0;
    let mut trace_node_count = 0;
    let mut notes = Vec::new();

    for (idx, name) in node_names.iter().enumerate() {
        let is_screen = name == "arda_screen_cyan_emissive_overlay";
        let is_bezel = name.starts_with("arda_bezel_");
        let is_trace = name.starts_with("arda_cyan_");
        if !(is_screen || is_bezel || is_trace) {
            continue;
        }
        let Some(node) = nodes.get(idx) else {
            continue;
        };
        if is_screen {
            screen_node_present = true;
            screen_thin_axis = node_scale(node).and_then(|scale| smallest_axis(&scale));
        }
        if is_bezel {
            bezel_node_count += 1;
        }
        if is_trace {
            trace_node_count += 1;
        }
        if let Some(translation) = node_translation(node) {
            overlay_positions.push(translation);
        }
    }

    if !screen_node_present && bezel_node_count == 0 && trace_node_count == 0 {
        return None;
    }

    let (inferred_depth_axis, max_overlay_plane_separation) = if overlay_positions.len() >= 2 {
        let ranges = axis_ranges(&overlay_positions);
        let depth_axis = screen_thin_axis.clone().or_else(|| {
            ranges
                .iter()
                .min_by(|(_, lhs), (_, rhs)| lhs.total_cmp(rhs))
                .map(|(axis, _)| (*axis).to_string())
        });
        let separation = depth_axis.as_deref().and_then(|depth_axis| {
            ranges
                .iter()
                .find_map(|(axis, range)| (*axis == depth_axis).then_some(*range))
        });
        (depth_axis, separation)
    } else {
        (screen_thin_axis.clone(), None)
    };

    if screen_node_present
        && screen_thin_axis.is_none()
        && max_overlay_plane_separation.is_none_or(|separation| separation > 0.02)
    {
        notes.push(
            "screen overlay has no scale data and coplanarity could not be proven tightly"
                .to_string(),
        );
    }
    if screen_thin_axis.is_some()
        && screen_thin_axis != inferred_depth_axis
        && inferred_depth_axis.is_some()
    {
        notes.push(format!(
            "screen thin axis {:?} differs from inferred overlay plane axis {:?}",
            screen_thin_axis, inferred_depth_axis
        ));
    }

    Some(ArdaOverlayAlignmentInspection {
        screen_node_present,
        bezel_node_count,
        trace_node_count,
        inferred_depth_axis,
        max_overlay_plane_separation,
        screen_thin_axis,
        notes,
    })
}

fn node_translation(node: &Value) -> Option<[f64; 3]> {
    value_triplet(node.get("translation")?)
}

fn node_scale(node: &Value) -> Option<[f64; 3]> {
    value_triplet(node.get("scale")?)
}

fn value_triplet(value: &Value) -> Option<[f64; 3]> {
    let values = value.as_array()?;
    if values.len() != 3 {
        return None;
    }
    Some([
        values.first()?.as_f64()?,
        values.get(1)?.as_f64()?,
        values.get(2)?.as_f64()?,
    ])
}

fn axis_ranges(points: &[[f64; 3]]) -> Vec<(&'static str, f64)> {
    (0..3)
        .map(|axis| {
            let mut min_value = f64::INFINITY;
            let mut max_value = f64::NEG_INFINITY;
            for point in points {
                min_value = min_value.min(point[axis]);
                max_value = max_value.max(point[axis]);
            }
            let label = match axis {
                0 => "x",
                1 => "y",
                _ => "z",
            };
            (label, max_value - min_value)
        })
        .collect()
}

fn smallest_axis(values: &[f64; 3]) -> Option<String> {
    let axes = [("x", values[0]), ("y", values[1]), ("z", values[2])];
    axes.iter()
        .min_by(|(_, lhs), (_, rhs)| lhs.total_cmp(rhs))
        .map(|(axis, _)| (*axis).to_string())
}

pub fn ensure_clean_arda_materialization(
    inspection: &GlbMaterializationInspection,
) -> anyhow::Result<()> {
    if !inspection.duplicate_arda_procedural_nodes.is_empty() {
        anyhow::bail!(
            "duplicate ARDA procedural nodes detected: {:?}",
            inspection.duplicate_arda_procedural_nodes
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_materialization_sidecar(
    path: &Path,
    materializer: &str,
    backend: &BlenderExecutionBackend,
    fallback_reason: Option<&str>,
    source_sha256: &str,
    output_sha256: &str,
    remote_host: Option<&str>,
    blender_path: &Path,
    inspection: &GlbMaterializationInspection,
) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read metadata sidecar {}: {e}", path.display()))?;
    let mut metadata: Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse metadata sidecar {}: {e}", path.display()))?;
    let Some(obj) = metadata.as_object_mut() else {
        anyhow::bail!("metadata sidecar {} is not a JSON object", path.display());
    };

    let forge = obj
        .entry("forge".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !forge.is_object() {
        *forge = serde_json::json!({});
    }
    let Some(forge_obj) = forge.as_object_mut() else {
        anyhow::bail!("forge metadata must be a JSON object");
    };
    let entry = serde_json::json!({
        "schema_version": MATERIALIZATION_SCHEMA_VERSION,
        "materializer": materializer,
        "backend": backend,
        "fallback_reason": fallback_reason,
        "source_sha256": source_sha256,
        "output_sha256": output_sha256,
        "remote_host": remote_host,
        "blender_path": blender_path.display().to_string(),
        "recorded_unix_secs": current_unix_secs(),
        "inspection": inspection,
    });
    forge_obj.insert("last_materialization".to_string(), entry.clone());
    let history = forge_obj
        .entry("materialization_history".to_string())
        .or_insert_with(|| serde_json::json!([]));
    if !history.is_array() {
        *history = serde_json::json!([]);
    }
    let Some(history_array) = history.as_array_mut() else {
        anyhow::bail!("materialization_history metadata must be an array");
    };
    history_array.push(entry);
    if history_array.len() > 20 {
        let drain_count = history_array.len() - 20;
        history_array.drain(0..drain_count);
    }

    std::fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )?;
    Ok(())
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Heuristic for deciding whether the monitor materializer should run.
pub fn should_materialize_arda_monitor(
    asset_id: &str,
    scene_binding: &str,
    material_family: &str,
    prompt: &str,
) -> bool {
    let haystack = format!("{asset_id} {scene_binding} {material_family} {prompt}").to_lowercase();
    haystack.contains("monitor") || haystack.contains("display") || haystack.contains("screen")
}

/// Heuristic scaffold for the next ARDA asset-family materializer.
pub fn should_materialize_arda_desk(
    asset_id: &str,
    scene_binding: &str,
    material_family: &str,
    prompt: &str,
) -> bool {
    let haystack = format!("{asset_id} {scene_binding} {material_family} {prompt}").to_lowercase();
    haystack.contains("desk") || haystack.contains("table") || haystack.contains("console_surface")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_glb_json(json: &str) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let len = 12 + 8 + json_bytes.len();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"glTF");
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&(len as u32).to_le_bytes());
        bytes.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"JSON");
        bytes.extend_from_slice(&json_bytes);
        bytes
    }

    #[test]
    fn monitor_assets_are_detected() {
        assert!(should_materialize_arda_monitor(
            "upper_monitor_1",
            "upper_monitor_1",
            "world_terminal_housing",
            "ARDA boardroom asset"
        ));
        assert!(should_materialize_arda_monitor(
            "console",
            "console",
            "world_terminal_housing",
            "dark graphite display with cyan glow"
        ));
    }

    #[test]
    fn unrelated_assets_are_not_detected() {
        assert!(!should_materialize_arda_monitor(
            "desk_left_surface",
            "desk_left_surface",
            "world_district_structure",
            "boardroom table surface"
        ));
    }

    #[test]
    fn desk_assets_are_detected_by_scaffold() {
        assert!(should_materialize_arda_desk(
            "desk_left_surface",
            "desk_left_surface",
            "boardroom_desk_surface",
            "dark command table"
        ));
        assert!(!should_materialize_arda_desk(
            "upper_monitor_1",
            "upper_monitor_1",
            "boardroom_monitor_bezel",
            "cyan screen"
        ));
    }

    #[test]
    fn glb_inspection_counts_arda_nodes_and_emissive_materials() {
        let bytes = minimal_glb_json(
            r#"{
                "asset": {"version": "2.0"},
                "nodes": [{"name":"base"}, {"name":"arda_screen"}, {"name":"arda_bezel"}],
                "meshes": [{"primitives":[{}]}, {"primitives":[{},{}]}],
                "materials": [{"name":"body"}, {"name":"glow", "emissiveFactor":[0,1,1]}]
            }"#,
        );
        let inspection = inspect_glb_materialization_bytes(&bytes).expect("inspect glb");
        assert_eq!(inspection.node_count, 3);
        assert_eq!(inspection.mesh_count, 2);
        assert_eq!(inspection.primitive_count, 3);
        assert_eq!(inspection.emissive_material_count, 1);
        assert_eq!(inspection.arda_procedural_nodes, 2);
        assert!(inspection.duplicate_arda_procedural_nodes.is_empty());
        assert!(inspection.overlay_alignment.is_none());
        ensure_clean_arda_materialization(&inspection).expect("clean materialization");
    }

    #[test]
    fn glb_inspection_reports_overlay_alignment_diagnostics() {
        let bytes = minimal_glb_json(
            r#"{
                "asset": {"version": "2.0"},
                "nodes": [
                    {"name":"geometry_0"},
                    {"name":"arda_screen_cyan_emissive_overlay", "translation":[0.0,0.0,0.02], "scale":[0.5,0.25,0.01]},
                    {"name":"arda_bezel_top_graphite", "translation":[0.0,0.32,0.021], "scale":[0.6,0.03,0.01]},
                    {"name":"arda_cyan_top_trace", "translation":[0.0,0.36,0.022], "scale":[0.3,0.01,0.005]}
                ],
                "meshes": [],
                "materials": []
            }"#,
        );
        let inspection = inspect_glb_materialization_bytes(&bytes).expect("inspect glb");
        let alignment = inspection
            .overlay_alignment
            .expect("overlay alignment diagnostics");
        assert!(alignment.screen_node_present);
        assert_eq!(alignment.bezel_node_count, 1);
        assert_eq!(alignment.trace_node_count, 1);
        assert_eq!(alignment.inferred_depth_axis.as_deref(), Some("z"));
        assert_eq!(alignment.screen_thin_axis.as_deref(), Some("z"));
        assert!(alignment.notes.is_empty());
    }

    #[test]
    fn idempotency_regression_rejects_duplicate_procedural_nodes() {
        let bytes = minimal_glb_json(
            r#"{
                "asset": {"version": "2.0"},
                "nodes": [{"name":"arda_screen"}, {"name":"arda_screen"}],
                "meshes": [],
                "materials": []
            }"#,
        );
        let inspection = inspect_glb_materialization_bytes(&bytes).expect("inspect glb");
        assert_eq!(
            inspection
                .duplicate_arda_procedural_nodes
                .get("arda_screen"),
            Some(&2)
        );
        assert!(ensure_clean_arda_materialization(&inspection).is_err());
    }

    #[test]
    fn monitor_template_removes_previous_procedural_nodes_before_rerun() {
        assert!(ARDA_MONITOR_MATERIALIZATION.contains("startswith(\"arda_\")"));
        assert!(ARDA_MONITOR_MATERIALIZATION.contains("bpy.data.objects.remove"));
    }
}
