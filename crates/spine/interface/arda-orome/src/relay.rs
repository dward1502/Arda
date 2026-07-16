// sigil: REPAIR
use std::path::PathBuf;

pub struct CliRelay {
    inbox_path: PathBuf,
}

impl CliRelay {
    pub fn new(inbox_path: PathBuf) -> Self {
        Self { inbox_path }
    }

    pub fn inbox_path(&self) -> &PathBuf {
        &self.inbox_path
    }

    pub fn is_command(&self, content: &str) -> bool {
        let trimmed = content.trim();
        trimmed.starts_with('!') || trimmed.starts_with('/') || trimmed.starts_with("opencode")
    }

    pub async fn write_to_inbox(&self, content: &str) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let entry = format!("[{}] {}\n", timestamp, content);

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.inbox_path)
            .await?;

        file.write_all(entry.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}
