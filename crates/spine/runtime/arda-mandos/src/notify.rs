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

    fn prefix_glyph() -> &'static str {
        // Prefer the visual sigil when the environment supports UTF-8.
        std::env::var("ARDA_MANDOS_NOTIFIER_ASCII")
            .ok()
            .filter(|value| value.eq_ignore_ascii_case("1") || value.eq_ignore_ascii_case("true"))
            .map(|_| "O")
            .unwrap_or("𓊝")
    }

    pub fn format_verdict(&self, outcome: &str, resonance: f64, score: f64) -> String {
        let sigil = match outcome {
            "Pass" => "◈",
            "Fail" => "∇",
            "Escalate" => "△",
            _ => "◈",
        };
        let prefix = Self::prefix_glyph();

        format!(
            "{prefix} Oracle | {sigil} {outcome} | Resonance: {:.2} | Score: {:.2}",
            resonance, score
        )
    }

    pub fn format_query(&self, task: &str) -> String {
        let truncated = truncate_str(task, 100);
        let prefix = Self::prefix_glyph();

        format!("{prefix} Oracle: Query — {truncated}")
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
