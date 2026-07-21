#![cfg(feature = "full-cli")]
// sigil: ANKH
//! CLI dispatch for the FORGE-MIND asset workflow.

use std::path::PathBuf;
use std::time::Duration;

use arda_forge_mind::forge::{
    builtin_template, generate_asset, materialize_arda_monitor, upgrade_asset, GenerateOverrides,
    GenerateSpec, RemoteWorkspaceConfig, ScriptSource, UpgradeSpec, DEFAULT_ASSETS_ROOT,
    DEFAULT_NEGATIVE_PROMPT,
};
use arda_forge_mind::tools::comfyui::ComfyUiClient;
use arda_forge_mind::tools::mcp_bridge::McpBridge;

use super::super::ForgeCommands;

pub(crate) async fn handle(command: ForgeCommands) -> anyhow::Result<()> {
    match command {
        ForgeCommands::Status => {
            let bridge = McpBridge::from_env();
            let comfy = ComfyUiClient::from_env();
            let workspace = RemoteWorkspaceConfig::from_env();
            let workspace_status = workspace.status_lines();
            println!(
                "forge-mind status:\n  blender_addr: {}\n  comfyui_addr: {}\n  default_assets_root: {}\n  {}\n  {}\n  {}\n  upgrade templates: prompt_1, prompt_2, prompt_3\n  generate pipeline: comfyui_text_to_3d_hunyuan",
                bridge.addr(),
                comfy.base_url(),
                DEFAULT_ASSETS_ROOT,
                workspace_status[0],
                workspace_status[1],
                workspace_status[2]
            );
        }
        ForgeCommands::Upgrade {
            asset_id,
            domain,
            template,
            script,
            prompt_file,
            assets_root,
            scene_binding,
            material_family,
            persistent,
            export_glb,
        } => {
            let (script_body, script_source, default_export) =
                resolve_script(&template, script.as_deref(), prompt_file.as_deref())?;

            let spec = UpgradeSpec {
                asset_id: asset_id.clone(),
                domain,
                assets_root: assets_root
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_ROOT)),
                script: script_body,
                script_source,
                scene_binding: scene_binding.unwrap_or_else(|| asset_id.clone()),
                material_family,
                persistent,
                export_glb: export_glb.unwrap_or(default_export),
            };

            let bridge = McpBridge::from_env();
            let output = upgrade_asset(bridge, spec).await?;
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        ForgeCommands::MaterializeMonitor {
            asset_id,
            domain,
            assets_root,
        } => {
            let root = assets_root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_ROOT));
            let glb_path = root
                .join(&domain)
                .join(&asset_id)
                .join(format!("{asset_id}.glb"));
            let report = materialize_arda_monitor(&glb_path).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "asset_id": asset_id,
                    "domain": domain,
                    "glb_path": glb_path,
                    "materializer": report.materializer,
                    "backend": report.backend,
                    "remote_host": report.remote_host,
                    "source_sha256": report.source_sha256,
                    "output_sha256": report.output_sha256,
                    "inspection": report.inspection,
                    "metadata_path": report.metadata_path,
                    "status": "materialized"
                }))?
            );
        }
        ForgeCommands::Generate {
            prompt,
            asset_id,
            domain,
            negative,
            assets_root,
            scene_binding,
            material_family,
            comfyui_addr,
            timeout_secs,
            sdxl_seed,
            sdxl_steps,
            sdxl_cfg,
            hy3d_seed,
            hy3d_steps,
            hy3d_octree,
            post_cleanup,
        } => {
            let spec = GenerateSpec {
                asset_id: asset_id.clone(),
                domain,
                positive_prompt: prompt,
                negative_prompt: negative.unwrap_or_else(|| DEFAULT_NEGATIVE_PROMPT.to_string()),
                assets_root: assets_root
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_ROOT)),
                scene_binding: scene_binding.unwrap_or_else(|| asset_id.clone()),
                material_family,
                overrides: GenerateOverrides {
                    sdxl_seed,
                    sdxl_steps,
                    sdxl_cfg,
                    sdxl_width: None,
                    sdxl_height: None,
                    hy3d_seed,
                    hy3d_steps,
                    hy3d_guidance: None,
                    hy3d_octree_resolution: hy3d_octree,
                },
                post_cleanup_blender: post_cleanup,
            };

            let mut comfy = match comfyui_addr {
                Some(addr) => ComfyUiClient::new(addr),
                None => ComfyUiClient::from_env(),
            };
            if let Some(t) = timeout_secs {
                comfy = comfy.with_timeout(Duration::from_secs(t));
            }

            let output = generate_asset(comfy, spec).await?;
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }
    Ok(())
}

fn resolve_script(
    template: &str,
    script_path: Option<&str>,
    prompt_path: Option<&str>,
) -> anyhow::Result<(String, ScriptSource, bool)> {
    if let Some(path) = script_path {
        let body = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read --script {}: {e}", path))?;
        return Ok((body, ScriptSource::File, true));
    }
    if let Some(path) = prompt_path {
        anyhow::bail!(
            "--prompt-file {} requires Manwe-routed LLM translation, not yet wired in v0. \
             Pass --script for raw Python or --template for a built-in.",
            path
        );
    }
    let (body, default_export) = builtin_template(template).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown --template `{}` (try prompt_1, prompt_2, prompt_3)",
            template
        )
    })?;
    let source = match template {
        "prompt_2" | "prompt_2_baking_prep" | "baking" => {
            ScriptSource::BuiltIn("prompt_2_baking_prep")
        }
        "prompt_3" | "prompt_3_monitor_treatment" | "monitor" => {
            ScriptSource::BuiltIn("prompt_3_monitor_treatment")
        }
        _ => ScriptSource::BuiltIn("prompt_1_geometry_cleanup"),
    };
    Ok((body.to_string(), source, default_export))
}
