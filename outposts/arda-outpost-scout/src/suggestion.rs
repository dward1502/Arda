use crate::{observation::CrateStatus, CrateObservation, SurveyReport};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdvisoryLevel {
    Advisory,
    Caution,
    Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdvisorySource {
    ShellWithoutSource,
    ShellWithoutTests,
    UnknownStatus,
    MissingEntrypoints,
    NoObservations,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvisoryInput {
    pub name: String,
    pub status: CrateStatus,
    pub key_entrypoints: Vec<String>,
    pub test_surface: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Advisory {
    pub source: AdvisorySource,
    pub level: AdvisoryLevel,
    pub message: String,
}

impl Advisory {
    pub fn new(source: AdvisorySource, level: AdvisoryLevel, message: impl Into<String>) -> Self {
        Self {
            source,
            level,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvisoryReport {
    pub source: String,
    pub generated_at: DateTime<Utc>,
    pub schema_version: String,
    pub max_level: AdvisoryLevel,
    pub advisories: Vec<Advisory>,
}

fn rank_to_level(rank: u8) -> AdvisoryLevel {
    match rank {
        0 => AdvisoryLevel::Advisory,
        1 => AdvisoryLevel::Caution,
        _ => AdvisoryLevel::Action,
    }
}

impl AdvisoryReport {
    pub fn new(source: impl Into<String>, advisories: Vec<Advisory>) -> Self {
        let max_level = advisories
            .iter()
            .map(|advisory| level_rank(advisory.level))
            .max()
            .map(rank_to_level)
            .unwrap_or(AdvisoryLevel::Action);
        Self {
            source: source.into(),
            generated_at: Utc::now(),
            schema_version: "1".to_string(),
            max_level,
            advisories,
        }
    }
}

pub fn analyze_survey(report: &SurveyReport) -> AdvisoryReport {
    let advisories = if report.observations.is_empty() {
        vec![Advisory::new(
            AdvisorySource::NoObservations,
            AdvisoryLevel::Action,
            "survey returned no crate observations; scout path or repo layout may be invalid",
        )]
    } else {
        report
            .observations
            .iter()
            .flat_map(advisories_for_observation)
            .collect()
    };

    AdvisoryReport::new(report.source.clone(), advisories)
}

pub fn summarize_advisories(report: &AdvisoryReport) -> String {
    let mut parts = Vec::new();
    parts.push(format!("source={}", report.source));
    parts.push(format!("max_level={:?}", report.max_level));
    parts.push(format!("advisories={}", report.advisories.len()));
    for advisory in &report.advisories {
        parts.push(format!(
            "- {:?}: {:?}: {}",
            advisory.source, advisory.level, advisory.message
        ));
    }
    parts.join("\n")
}

pub fn advisories_for_observation(observation: &CrateObservation) -> Vec<Advisory> {
    let mut advisories = Vec::new();

    if observation.status == CrateStatus::Shell {
        advisories.push(Advisory::new(
            AdvisorySource::ShellWithoutSource,
            AdvisoryLevel::Action,
            format!("{} is shell and may lack active source", observation.name),
        ));
    }

    if observation.status == CrateStatus::Shell && observation.test_surface.is_empty() {
        advisories.push(Advisory::new(
            AdvisorySource::ShellWithoutTests,
            AdvisoryLevel::Caution,
            format!("{} is shell with no detected tests", observation.name),
        ));
    }

    if matches!(
        observation.status,
        CrateStatus::Unknown | CrateStatus::Deprecated
    ) {
        advisories.push(Advisory::new(
            AdvisorySource::UnknownStatus,
            AdvisoryLevel::Caution,
            format!("{} has status {:?}", observation.name, observation.status),
        ));
    }

    if observation.key_entrypoints.is_empty() {
        advisories.push(Advisory::new(
            AdvisorySource::MissingEntrypoints,
            AdvisoryLevel::Advisory,
            format!("{} has no detected entrypoints", observation.name),
        ));
    }

    advisories
}

fn level_rank(level: AdvisoryLevel) -> u8 {
    match level {
        AdvisoryLevel::Advisory => 0,
        AdvisoryLevel::Caution => 1,
        AdvisoryLevel::Action => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_without_tests_is_caution() {
        let observation = CrateObservation {
            path: "tmp/shell-no-tests".into(),
            name: "shell-no-tests".into(),
            purpose: None,
            status: CrateStatus::Shell,
            key_entrypoints: vec!["src/lib.rs".to_string()],
            test_surface: vec![],
            dependencies: vec![],
            dev_patterns: vec![],
            observed_at: Utc::now(),
        };

        let advisories = advisories_for_observation(&observation);
        let mut ranks = advisories
            .iter()
            .map(|adv| level_rank(adv.level))
            .collect::<Vec<_>>();
        ranks.sort();
        assert_eq!(ranks.first().copied(), Some(1));
    }

    #[test]
    fn empty_survey_returns_action_advisory() {
        let report = SurveyReport::new("node-pi5-warden", vec![]);
        let advisories = analyze_survey(&report);
        assert_eq!(advisories.advisories.len(), 1);
        assert!(matches!(
            advisories.advisories[0].source,
            AdvisorySource::NoObservations
        ));
    }

    #[test]
    fn summarize_advisories_reports_all_entries() {
        let observation = CrateObservation {
            path: "tmp/shell-app".into(),
            name: "shell-app".into(),
            purpose: None,
            status: CrateStatus::Shell,
            key_entrypoints: vec!["src/lib.rs".to_string()],
            test_surface: vec![],
            dependencies: vec![],
            dev_patterns: vec![],
            observed_at: Utc::now(),
        };

        let report = SurveyReport::new("node-pi5-warden", vec![observation]);
        let advisory_report = analyze_survey(&report);
        let summary = summarize_advisories(&advisory_report);
        assert!(summary.contains("shell-app"));
        assert!(summary.contains("advisories=2"));
    }
}
