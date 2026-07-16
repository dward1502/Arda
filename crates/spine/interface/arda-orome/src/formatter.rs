// sigil: REPAIR
use crate::registry::AgentStatus;
use arda_core::soterion::SIGIL_DICTIONARY;

pub struct SoterionFormatter;

impl SoterionFormatter {
    fn normalized_identity(agent_type: &str) -> String {
        match agent_type.to_ascii_lowercase().as_str() {
            "arandur" | "ceo" => "ceo".to_string(),
            "prometheus" => "prometheus".to_string(),
            "warden" => "warden".to_string(),
            "hermes" => "hermes".to_string(),
            "charon" => "charon".to_string(),
            other => other.to_string(),
        }
    }

    pub fn sigil(agent_type: &str) -> &'static str {
        match Self::normalized_identity(agent_type).as_str() {
            "oracle" => "𓊝",
            "hermes" => "𓅃",
            "athena" => "𓂀",
            "plutus" => "𓆣",
            "apollo" => "𓋹",
            "soterion" => "𓁿",
            "ceo" | "arandur" => "𓀀",
            "warden" => "𓃭",
            "hades" => "𓁷",
            "prometheus" => "∇",
            "charon" => "↝",
            _ => "𓁿",
        }
    }

    fn is_control_plane(agent_id: &str) -> bool {
        matches!(
            Self::normalized_identity(agent_id).as_str(),
            "ceo" | "prometheus" | "warden" | "hermes" | "charon"
        )
    }

    fn status_mark(status: &AgentStatus) -> &'static str {
        match status {
            AgentStatus::Online => "◈",
            AgentStatus::Busy => "⚡",
            AgentStatus::Away => "↝",
            AgentStatus::Offline => "✖",
        }
    }

    pub fn format_agent_status(agent_id: &str, status: &AgentStatus) -> String {
        let sigil = Self::sigil(agent_id);
        let status_emoji = Self::status_mark(status);

        format!("{} {} {}", sigil, agent_id, status_emoji)
    }

    pub fn format_status_summary(agents: &[(String, AgentStatus)]) -> String {
        if agents.is_empty() {
            return "𓅃 ∅".to_string();
        }

        let focus: Vec<(String, AgentStatus)> = agents
            .iter()
            .filter(|(agent_id, _)| Self::is_control_plane(agent_id))
            .cloned()
            .collect();
        let focus = if focus.is_empty() {
            agents.to_vec()
        } else {
            focus
        };
        let active_total = agents
            .iter()
            .filter(|(_, status)| !matches!(status, AgentStatus::Offline))
            .count();

        let roster = focus
            .iter()
            .map(|(agent_id, status)| Self::format_agent_status(agent_id, status))
            .collect::<Vec<_>>()
            .join(" | ");
        format!("𓅃 {} | ∑ {}", roster, active_total)
    }

    pub fn format_compact(message: &str, max_len: usize) -> String {
        if message.len() <= max_len {
            return message.to_string();
        }

        let mut result = String::with_capacity(max_len);
        let words: Vec<&str> = message.split_whitespace().collect();

        for word in words {
            if result.len() + word.len() + 1 > max_len - 3 {
                break;
            }
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(word);
        }

        if result.len() < message.len() {
            result.push_str("...");
        }

        result
    }

    pub fn sigil_meaning(sigil: &str) -> Option<&'static str> {
        SIGIL_DICTIONARY.get(sigil).copied()
    }

    pub fn format_joule_work(joules: f64) -> String {
        if joules >= 1_000_000.0 {
            format!("{:.1}M 𓆣", joules / 1_000_000.0)
        } else if joules >= 1_000.0 {
            format!("{:.1}K 𓆣", joules / 1_000.0)
        } else {
            format!("{:.0} 𓆣", joules)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigil_lookup() {
        assert_eq!(SoterionFormatter::sigil("hermes"), "𓅃");
        assert_eq!(SoterionFormatter::sigil("oracle"), "𓊝");
        assert_eq!(SoterionFormatter::sigil("athena"), "𓂀");
    }

    #[test]
    fn test_compact() {
        let long = "This is a very long message that should be truncated to fit within the character limit";
        let compact = SoterionFormatter::format_compact(long, 30);
        assert!(compact.len() <= 30);
    }
}
