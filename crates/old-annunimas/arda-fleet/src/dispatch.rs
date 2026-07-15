use crate::FleetDecision;
use std::process::Stdio;
use tokio::process::Command;

pub struct EdgeDispatcher;

impl Default for EdgeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn dispatch_to_node(
        &self,
        ssh_target: &str,
        command: &str,
    ) -> anyhow::Result<DispatchResult> {
        let output = Command::new("tailscale")
            .args(["ssh", "--", ssh_target, command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

        let stderr_filtered: String = stderr
            .lines()
            .filter(|line| !line.contains("client version") && !line.contains("Warning:"))
            .collect::<Vec<_>>()
            .join("\n");

        let actual_success =
            output.status.success() || (stderr_filtered.is_empty() && !stdout.is_empty());

        let result = DispatchResult {
            success: actual_success,
            stdout,
            stderr: stderr_filtered,
            exit_code: output.status.code(),
        };

        Ok(result)
    }

    pub async fn dispatch_task(
        &self,
        decision: &FleetDecision,
        task_id: &str,
        task_payload: &str,
    ) -> anyhow::Result<DispatchResult> {
        let node_id = match decision {
            FleetDecision::Accept { node_id, .. } => node_id,
            FleetDecision::Reject { .. } => {
                return Err(anyhow::anyhow!("Cannot dispatch rejected decision"))
            }
        };

        let (tailscale_name, ssh_user) = match node_id.as_str() {
            "node-ser9-worker" => ("bluefin", "citadel"),
            "node-backbone-server-01" => ("beelink", "ardaserver"),
            "node-pi5-warden" => ("warden", "pi"),
            "node-pi5-citadel-avatar" => ("raspberrypi", "citadel"),
            _ => return Err(anyhow::anyhow!("Unknown node: {}", node_id)),
        };

        let ssh_target = format!("{}@{}", ssh_user, tailscale_name);

        let remote_cmd = format!(
            "cd /var/home/arda && cargo run -- run -t execute '{}' --payload '{}'",
            task_id, task_payload
        );

        self.dispatch_to_node(&ssh_target, &remote_cmd).await
    }

    pub async fn check_node_reachable(&self, node_name: &str) -> bool {
        let result = self.dispatch_to_node(node_name, "echo ok").await;
        result.map(|r| r.success).unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct DispatchResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl DispatchResult {
    pub fn is_success(&self) -> bool {
        self.success
    }
}
