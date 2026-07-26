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

    pub fn format_verdict(&self, outcome: &str, resonance: f64, score: f64) -> String {
        let sigil = match outcome {
            "Pass" => "◈",
            "Fail" => "∇",
            "Escalate" => "△",
            _ => "◈",
        };

        format!(
            "𓊝 Oracle | {} {} | Resonance: {:.2} | Score: {:.2}",
            sigil, outcome, resonance, score
        )
    }

    pub fn format_query(&self, task: &str) -> String {
        let truncated = truncate_str(task, 100);

        format!("𓊝 Oracle: Query — {truncated}")
    }

    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }
}

fn truncate_str(input: &str, max_bytes: usize) -> String {
    if input.chars().count() <= max_bytes {
        return input.to_string();
    }

    let mut end = max_bytes;
    while !input.is_char_boundary(end) && end > 0 {
        end -= 1;
    }

    format!("{}...", &input[..end])
}
