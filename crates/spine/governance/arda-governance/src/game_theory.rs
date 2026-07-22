// sigil: REPAIR
//! Game Theory Agent Selection
//!
//! Agent selection based on historical performance plus explicit
//! capability/task-class metadata. This is still a local heuristic and
//! policy-backed fallback surface, not autonomous game-theory consensus.

use arda_core::task::{Task, TaskStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::versions::{legacy_game_theory_policy_version, GAME_THEORY_POLICY_VERSION};
use crate::{
    love_equation_score, normalize_legacy_unit_or_percent, profile_joulework, triad_validate,
};

pub const GAME_THEORY_SELECTION_POLICY_VERSION: &str = GAME_THEORY_POLICY_VERSION;
pub const GAME_THEORY_HEURISTIC_LABEL: &str =
    "capability_weighted_heuristic_not_autonomous_consensus";
pub const GAME_THEORY_FALLBACK_LABEL: &str = "policy_backed_fallback_not_autonomous_consensus";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScore {
    pub name: String,
    pub total_tasks: u32,
    pub successful: u32,
    pub average_resonance: f64,
    pub average_love_equation: f64,
    pub joule_honesty: f64,
    pub triad_pass_rate: f64,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub action_classes: Vec<String>,
}

impl AgentScore {
    pub fn with_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.capabilities = capabilities.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_action_classes(
        mut self,
        action_classes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.action_classes = action_classes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameTheorySelectionKind {
    HistoricalWeightedHeuristic,
    CapabilityWeightedHeuristic,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameTheoryPolicy {
    pub kind: GameTheorySelectionKind,
    pub label: String,
    pub autonomous_consensus: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameTheoryConfidenceBand {
    /// No eligible weighted candidates or zero confidence.
    #[default]
    NoData,
    /// Confidence greater than zero and below 0.50.
    Low,
    /// Confidence from 0.50 (inclusive) to 0.75 (exclusive).
    Medium,
    /// Confidence at or above 0.75.
    High,
}

impl GameTheoryConfidenceBand {
    fn from_confidence(confidence: f64) -> Self {
        if confidence <= 0.0 {
            Self::NoData
        } else if confidence < 0.50 {
            Self::Low
        } else if confidence < 0.75 {
            Self::Medium
        } else {
            Self::High
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameTheorySelectionResult {
    pub selected_agent: Option<String>,
    pub policy: GameTheoryPolicy,
    pub candidate_count: usize,
    pub filtered_out_count: usize,
    pub fallback_reason: Option<String>,
    pub confidence: f64,
    #[serde(default)]
    pub confidence_band: GameTheoryConfidenceBand,
    #[serde(default = "legacy_game_theory_policy_version")]
    pub selection_policy_version: String,
}

impl GameTheorySelectionResult {
    fn capability_weighted_heuristic(
        selected_agent: Option<String>,
        candidate_count: usize,
        filtered_out_count: usize,
        confidence: f64,
    ) -> Self {
        Self {
            selected_agent,
            policy: GameTheoryPolicy {
                kind: GameTheorySelectionKind::CapabilityWeightedHeuristic,
                label: GAME_THEORY_HEURISTIC_LABEL.to_string(),
                autonomous_consensus: false,
            },
            candidate_count,
            filtered_out_count,
            fallback_reason: None,
            confidence,
            confidence_band: GameTheoryConfidenceBand::from_confidence(confidence),
            selection_policy_version: GAME_THEORY_SELECTION_POLICY_VERSION.to_string(),
        }
    }

    fn fallback(
        selected_agent: Option<String>,
        candidate_count: usize,
        filtered_out_count: usize,
        reason: &str,
    ) -> Self {
        Self {
            selected_agent,
            policy: GameTheoryPolicy {
                kind: GameTheorySelectionKind::Fallback,
                label: GAME_THEORY_FALLBACK_LABEL.to_string(),
                autonomous_consensus: false,
            },
            candidate_count,
            filtered_out_count,
            fallback_reason: Some(reason.to_string()),
            confidence: 0.0,
            confidence_band: GameTheoryConfidenceBand::NoData,
            selection_policy_version: GAME_THEORY_SELECTION_POLICY_VERSION.to_string(),
        }
    }
}

pub struct GameTheory {
    scores: HashMap<String, AgentScore>,
    fallback_agent: Option<String>,
}

impl GameTheory {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            fallback_agent: Some("athena".to_string()),
        }
    }

    pub fn with_fallback_agent(mut self, fallback_agent: Option<impl Into<String>>) -> Self {
        self.fallback_agent = fallback_agent.map(Into::into);
        self
    }

    pub fn select_agent(&self, task_type: &str) -> Option<String> {
        self.select_agent_with_policy(task_type).selected_agent
    }

    pub fn select_agent_with_policy(&self, task_type: &str) -> GameTheorySelectionResult {
        self.select_agent_for_action_class(task_type, None)
    }

    pub fn select_agent_for_action_class(
        &self,
        task_type: &str,
        action_class: Option<&str>,
    ) -> GameTheorySelectionResult {
        let scored_candidates: Vec<_> = self
            .scores
            .iter()
            .filter(|(_, score)| score.total_tasks > 0)
            .collect();
        let candidate_count = scored_candidates.len();
        if candidate_count == 0 {
            return GameTheorySelectionResult::fallback(
                self.fallback_agent.clone(),
                0,
                0,
                "no_candidates",
            );
        }

        let candidates: Vec<_> = scored_candidates
            .into_iter()
            .filter(|(_, score)| score_supports_task(score, task_type, action_class))
            .collect();
        let filtered_out_count = candidate_count.saturating_sub(candidates.len());

        if candidates.is_empty() {
            return GameTheorySelectionResult::fallback(
                self.fallback_agent.clone(),
                candidate_count,
                filtered_out_count,
                "no_capable_candidates",
            );
        }

        let total_weight: f64 = candidates
            .iter()
            .map(|(_, score)| selection_weight(score))
            .sum();
        if total_weight <= 0.0 || !total_weight.is_finite() {
            return GameTheorySelectionResult::fallback(
                self.fallback_agent.clone(),
                candidate_count,
                filtered_out_count,
                "no_positive_weight",
            );
        }

        let mut best: Option<(&String, f64)> = None;
        for (name, score) in candidates {
            let weight = selection_weight(score);
            if weight <= 0.0 || !weight.is_finite() {
                continue;
            }
            match best {
                Some((best_name, best_weight))
                    if weight < best_weight || (weight == best_weight && name >= best_name) => {}
                _ => best = Some((name, weight)),
            }
        }

        if let Some((name, weight)) = best {
            let confidence = (weight / total_weight).clamp(0.0, 1.0);
            return GameTheorySelectionResult::capability_weighted_heuristic(
                Some(name.clone()),
                candidate_count,
                filtered_out_count,
                confidence,
            );
        }

        GameTheorySelectionResult::fallback(
            self.fallback_agent.clone(),
            candidate_count,
            filtered_out_count,
            "no_selected_weight",
        )
    }

    pub fn update_score(&mut self, task: &Task) {
        let agent = task.assigned_agent.as_deref().unwrap_or("unknown");
        let entry = self.scores.entry(agent.to_string()).or_insert(AgentScore {
            name: agent.to_string(),
            total_tasks: 0,
            successful: 0,
            average_resonance: 0.5,
            average_love_equation: 0.5,
            joule_honesty: 1.0,
            triad_pass_rate: 0.5,
            capabilities: vec![task.task_type.clone()],
            action_classes: Vec::new(),
        });

        if !entry
            .capabilities
            .iter()
            .any(|capability| capability == &task.task_type)
        {
            entry.capabilities.push(task.task_type.clone());
        }

        entry.total_tasks += 1;
        if matches!(task.status, TaskStatus::Complete) {
            entry.successful += 1;
        }

        let triad = triad_validate(task, None);
        let love = love_equation_score(task);
        let joule = profile_joulework(task);

        let success_rate = entry.successful as f64 / entry.total_tasks as f64;
        entry.average_resonance = success_rate;
        entry.average_love_equation =
            rolling_average(entry.average_love_equation, love.score, entry.total_tasks);
        entry.joule_honesty =
            rolling_average(entry.joule_honesty, joule.honesty_ratio, entry.total_tasks);
        entry.triad_pass_rate = rolling_average(
            entry.triad_pass_rate,
            if triad.passed { 1.0 } else { 0.0 },
            entry.total_tasks,
        );
    }
}

fn score_supports_task(score: &AgentScore, task_type: &str, action_class: Option<&str>) -> bool {
    let capability_match = score.capabilities.is_empty()
        || score
            .capabilities
            .iter()
            .any(|capability| capability == task_type || capability == "*");
    let action_class_match = match action_class {
        Some(class) => {
            score.action_classes.is_empty()
                || score
                    .action_classes
                    .iter()
                    .any(|candidate_class| candidate_class == class || candidate_class == "*")
        }
        None => true,
    };
    capability_match && action_class_match
}

fn selection_weight(score: &AgentScore) -> f64 {
    let weight = normalize_legacy_unit_or_percent(score.average_resonance).get() * 0.45
        + normalize_legacy_unit_or_percent(score.average_love_equation).get() * 0.25
        + normalize_legacy_unit_or_percent(score.joule_honesty).get() * 0.15
        + normalize_legacy_unit_or_percent(score.triad_pass_rate).get() * 0.15;
    weight.clamp(0.0, 1.0)
}

fn rolling_average(current: f64, sample: f64, total_tasks: u32) -> f64 {
    if total_tasks <= 1 {
        return sample;
    }
    let prior_weight = (total_tasks - 1) as f64;
    ((current * prior_weight) + sample) / total_tasks as f64
}

impl Default for GameTheory {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate game theory score for task
pub fn game_theory_score(task: &Task) -> f64 {
    let base = match task.status {
        TaskStatus::Complete => 1.0,
        TaskStatus::Running => 0.5,
        TaskStatus::Pending => 0.2,
        TaskStatus::Failed { .. } => 0.05,
        TaskStatus::Retry { attempt, .. } => 0.3 / attempt.max(1) as f64,
    };
    let triad = triad_validate(task, None);
    let love = love_equation_score(task);
    let joule = profile_joulework(task);
    let governance_bonus = if triad.passed { 0.12 } else { -0.18 }
        + (love.score * 0.10)
        + (joule.honesty_ratio * 0.08);
    (base + governance_bonus).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::Task;

    fn scored_agent(name: &str, capability: &str, weight_seed: f64) -> AgentScore {
        AgentScore {
            name: name.to_string(),
            total_tasks: 8,
            successful: 8,
            average_resonance: weight_seed,
            average_love_equation: weight_seed / 100.0,
            joule_honesty: weight_seed / 100.0,
            triad_pass_rate: weight_seed / 100.0,
            capabilities: vec![capability.to_string()],
            action_classes: Vec::new(),
        }
    }

    #[test]
    fn update_score_tracks_governance_fields_and_capability() {
        let mut tracker = GameTheory::new();
        let mut task = Task::new(
            "athena synthesize client brief because evidence matters",
            "analyze",
        );
        task.assigned_agent = Some("athena".to_string());
        task.status = TaskStatus::Complete;
        task.joule_cost_estimated = 4.0;
        task.joule_cost_actual = 4.2;

        tracker.update_score(&task);
        let score = tracker.scores.get("athena").expect("score");
        assert!(score.average_love_equation > 0.0);
        assert!(score.joule_honesty > 0.0);
        assert!(score.triad_pass_rate >= 0.0);
        assert_eq!(score.capabilities, vec!["analyze".to_string()]);
    }

    #[test]
    fn select_agent_falls_back_when_candidate_weights_are_zero() {
        let mut tracker = GameTheory::new();
        tracker.scores.insert(
            "athena_zero".to_string(),
            AgentScore {
                name: "athena_zero".to_string(),
                total_tasks: 1,
                successful: 0,
                average_resonance: 0.0,
                average_love_equation: 0.0,
                joule_honesty: 0.0,
                triad_pass_rate: 0.0,
                capabilities: vec!["analyze".to_string()],
                action_classes: Vec::new(),
            },
        );

        let result = tracker.select_agent_with_policy("analyze");
        assert_eq!(result.selected_agent, Some("athena".to_string()));
        assert_eq!(result.policy.kind, GameTheorySelectionKind::Fallback);
        assert_eq!(result.policy.label, GAME_THEORY_FALLBACK_LABEL);
        assert_eq!(
            result.fallback_reason.as_deref(),
            Some("no_positive_weight")
        );
        assert_eq!(result.candidate_count, 1);
        assert_eq!(result.filtered_out_count, 0);
        assert_eq!(result.confidence, 0.0);
        assert!(!result.policy.autonomous_consensus);
        assert_eq!(tracker.select_agent("analyze"), Some("athena".to_string()));
    }

    #[test]
    fn selection_policy_discloses_capability_weighted_heuristic() {
        let mut tracker = GameTheory::new();
        tracker.scores.insert(
            "apollo".to_string(),
            scored_agent("apollo", "execute", 100.0),
        );

        let result = tracker.select_agent_with_policy("execute");
        assert_eq!(result.selected_agent, Some("apollo".to_string()));
        assert_eq!(
            result.policy.kind,
            GameTheorySelectionKind::CapabilityWeightedHeuristic
        );
        assert_eq!(result.policy.label, GAME_THEORY_HEURISTIC_LABEL);
        assert!(!result.policy.autonomous_consensus);
        assert_eq!(result.candidate_count, 1);
        assert_eq!(result.filtered_out_count, 0);
        assert!(result.fallback_reason.is_none());
        assert_eq!(result.confidence, 1.0);
        assert_eq!(result.confidence_band, GameTheoryConfidenceBand::High);
        assert_eq!(
            result.selection_policy_version,
            GAME_THEORY_SELECTION_POLICY_VERSION
        );
    }

    #[test]
    fn selects_best_capable_candidate_after_filtering() {
        let mut tracker = GameTheory::new();
        tracker.scores.insert(
            "athena".to_string(),
            scored_agent("athena", "analyze", 80.0),
        );
        tracker.scores.insert(
            "apollo".to_string(),
            scored_agent("apollo", "execute", 70.0),
        );
        tracker.scores.insert(
            "charon".to_string(),
            scored_agent("charon", "execute", 95.0),
        );

        let result = tracker.select_agent_with_policy("execute");
        assert_eq!(result.selected_agent, Some("charon".to_string()));
        assert_eq!(result.candidate_count, 3);
        assert_eq!(result.filtered_out_count, 1);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn falls_back_when_no_candidate_has_required_capability() {
        let mut tracker = GameTheory::new();
        tracker.scores.insert(
            "apollo".to_string(),
            scored_agent("apollo", "execute", 100.0),
        );

        let result = tracker.select_agent_with_policy("summarize");
        assert_eq!(result.selected_agent, Some("athena".to_string()));
        assert_eq!(result.policy.kind, GameTheorySelectionKind::Fallback);
        assert_eq!(result.candidate_count, 1);
        assert_eq!(result.filtered_out_count, 1);
        assert_eq!(
            result.fallback_reason.as_deref(),
            Some("no_capable_candidates")
        );
    }

    #[test]
    fn action_class_filtering_is_auditable() {
        let mut tracker = GameTheory::new();
        tracker.scores.insert(
            "hades".to_string(),
            scored_agent("hades", "remove", 90.0).with_action_classes(["destructive_delete"]),
        );
        tracker.scores.insert(
            "apollo".to_string(),
            scored_agent("apollo", "remove", 80.0).with_action_classes(["routine_maintenance"]),
        );

        let result = tracker.select_agent_for_action_class("remove", Some("destructive_delete"));
        assert_eq!(result.selected_agent, Some("hades".to_string()));
        assert_eq!(result.candidate_count, 2);
        assert_eq!(result.filtered_out_count, 1);
        assert!(result.fallback_reason.is_none());
    }
}
