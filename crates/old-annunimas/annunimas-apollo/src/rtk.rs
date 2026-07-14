// sigil: REPAIR
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub dependencies: Vec<String>,
    pub estimated_cost: f64,
    pub priority: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationStrategy {
    Greedy,
    Dynamic,
    Heuristic,
}

pub struct RtkOptimizer {
    strategy: OptimizationStrategy,
    completed: HashMap<String, f64>,
}

impl RtkOptimizer {
    pub fn new() -> Self {
        Self {
            strategy: OptimizationStrategy::Heuristic,
            completed: HashMap::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: OptimizationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn optimize(&self, tasks: &[TaskNode]) -> Vec<String> {
        match self.strategy {
            OptimizationStrategy::Greedy => self.greedy_schedule(tasks),
            OptimizationStrategy::Dynamic => self.dynamic_program(tasks),
            OptimizationStrategy::Heuristic => self.heuristic_schedule(tasks),
        }
    }

    fn greedy_schedule(&self, tasks: &[TaskNode]) -> Vec<String> {
        let mut sorted = tasks.to_vec();
        sorted.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().map(|t| t.id).collect()
    }

    fn heuristic_schedule(&self, tasks: &[TaskNode]) -> Vec<String> {
        let mut scheduled = Vec::new();
        let mut completed_ids = std::collections::HashSet::new();
        let mut remaining: HashMap<String, &TaskNode> =
            tasks.iter().map(|t| (t.id.clone(), t)).collect();

        while !remaining.is_empty() {
            let mut ready_ids: Vec<String> = remaining
                .iter()
                .filter(|(_, task)| {
                    task.dependencies
                        .iter()
                        .all(|d| completed_ids.contains(d.as_str()))
                })
                .map(|(id, _)| id.clone())
                .collect();

            if ready_ids.is_empty() {
                if let Some(fallback_id) = remaining.keys().next().cloned() {
                    if let Some(node) = remaining.remove(&fallback_id) {
                        scheduled.push(node.id.clone());
                        completed_ids.insert(node.id.clone());
                    }
                }
                continue;
            }

            ready_ids.sort_by(|a, b| {
                let pa = remaining.get(a).map(|t| t.priority).unwrap_or(0.0);
                let pb = remaining.get(b).map(|t| t.priority).unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            });

            for id in ready_ids {
                if let Some(node) = remaining.remove(&id) {
                    scheduled.push(node.id.clone());
                    completed_ids.insert(node.id.clone());
                }
            }
        }

        scheduled
    }

    fn dynamic_program(&self, tasks: &[TaskNode]) -> Vec<String> {
        self.heuristic_schedule(tasks)
    }

    pub fn record_completion(&mut self, task_id: &str, actual_cost: f64) {
        self.completed.insert(task_id.to_string(), actual_cost);
    }

    pub fn average_cost(&self) -> f64 {
        if self.completed.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.completed.values().sum();
        sum / self.completed.len() as f64
    }
}

impl Default for RtkOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
