// Temporal scheduling engine for Chronos agent
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a scheduled task with temporal constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalTask {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub scheduled_time: DateTime<Utc>,
    pub duration: Duration,
    pub resource_requirements: ResourceRequirements,
    pub metadata: HashMap<String, String>,
}

/// Resource requirements for task scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub gpu_required: bool,
    pub gpu_memory_mb: Option<u64>,
}

/// Scheduling result
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub scheduled: Vec<TemporalTask>,
    pub optimized: bool,
    pub conflicts: Vec<String>,
}

/// Main scheduler for temporal tasks
pub struct Scheduler {
    tasks: Vec<TemporalTask>,
    resource_capacity: ResourceRequirements,
}

impl Scheduler {
    pub fn new(resource_capacity: ResourceRequirements) -> Self {
        Self {
            tasks: Vec::new(),
            resource_capacity,
        }
    }

    pub fn add_task(&mut self, task: TemporalTask) -> Result<()> {
        if task.duration.num_seconds() <= 0 {
            anyhow::bail!("Task duration must be positive");
        }

        self.tasks.push(task);
        Ok(())
    }

    pub fn schedule(&self) -> ScheduleResult {
        let mut scheduled = Vec::new();
        let mut conflicts = Vec::new();

        // Sort tasks by priority (higher first) then by scheduled time
        let mut sorted_tasks: Vec<_> = self.tasks.iter().collect();
        sorted_tasks.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.scheduled_time.cmp(&b.scheduled_time))
        });

        // Simple scheduling algorithm (can be enhanced with more sophisticated algorithms)
        for task in sorted_tasks {
            // Check resource availability
            if self.can_schedule(task) {
                scheduled.push(task.clone());
            } else {
                conflicts.push(format!(
                    "Task {} conflicts with resource constraints",
                    task.id
                ));
            }
        }

        ScheduleResult {
            scheduled,
            optimized: true,
            conflicts,
        }
    }

    fn can_schedule(&self, task: &TemporalTask) -> bool {
        let total_cpu = task.resource_requirements.cpu_percent;
        let total_memory = task.resource_requirements.memory_percent;

        total_cpu <= self.resource_capacity.cpu_percent
            && total_memory <= self.resource_capacity.memory_percent
    }

    /// Get pending tasks for a specific time window
    pub fn get_pending_tasks(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&TemporalTask> {
        self.tasks
            .iter()
            .filter(|task| task.scheduled_time >= start && task.scheduled_time < end)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let capacity = ResourceRequirements {
            cpu_percent: 100.0,
            memory_percent: 100.0,
            gpu_required: false,
            gpu_memory_mb: None,
        };
        let scheduler = Scheduler::new(capacity);
        assert_eq!(scheduler.tasks.len(), 0);
    }

    #[test]
    fn test_add_task() {
        let mut scheduler = Scheduler::new(ResourceRequirements {
            cpu_percent: 100.0,
            memory_percent: 100.0,
            gpu_required: false,
            gpu_memory_mb: None,
        });

        let task = TemporalTask {
            id: "task-1".to_string(),
            name: "Test Task".to_string(),
            priority: 1,
            scheduled_time: Utc::now(),
            duration: Duration::hours(1),
            resource_requirements: ResourceRequirements {
                cpu_percent: 50.0,
                memory_percent: 50.0,
                gpu_required: false,
                gpu_memory_mb: None,
            },
            metadata: HashMap::new(),
        };

        scheduler.add_task(task).unwrap();
        assert_eq!(scheduler.tasks.len(), 1);
    }

    #[test]
    fn test_schedule() {
        let capacity = ResourceRequirements {
            cpu_percent: 100.0,
            memory_percent: 100.0,
            gpu_required: false,
            gpu_memory_mb: None,
        };
        let mut scheduler = Scheduler::new(capacity);

        let task = TemporalTask {
            id: "task-1".to_string(),
            name: "Test Task".to_string(),
            priority: 1,
            scheduled_time: Utc::now(),
            duration: Duration::hours(1),
            resource_requirements: ResourceRequirements {
                cpu_percent: 50.0,
                memory_percent: 50.0,
                gpu_required: false,
                gpu_memory_mb: None,
            },
            metadata: HashMap::new(),
        };

        scheduler.add_task(task).unwrap();
        let result = scheduler.schedule();
        assert_eq!(result.scheduled.len(), 1);
        assert_eq!(result.conflicts.len(), 0);
    }

    #[test]
    fn test_resource_conflict() {
        let capacity = ResourceRequirements {
            cpu_percent: 50.0,
            memory_percent: 50.0,
            gpu_required: false,
            gpu_memory_mb: None,
        };
        let mut scheduler = Scheduler::new(capacity);

        // First task that fits
        let task1 = TemporalTask {
            id: "task-1".to_string(),
            name: "Task 1".to_string(),
            priority: 1,
            scheduled_time: Utc::now(),
            duration: Duration::hours(1),
            resource_requirements: ResourceRequirements {
                cpu_percent: 30.0,
                memory_percent: 30.0,
                gpu_required: false,
                gpu_memory_mb: None,
            },
            metadata: HashMap::new(),
        };

        // Second task that exceeds capacity
        let task2 = TemporalTask {
            id: "task-2".to_string(),
            name: "Task 2".to_string(),
            priority: 2,
            scheduled_time: Utc::now(),
            duration: Duration::hours(1),
            resource_requirements: ResourceRequirements {
                cpu_percent: 60.0,
                memory_percent: 60.0,
                gpu_required: false,
                gpu_memory_mb: None,
            },
            metadata: HashMap::new(),
        };

        scheduler.add_task(task1).unwrap();
        scheduler.add_task(task2).unwrap();
        let result = scheduler.schedule();
        assert_eq!(result.scheduled.len(), 1);
        assert_eq!(result.conflicts.len(), 1);
    }
}
