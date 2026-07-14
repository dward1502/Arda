//! Top-level asset-production surface for forge-mind.
//!
//! Public callers should reach into forge-mind via this module's exports;
//! the underlying `tools::mcp_bridge`, `tools::blender_tools`,
//! `tools::comfyui`, and `tools::vision` layers are transport details.

/// Canonical ARDA assets root, relative to the workspace.
/// All paths (upgrade, generate, iterate) write under this.
pub const DEFAULT_ASSETS_ROOT: &str = "apps/arda-hud/src/assets/scene";

#[cfg(feature = "mcp-bridge")]
pub mod upgrade;

#[cfg(all(feature = "comfyui", feature = "mcp-bridge"))]
pub mod generate;

#[cfg(feature = "iterate")]
pub mod governance;

#[cfg(feature = "iterate")]
pub mod iterate;

#[cfg(feature = "mcp-bridge")]
pub mod materialize;

#[cfg(feature = "mcp-bridge")]
pub mod remote_workspace;

#[cfg(feature = "iterate")]
pub mod render;

#[cfg(feature = "mcp-bridge")]
pub use upgrade::{
    builtin_template, upgrade_asset, AssetOutput, ScriptSource, UpgradeSpec,
    PROMPT_1_GEOMETRY_CLEANUP,
};

#[cfg(all(feature = "comfyui", feature = "mcp-bridge"))]
pub use generate::{
    generate_asset, GenerateOverrides, GenerateSpec, GeneratedAsset, DEFAULT_NEGATIVE_PROMPT,
};

#[cfg(feature = "iterate")]
pub use iterate::{
    iterate_asset, IterateResult, IterateSpec, IterationRecord, DEFAULT_ACCEPT_THRESHOLD,
    DEFAULT_BUDGET_ITERS,
};

#[cfg(feature = "mcp-bridge")]
pub use materialize::{
    materialize_arda_monitor, should_materialize_arda_desk, should_materialize_arda_monitor,
    BlenderExecutionBackend, GlbMaterializationInspection, MaterializationReport,
};

#[cfg(feature = "mcp-bridge")]
pub use remote_workspace::RemoteWorkspaceConfig;

#[cfg(feature = "iterate")]
pub use render::{RenderConfig, DEFAULT_ANGLES, DEFAULT_PYTHON};
