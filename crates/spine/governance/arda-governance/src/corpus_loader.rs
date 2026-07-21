use crate::corpus::{CorpusDomain, GovernanceCorpus, PhilosopherContext, PhilosopherId, PhilosopherVerdict};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct LoadedCorpus {
    pub id: PhilosopherId,
    pub name: String,
    pub domain: CorpusDomain,
    pub description: String,
    
    patterns: Vec<Regex>,
    weights: HashMap<String, f32>,           // original regex string → weight
    veto_classes: HashMap<String, String>,
}

impl LoadedCorpus {
    pub fn load(philosopher_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let name = philosopher_dir.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Load patterns.regex
        let patterns_text = std::fs::read_to_string(philosopher_dir.join("patterns.regex"))?;
        let patterns: Vec<Regex> = patterns_text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .filter_map(|line| Regex::new(line.trim()).ok())
            .collect();

        // Load weights.toml (simplified parser — expand as needed)
        let weights_toml: toml::Value = toml::from_str(
            &std::fs::read_to_string(philosopher_dir.join("weights.toml"))?
        )?;

        let mut weights = HashMap::new();
        let mut veto_classes = HashMap::new();

        if let Some(patterns_arr) = weights_toml.get("pattern").and_then(|v| v.as_array()) {
            for p in patterns_arr {
                if let (Some(regex), Some(w), Some(veto)) = (
                    p.get("regex").and_then(|v| v.as_str()),
                    p.get("weight").and_then(|v| v.as_float()),
                    p.get("veto_class").and_then(|v| v.as_str()),
                ) {
                    weights.insert(regex.to_string(), w as f32);
                    veto_classes.insert(regex.to_string(), veto.to_string());
                }
            }
        }

        Ok(Self {
            id: PhilosopherId(name.clone()),
            name,
            domain: CorpusDomain::Philosophical, // map properly in real version
            description: "Loaded from philosopher_data corpus".to_string(),
            patterns,
            weights,
            veto_classes,
        })
    }

    pub fn quick_check(&self, ctx: &PhilosopherContext) -> Option<PhilosopherVerdict> {
        let text = format!("{} {}", ctx.task.description, ctx.task.task_type);
        let mut total_score = 0.0f32;
        let mut matched = 0;

        for pat in &self.patterns {
            if pat.is_match(&text) {
                matched += 1;
                if let Some(w) = self.weights.get(pat.as_str()) {
                    total_score += w;
                }
            }
        }

        if matched == 0 {
            return None; // no strong signal → fall through to deeper validation
        }

        let avg_score = total_score / matched.max(1) as f32;
        let passed = avg_score > 0.5;

        Some(PhilosopherVerdict {
            passed,
            score: avg_score.clamp(0.0, 1.0),
            confidence: (matched as f32 / self.patterns.len() as f32).clamp(0.3, 0.9),
            reason: format!("Matched {} deterministic patterns from {} corpus", matched, self.name),
            veto_code: if !passed { Some(format!("{}_PATTERN_VETO", self.name.to_uppercase())) } else { None },
            metadata: serde_json::json!({ "matched_patterns": matched }),
        })
    }
}