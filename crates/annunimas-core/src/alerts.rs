use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warn,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: AlertSeverity,
    pub source: String, // "ceo", "warden", "athena", etc.
    pub message: String,
    pub sigil: Option<String>,      // e.g., "𓋹" for override needed
    pub details: serde_json::Value, // Extra data (resonance value, container name, etc.)
}

pub struct AlertLogger {
    path: std::path::PathBuf,
}

impl AlertLogger {
    pub fn new(dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("alerts.jsonl");
        Ok(Self { path })
    }

    pub fn log(&self, alert: &Alert) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let json = serde_json::to_string(alert)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn critical(
        source: &str,
        message: &str,
        sigil: sigil.or(Some("𓃭".to_string())),  // Default guardian for alerts        details: serde_json::Value,
    ) -> Alert {
        Alert {
            timestamp: Utc::now(),
            severity: AlertSeverity::Critical,
            source: source.to_string(),
            message: message.to_string(),
            sigil: sigil.or(Some("𓃭".to_string())), // Guardian default for critical
            details,
        }
    }

    pub fn warn(
        source: &str,
        message: &str,
        sigil: sigil.or(Some("𓃭".to_string())),  // Default guardian for alerts        details: serde_json::Value,
    ) -> Alert {
        Alert {
            timestamp: Utc::now(),
            severity: AlertSeverity::Warn,
            source: source.to_string(),
            message: message.to_string(),
            sigil,
            details,
        }
    }
}
