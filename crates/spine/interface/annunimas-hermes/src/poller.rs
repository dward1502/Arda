// sigil: REPAIR
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

pub struct StatusPoller {
    registry: Arc<RwLock<crate::AgentRegistry>>,
    discord: Arc<dyn crate::McpChannel>,
    interval_secs: u64,
    last_summary: Option<String>,
}

impl StatusPoller {
    pub fn new(
        registry: Arc<RwLock<crate::AgentRegistry>>,
        discord: Arc<dyn crate::McpChannel>,
        interval_secs: u64,
    ) -> Self {
        Self {
            registry,
            discord,
            interval_secs,
            last_summary: None,
        }
    }

    pub async fn start(&mut self) {
        let mut ticker = interval(Duration::from_secs(self.interval_secs));

        loop {
            ticker.tick().await;

            if let Err(e) = self.check_and_update().await {
                tracing::error!("Status poll failed: {}", e);
            }
        }
    }

    pub async fn check_and_update(&mut self) -> Result<(), crate::McpChannelError> {
        let agents = {
            let reg = self.registry.read().await;
            reg.list_agents()
                .into_iter()
                .map(|info| (info.name.clone(), info.status))
                .collect::<Vec<_>>()
        };

        let summary = crate::SoterionFormatter::format_status_summary(&agents);

        if Some(&summary) != self.last_summary.as_ref() {
            self.discord.send(&summary, "status").await?;
            self.last_summary = Some(summary);
        }

        Ok(())
    }

    pub async fn force_update(&mut self) -> Result<(), crate::McpChannelError> {
        self.last_summary = None;
        self.check_and_update().await
    }
}

pub async fn start_status_poller(
    registry: Arc<RwLock<crate::AgentRegistry>>,
    discord: Arc<dyn crate::McpChannel>,
    interval_secs: u64,
) {
    let mut poller = StatusPoller::new(registry, discord, interval_secs);
    poller.start().await;
}
