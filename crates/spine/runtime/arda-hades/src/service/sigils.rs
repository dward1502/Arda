use crate::types::{ActionRecord, SigilState, SigilVacuumRule};
use arda_core::error::{ArdaError, Result};
use arda_core::file_sigil_name_from_registry;
use arda_core::SoterionMachineSigil;
use regex::Regex;
use std::fs;
use std::path::Path;

pub(super) fn read_sigil(path: &Path) -> Option<SigilState> {
    let content = fs::read_to_string(path).ok()?;

    // Fast-path: inspect first non-empty line to pick the right parser.
    // This avoids trying all 4 parsers on files that clearly have no sigil.
    let first_line = content.lines().next().unwrap_or_default();
    let first_trimmed = first_line.trim_start();

    if first_trimmed.is_empty() {
        return None;
    }

    // JSON object on first line?
    if first_trimmed.starts_with('{') {
        // Try just the first line first, then the full trimmed content
        return parse_sigil_from_json(first_line).or_else(|| parse_sigil_from_json(content.trim()));
    }

    // YAML frontmatter?
    if first_trimmed == "---" {
        return parse_sigil_from_frontmatter(&content);
    }

    // Comment-style sigil line (# sigil: REPAIR, // sigil: COIN, etc.)?
    if first_trimmed.starts_with('#')
        || first_trimmed.starts_with("//")
        || first_trimmed.starts_with(';')
        || first_trimmed.starts_with("/*")
        || first_trimmed.starts_with('*')
        || first_trimmed.starts_with("<!--")
    {
        if let Some(sigil) = parse_scalar_sigil_line(first_trimmed) {
            return Some(map_sigil(&sigil));
        }
    }

    // Fallback: scan first 80 lines for key-value sigil patterns.
    parse_sigil_from_key_value_lines(&content)
}

fn parse_sigil_from_json(input: &str) -> Option<SigilState> {
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    let sigil = value.get("sigil").and_then(|v| v.as_str())?;
    Some(map_sigil(sigil))
}

fn parse_sigil_from_frontmatter(content: &str) -> Option<SigilState> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut block = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    parse_sigil_from_key_value_lines(&block)
}

fn parse_sigil_from_key_value_lines(content: &str) -> Option<SigilState> {
    for line in content.lines().take(80) {
        if let Some(sigil) = parse_scalar_sigil_line(line) {
            return Some(map_sigil(&sigil));
        }
    }
    None
}

fn parse_scalar_sigil_line(line: &str) -> Option<String> {
    // Strip comment prefixes in-place without allocating intermediate Strings.
    let mut raw = line.trim();
    for prefix in ["//", "#", ";", "/*", "*", "<!--"] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            raw = rest.trim_start();
        }
    }
    if let Some(rest) = raw.strip_suffix("-->") {
        raw = rest.trim_end();
    }
    if raw.is_empty() {
        return None;
    }
    if let Some(name) = file_sigil_name_from_registry(raw) {
        return Some(name);
    }
    // Case-insensitive check for "sigil" prefix without allocating a lowercase copy.
    let lower = raw.to_ascii_lowercase();
    if !lower.starts_with("sigil") {
        return None;
    }
    let (_, rhs) = raw.split_once(':').or_else(|| raw.split_once('='))?;
    let value = rhs
        .trim()
        .trim_end_matches(',')
        .trim_matches('"')
        .trim_matches('\'')
        .trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_owned())
}

fn map_sigil(sigil: &str) -> SigilState {
    let normalized =
        file_sigil_name_from_registry(sigil).unwrap_or_else(|| sigil.trim().to_ascii_uppercase());
    match normalized.as_str() {
        "ANKH" => SigilState::Ankh,
        "EYE" => SigilState::Eye,
        "SCROLL" => SigilState::Scroll,
        "COIN" => SigilState::Coin,
        "REPAIR" => SigilState::Repair,
        "ORPHAN_TEMP" => SigilState::OrphanTemp,
        "QUARANTINE" => SigilState::Quarantine,
        "CONDEMNED" => SigilState::Condemned,
        "ARCHIVED" => SigilState::Archived,
        _ => SigilState::Unknown,
    }
}

pub(super) fn hades_event_sigil(
    event: &str,
    details: &serde_json::Value,
) -> Option<SoterionMachineSigil> {
    match event {
        "coin_detected" | "destructive_quorum_denied" => Some(SoterionMachineSigil::new(
            "SG_HADES_QUARANTINE",
            vec!["hades".to_owned(), "quarantine".to_owned()],
            "high",
            "quarantine",
            "hades",
        )),
        "sweep_complete"
            if details
                .get("actions_taken")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                == 0 =>
        {
            Some(SoterionMachineSigil::new(
                "SG_HADES_VACUUM_CANDIDATE",
                vec!["hades".to_owned(), "vacuum".to_owned()],
                "medium",
                "vacuum",
                "hades",
            ))
        }
        _ => None,
    }
}

pub(super) fn action_record_matches_rule(
    record: &ActionRecord,
    rule: &SigilVacuumRule,
) -> Result<bool> {
    if let Some(pattern) = &rule.code_regex {
        let regex = Regex::new(pattern).map_err(|err| ArdaError::Agent {
            agent: "hades".to_owned(),
            message: format!("invalid sigil code regex: {err}"),
        })?;
        let Some(code) = record.sigil_code.as_deref() else {
            return Ok(false);
        };
        if !regex.is_match(code) {
            return Ok(false);
        }
    }
    if let Some(retention) = &rule.retention {
        if record.sigil_retention.as_deref() != Some(retention.as_str()) {
            return Ok(false);
        }
    }
    if let Some(tag) = &rule.tag {
        if !record.sigil_tags.iter().any(|value| value == tag) {
            return Ok(false);
        }
    }
    if let Some(source) = &rule.source {
        if record.sigil_source.as_deref() != Some(source.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn json_value_matches_rule(
    value: &serde_json::Value,
    rule: &SigilVacuumRule,
) -> Result<bool> {
    let sigil = value.get("soterion").or_else(|| {
        value
            .get("payload")
            .and_then(|payload| payload.get("soterion"))
    });

    let code = sigil
        .and_then(|entry| entry.get("sigil_code"))
        .and_then(|entry| entry.as_str())
        .or_else(|| value.get("sigil_code").and_then(|entry| entry.as_str()));
    let retention = sigil
        .and_then(|entry| entry.get("sigil_retention"))
        .and_then(|entry| entry.as_str())
        .or_else(|| {
            value
                .get("sigil_retention")
                .and_then(|entry| entry.as_str())
        });
    let source = sigil
        .and_then(|entry| entry.get("sigil_source"))
        .and_then(|entry| entry.as_str())
        .or_else(|| value.get("sigil_source").and_then(|entry| entry.as_str()));
    let tags: Vec<String> = sigil
        .and_then(|entry| entry.get("sigil_tags"))
        .and_then(|entry| entry.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .or_else(|| {
            value
                .get("sigil_tags")
                .and_then(|entry| entry.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
        })
        .unwrap_or_default();

    if let Some(pattern) = &rule.code_regex {
        let regex = Regex::new(pattern).map_err(|err| ArdaError::Agent {
            agent: "hades".to_owned(),
            message: format!("invalid sigil code regex: {err}"),
        })?;
        let Some(code) = code else {
            return Ok(false);
        };
        if !regex.is_match(code) {
            return Ok(false);
        }
    }
    if let Some(expected) = &rule.retention {
        if retention != Some(expected.as_str()) {
            return Ok(false);
        }
    }
    if let Some(expected) = &rule.tag {
        if !tags.iter().any(|value| value == expected) {
            return Ok(false);
        }
    }
    if let Some(expected) = &rule.source {
        if source != Some(expected.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn sigil_label(sigil: SigilState) -> &'static str {
    match sigil {
        SigilState::Ankh => "ANKH",
        SigilState::Eye => "EYE",
        SigilState::Scroll => "SCROLL",
        SigilState::Coin => "COIN",
        SigilState::Repair => "REPAIR",
        SigilState::OrphanTemp => "ORPHAN_TEMP",
        SigilState::Archived => "ARCHIVE",
        SigilState::Condemned => "CONDEMNED",
        SigilState::Quarantine => "QUARANTINE",
        SigilState::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_coin_comment_maps_to_coin_sigil() {
        assert_eq!(parse_scalar_sigil_line("# 🪙").as_deref(), Some("COIN"));
        assert_eq!(parse_scalar_sigil_line("// 🪙").as_deref(), Some("COIN"));
        assert!(matches!(map_sigil("🪙"), SigilState::Coin));
    }
}
