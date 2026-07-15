//! Remote workspace staging for Blender-backed Forge-Mind passes.
//!
//! Forge-Mind is intentionally a distributed service surface: the control
//! workstation may own the project checkout while BlenderMCP/ComfyUI/vision run
//! on GPU-capable fleet nodes. Blender can only import paths visible from the
//! host running Blender, so this module maps and optionally syncs local asset
//! paths into a configured remote workspace before invoking BlenderMCP.

use std::path::{Path, PathBuf};
use tokio::process::Command;

const ENV_LOCAL_ROOT: &str = "FORGE_WORKSPACE_ROOT";
const ENV_REMOTE_ROOT: &str = "FORGE_BLENDER_REMOTE_ROOT";
const ENV_SYNC_HOST: &str = "FORGE_BLENDER_SYNC_HOST";
const ENV_SSH_PORT: &str = "FORGE_BLENDER_SSH_PORT";
const ENV_BLENDER_CLI: &str = "FORGE_BLENDER_CLI_COMMAND";

#[derive(Debug, Clone)]
pub struct RemoteWorkspaceConfig {
    local_root: PathBuf,
    remote_root: Option<PathBuf>,
    sync_host: Option<String>,
    ssh_port: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StagedAsset {
    pub local_path: PathBuf,
    pub blender_path: PathBuf,
    sync_host: Option<String>,
    ssh_port: Option<String>,
}

impl RemoteWorkspaceConfig {
    pub fn from_env() -> Self {
        let local_root = std::env::var(ENV_LOCAL_ROOT)
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Self {
            local_root,
            remote_root: std::env::var(ENV_REMOTE_ROOT).ok().map(PathBuf::from),
            sync_host: std::env::var(ENV_SYNC_HOST)
                .ok()
                .filter(|v| !v.trim().is_empty()),
            ssh_port: std::env::var(ENV_SSH_PORT)
                .ok()
                .filter(|v| !v.trim().is_empty()),
        }
    }

    pub fn status_lines(&self) -> Vec<String> {
        vec![
            format!("forge_workspace_root: {}", self.local_root.display()),
            format!(
                "blender_remote_root: {}",
                self.remote_root
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<disabled>".to_string())
            ),
            format!(
                "blender_sync_host: {}",
                self.sync_host
                    .as_deref()
                    .unwrap_or("<none; path mapping only>")
            ),
        ]
    }

    pub async fn stage_for_blender(&self, local_path: &Path) -> anyhow::Result<StagedAsset> {
        let local_abs = std::fs::canonicalize(local_path).map_err(|e| {
            anyhow::anyhow!(
                "failed to canonicalize local Blender asset path {}: {e}",
                local_path.display()
            )
        })?;

        let Some(remote_root) = &self.remote_root else {
            return Ok(StagedAsset {
                blender_path: local_abs.clone(),
                local_path: local_abs,
                sync_host: None,
                ssh_port: None,
            });
        };

        let local_root_abs = std::fs::canonicalize(&self.local_root).map_err(|e| {
            anyhow::anyhow!(
                "failed to canonicalize {ENV_LOCAL_ROOT} {}: {e}",
                self.local_root.display()
            )
        })?;
        let relative = local_abs.strip_prefix(&local_root_abs).map_err(|_| {
            anyhow::anyhow!(
                "asset path {} is not under Forge workspace root {}; set {ENV_LOCAL_ROOT} or disable {ENV_REMOTE_ROOT}",
                local_abs.display(),
                local_root_abs.display()
            )
        })?;
        let remote_path = remote_root.join(relative);

        if let Some(host) = &self.sync_host {
            sync_to_remote(host, self.ssh_port.as_deref(), &local_abs, &remote_path).await?;
        }

        Ok(StagedAsset {
            local_path: local_abs,
            blender_path: remote_path,
            sync_host: self.sync_host.clone(),
            ssh_port: self.ssh_port.clone(),
        })
    }
}

impl Default for RemoteWorkspaceConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl StagedAsset {
    pub async fn sync_back(&self) -> anyhow::Result<()> {
        if let Some(host) = &self.sync_host {
            sync_from_remote(
                host,
                self.ssh_port.as_deref(),
                &self.blender_path,
                &self.local_path,
            )
            .await?;
        }
        Ok(())
    }

    pub fn has_remote_sync(&self) -> bool {
        self.sync_host.is_some()
    }

    pub fn sync_host(&self) -> Option<&str> {
        self.sync_host.as_deref()
    }

    pub async fn run_remote_blender_script(
        &self,
        script_name: &str,
        code: &str,
    ) -> anyhow::Result<()> {
        let host = self.sync_host.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "remote Blender CLI fallback requires {ENV_SYNC_HOST}; no sync host configured"
            )
        })?;
        let script_dir = self
            .blender_path
            .parent()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "staged Blender path has no parent: {}",
                    self.blender_path.display()
                )
            })?
            .join(".forge-mind-scripts");
        let remote_script = script_dir.join(script_name);
        let mut local_script = std::env::temp_dir();
        local_script.push(format!("forge-mind-{}-{script_name}", std::process::id()));
        std::fs::write(&local_script, code)?;

        run_ssh_mkdir(host, self.ssh_port.as_deref(), &script_dir).await?;
        run_scp_to_remote(
            host,
            self.ssh_port.as_deref(),
            &local_script,
            &remote_script,
        )
        .await?;
        let _ = std::fs::remove_file(&local_script);

        let blender_cmd = std::env::var(ENV_BLENDER_CLI)
            .unwrap_or_else(|_| "flatpak run --command=blender org.blender.Blender".to_string());
        let remote_cmd = format!(
            "{} -b --python {}",
            blender_cmd,
            shell_quote(&remote_script.display().to_string())
        );
        run_ssh_shell(
            host,
            self.ssh_port.as_deref(),
            &remote_cmd,
            "remote Blender CLI materialization",
        )
        .await
    }
}

async fn sync_to_remote(
    host: &str,
    port: Option<&str>,
    local_path: &Path,
    remote_path: &Path,
) -> anyhow::Result<()> {
    let remote_parent = remote_path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "remote Blender path has no parent: {}",
            remote_path.display()
        )
    })?;
    run_ssh_mkdir(host, port, remote_parent).await?;
    run_scp_to_remote(host, port, local_path, remote_path).await
}

async fn sync_from_remote(
    host: &str,
    port: Option<&str>,
    remote_path: &Path,
    local_path: &Path,
) -> anyhow::Result<()> {
    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    run_scp_from_remote(host, port, remote_path, local_path).await
}

async fn run_ssh_mkdir(host: &str, port: Option<&str>, remote_parent: &Path) -> anyhow::Result<()> {
    let mut cmd = Command::new("ssh");
    if let Some(port) = port {
        cmd.arg("-p").arg(port);
    }
    cmd.arg(host)
        .arg("mkdir")
        .arg("-p")
        .arg(remote_parent.as_os_str());
    run_checked(cmd, "ssh mkdir for Blender remote workspace").await
}

async fn run_scp_to_remote(
    host: &str,
    port: Option<&str>,
    local_path: &Path,
    remote_path: &Path,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("scp");
    if let Some(port) = port {
        cmd.arg("-P").arg(port);
    }
    cmd.arg(local_path.as_os_str())
        .arg(format!("{}:{}", host, remote_path.display()));
    run_checked(cmd, "scp asset to Blender remote workspace").await
}

async fn run_scp_from_remote(
    host: &str,
    port: Option<&str>,
    remote_path: &Path,
    local_path: &Path,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("scp");
    if let Some(port) = port {
        cmd.arg("-P").arg(port);
    }
    cmd.arg(format!("{}:{}", host, remote_path.display()))
        .arg(local_path.as_os_str());
    run_checked(cmd, "scp materialized asset from Blender remote workspace").await
}

async fn run_ssh_shell(
    host: &str,
    port: Option<&str>,
    remote_cmd: &str,
    label: &str,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("ssh");
    if let Some(port) = port {
        cmd.arg("-p").arg(port);
    }
    cmd.arg(host).arg(remote_cmd);
    run_checked(cmd, label).await
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn run_checked(mut cmd: Command, label: &str) -> anyhow::Result<()> {
    let output = cmd
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("{label} failed to start: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "{label} failed with status {}\nstdout: {}\nstderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_config_uses_local_path() {
        let cwd = std::env::current_dir().expect("test cwd");
        let tmp = cwd.join("target/forge_remote_workspace_disabled.glb");
        std::fs::create_dir_all(tmp.parent().expect("tmp parent")).expect("mkdir");
        std::fs::write(&tmp, b"glTF").expect("write tmp");
        let cfg = RemoteWorkspaceConfig {
            local_root: cwd,
            remote_root: None,
            sync_host: None,
            ssh_port: None,
        };
        let staged = cfg.stage_for_blender(&tmp).await.expect("stage local");
        assert_eq!(staged.local_path, staged.blender_path);
    }

    #[tokio::test]
    async fn remote_mapping_preserves_relative_workspace_path() {
        let cwd = std::env::current_dir().expect("test cwd");
        let tmp = cwd.join("target/forge_remote_workspace_mapped.glb");
        std::fs::create_dir_all(tmp.parent().expect("tmp parent")).expect("mkdir");
        std::fs::write(&tmp, b"glTF").expect("write tmp");
        let cfg = RemoteWorkspaceConfig {
            local_root: cwd,
            remote_root: Some(PathBuf::from("/srv/arda/forge-workspace")),
            sync_host: None,
            ssh_port: None,
        };
        let staged = cfg.stage_for_blender(&tmp).await.expect("stage mapped");
        assert!(staged
            .blender_path
            .ends_with("target/forge_remote_workspace_mapped.glb"));
    }
}
