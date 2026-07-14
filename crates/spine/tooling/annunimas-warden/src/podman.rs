// sigil: REPAIR
//! Podman client wrapper
//!
//! Real Podman API client for container management.

use anyhow::{Context, Result};
use podman_api::Podman;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
}

pub struct PodmanClient {
    client: Podman,
}

impl PodmanClient {
    /// Connect to Podman socket
    pub fn new() -> Result<Self> {
        let socket_paths = [
            "/run/user/1000/podman/podman.sock",
            "/run/podman/podman.sock",
        ];

        for path in socket_paths {
            if std::path::Path::new(path).exists() {
                info!("Connecting to Podman socket: {}", path);
                let client = Podman::unix(path);
                return Ok(Self { client });
            }
        }

        Err(anyhow::anyhow!(
            "No Podman socket found - is Podman running?"
        ))
    }

    /// List all containers (running and stopped)
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let containers = self
            .client
            .containers()
            .list(
                &podman_api::opts::ContainerListOpts::builder()
                    .all(true)
                    .build(),
            )
            .await
            .context("Failed to list containers")?;

        let mut result = Vec::new();
        for c in containers {
            result.push(container_info_from_parts(
                c.id,
                c.names.unwrap_or_default(),
                c.image,
                c.state,
            ));
        }

        Ok(result)
    }

    /// List only running containers
    pub async fn list_running(&self) -> Result<Vec<ContainerInfo>> {
        let containers = self
            .client
            .containers()
            .list(
                &podman_api::opts::ContainerListOpts::builder()
                    .all(false)
                    .build(),
            )
            .await
            .context("Failed to list running containers")?;

        let mut result = Vec::new();
        for c in containers {
            result.push(container_info_from_parts(
                c.id,
                c.names.unwrap_or_default(),
                c.image,
                c.state,
            ));
        }

        Ok(result)
    }

    /// Start a container
    pub async fn start(&self, container_name: &str) -> Result<()> {
        let id = self.find_container_id(container_name).await?;
        self.client
            .containers()
            .get(&id)
            .start(None)
            .await
            .context(format!("Failed to start container {}", container_name))?;
        info!("Started container: {}", container_name);
        Ok(())
    }

    /// Stop a container
    pub async fn stop(&self, container_name: &str, _timeout: u64) -> Result<()> {
        let id = self.find_container_id(container_name).await?;
        let opts = podman_api::opts::ContainerStopOpts::default();
        self.client
            .containers()
            .get(&id)
            .stop(&opts)
            .await
            .context(format!("Failed to stop container {}", container_name))?;
        info!("Stopped container: {}", container_name);
        Ok(())
    }

    /// Restart a container
    pub async fn restart(&self, container_name: &str) -> Result<()> {
        let id = self.find_container_id(container_name).await?;
        self.client
            .containers()
            .get(&id)
            .restart()
            .await
            .context(format!("Failed to restart container {}", container_name))?;
        info!("Restarted container: {}", container_name);
        Ok(())
    }

    /// Inspect a container - get full details
    pub async fn inspect(&self, container_name: &str) -> Result<serde_json::Value> {
        let id = self.find_container_id(container_name).await?;
        let info = self
            .client
            .containers()
            .get(&id)
            .inspect()
            .await
            .context(format!("Failed to inspect container {}", container_name))?;
        Ok(serde_json::to_value(info)?)
    }

    /// Find container ID by name
    async fn find_container_id(&self, container_name: &str) -> Result<String> {
        let containers = self.list_containers().await?;
        containers
            .into_iter()
            .find(|c| c.name == container_name || c.name.starts_with(container_name))
            .map(|c| c.id)
            .ok_or_else(|| anyhow::anyhow!("Container {} not found", container_name))
    }
}

fn primary_container_name(names: &[String]) -> &str {
    names
        .first()
        .map(|name| name.trim_start_matches('/'))
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown")
}

fn container_info_from_parts(
    id: Option<String>,
    names: Vec<String>,
    image: Option<String>,
    state: Option<String>,
) -> ContainerInfo {
    ContainerInfo {
        id: id.unwrap_or_default(),
        name: String::from(primary_container_name(&names)),
        image: image.unwrap_or_default(),
        state: state.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{container_info_from_parts, primary_container_name};

    #[test]
    fn primary_container_name_trims_podman_leading_slash() {
        let names = vec![String::from("/annunimas-charon")];

        assert_eq!(primary_container_name(&names), "annunimas-charon");
    }

    #[test]
    fn container_info_mapping_uses_unknown_name_when_missing() {
        let info = container_info_from_parts(
            Some(String::from("abc123")),
            Vec::new(),
            Some(String::from("localhost/warden:latest")),
            Some(String::from("running")),
        );

        assert_eq!(info.id, "abc123");
        assert_eq!(info.name, "unknown");
        assert_eq!(info.image, "localhost/warden:latest");
        assert_eq!(info.state, "running");
    }
}
