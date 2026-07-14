// sigil: REPAIR
pub struct OracleNotifier {
    channel_id: String,
}

impl OracleNotifier {
    pub fn new(channel_id: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
        }
    }

    pub fn format_verdict(&self, outcome: &str, resonance: f64) -> String {
        let sigil = match outcome {
            "Pass" => "◈",
            "Fail" => "∇",
            _ => "◈",
        };

        format!(
            "𓊝 Oracle | {} {} | Resonance: {:.2}",
            sigil, outcome, resonance
        )
    }

    pub fn format_query(&self, task: &str) -> String {
        let truncated = if task.len() > 100 {
            format!("{}...", &task[..97])
        } else {
            task.to_string()
        };

        format!("𓊝 Oracle: Query — {}", truncated)
    }

    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }
}
