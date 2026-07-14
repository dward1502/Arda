//! Headless render of a GLB to N camera angles via `bpy` (Python module).
//!
//! `bpy` is Blender's Python API as an importable module. We shell out to a
//! Python interpreter that has it installed (env `FORGE_BPY_PYTHON`, default
//! `/tmp/forge-venv/bin/python3`). The Python script lives at
//! `render_angles.py` next to this file, included at build time.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_PYTHON: &str = "/tmp/forge-venv/bin/python3";
pub const DEFAULT_ANGLES: &[&str] = &["front", "three_quarter", "side"];
pub const DEFAULT_WIDTH: u32 = 768;
pub const DEFAULT_HEIGHT: u32 = 768;

const RENDER_SCRIPT: &str = include_str!("render_angles.py");

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub python: PathBuf,
    pub angles: Vec<String>,
    pub width: u32,
    pub height: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl RenderConfig {
    pub fn from_env() -> Self {
        let python =
            std::env::var("FORGE_BPY_PYTHON").unwrap_or_else(|_| DEFAULT_PYTHON.to_string());
        let angles = std::env::var("FORGE_RENDER_ANGLES")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_ANGLES.iter().map(|s| s.to_string()).collect());
        let width = std::env::var("FORGE_RENDER_WIDTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_WIDTH);
        let height = std::env::var("FORGE_RENDER_HEIGHT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_HEIGHT);
        Self {
            python: PathBuf::from(python),
            angles,
            width,
            height,
        }
    }
}

/// Render `glb_path` to PNGs in `output_dir`, one per configured angle.
/// Returns the list of output PNG paths in the same order as `cfg.angles`.
pub fn render_glb_angles(
    glb_path: &Path,
    output_dir: &Path,
    asset_id: &str,
    cfg: &RenderConfig,
) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(output_dir)?;
    let glb_abs = std::fs::canonicalize(glb_path)?;
    let out_abs = std::fs::canonicalize(output_dir)?;

    let result = Command::new(&cfg.python)
        .arg("-c")
        .arg(RENDER_SCRIPT)
        .env("FORGE_GLB_PATH", &glb_abs)
        .env("FORGE_OUTPUT_DIR", &out_abs)
        .env("FORGE_ASSET_ID", asset_id)
        .env("FORGE_RENDER_WIDTH", cfg.width.to_string())
        .env("FORGE_RENDER_HEIGHT", cfg.height.to_string())
        .env("FORGE_ANGLES", cfg.angles.join(","))
        .output()
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to invoke bpy Python ({}): {e}. Set FORGE_BPY_PYTHON to a python interpreter with `bpy` installed.",
                cfg.python.display()
            )
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        anyhow::bail!(
            "bpy render exited {}: stderr=\n{}\nstdout=\n{}",
            result.status,
            stderr,
            stdout
        );
    }

    let paths: Vec<PathBuf> = cfg
        .angles
        .iter()
        .map(|a| out_abs.join(format!("{asset_id}_{a}.png")))
        .collect();
    for p in &paths {
        if !p.exists() {
            anyhow::bail!(
                "expected angle render not produced: {} (bpy stdout: {})",
                p.display(),
                String::from_utf8_lossy(&result.stdout)
            );
        }
    }
    Ok(paths)
}
