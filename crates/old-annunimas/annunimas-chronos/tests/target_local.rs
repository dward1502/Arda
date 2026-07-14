use annunimas_chronos::{
    summarize_state_feeds, AuditOrchestrator, AuditTask, ResourceRequirements, Scheduler,
    TemporalTask,
};
use anyhow::Result;
use chrono::{Duration, TimeZone, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn unique_target_local_root(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "annunimas_chronos_{test_name}_{}",
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros())
    ))
}

fn scheduled_task(id: &str, priority: u32, cpu_percent: f64, memory_percent: f64) -> TemporalTask {
    TemporalTask {
        id: id.to_string(),
        name: format!("Target-local task {id}"),
        priority,
        scheduled_time: Utc
            .with_ymd_and_hms(2026, 5, 25, 10, 0, 0)
            .single()
            .unwrap_or_else(Utc::now),
        duration: Duration::minutes(30),
        resource_requirements: ResourceRequirements {
            cpu_percent,
            memory_percent,
            gpu_required: false,
            gpu_memory_mb: None,
        },
        metadata: HashMap::new(),
    }
}

fn audit_task(id: &str, category: &str) -> AuditTask {
    AuditTask {
        id: id.to_string(),
        name: format!("Target-local {category} audit"),
        category: category.to_string(),
        scheduled_time: Utc::now() - Duration::minutes(5),
        duration: Duration::minutes(15),
        priority: 1,
        metadata: HashMap::from([("scope".to_string(), "target-local".to_string())]),
    }
}

#[test]
fn state_feed_summary_uses_target_local_root_without_workspace_state() -> Result<()> {
    let root = unique_target_local_root("state_feeds");
    let state_dir = root.join("core/state");
    fs::create_dir_all(&state_dir)?;
    fs::write(
        state_dir.join("warden_guardhouse.json"),
        r#"{"authority":"warden_target_local","generated_at_utc":"2026-05-25T09:45:00Z"}"#,
    )?;
    fs::write(
        state_dir.join("memory_activity.json"),
        r#"{"authority":"mnemosyne_target_local","generated_at_utc":"2026-05-25T07:00:00Z"}"#,
    )?;
    fs::write(
        state_dir.join("plutus_runtime.json"),
        r#"{"authority":"plutus_target_local","generated_at_utc":"2026-05-25T09:50:00Z"}"#,
    )?;

    let snapshot_time = Utc
        .with_ymd_and_hms(2026, 5, 25, 10, 0, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let summary = summarize_state_feeds(&root, snapshot_time);

    assert_eq!(summary.feed_count, 4);
    assert_eq!(summary.present_count, 3);
    assert_eq!(summary.missing_count, 1);
    assert_eq!(summary.invalid_json_count, 0);
    assert_eq!(summary.stale_count, 1);
    assert_eq!(summary.max_age_seconds, Some(10_800));
    assert!(summary
        .anomalies
        .iter()
        .any(|anomaly| anomaly == "mnemosyne:stale_feed"));
    assert!(summary
        .anomalies
        .iter()
        .any(|anomaly| anomaly == "charon:missing_feed"));

    let warden = summary
        .domains
        .iter()
        .find(|domain| domain.source == "warden")
        .ok_or_else(|| anyhow::anyhow!("warden feed missing from summary"))?;
    assert_eq!(warden.path, "core/state/warden_guardhouse.json");
    assert_eq!(warden.authority.as_deref(), Some("warden_target_local"));
    assert_eq!(warden.freshness, "current");

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn scheduler_and_audit_orchestrator_run_from_target_local_fixtures() -> Result<()> {
    let mut scheduler = Scheduler::new(ResourceRequirements {
        cpu_percent: 60.0,
        memory_percent: 60.0,
        gpu_required: false,
        gpu_memory_mb: None,
    });
    scheduler.add_task(scheduled_task("fits", 10, 25.0, 25.0))?;
    scheduler.add_task(scheduled_task("oversized", 20, 90.0, 10.0))?;

    let schedule = scheduler.schedule();
    assert_eq!(schedule.scheduled.len(), 1);
    assert_eq!(schedule.scheduled[0].id, "fits");
    assert_eq!(schedule.conflicts.len(), 1);
    assert!(schedule.conflicts[0].contains("oversized"));

    let mut orchestrator = AuditOrchestrator::new();
    let task = audit_task("security-target-local", "security");
    orchestrator.add_task(&task)?;
    let next = orchestrator
        .schedule_next()?
        .ok_or_else(|| anyhow::anyhow!("target-local audit task was not ready"))?;
    assert_eq!(next.id, "security-target-local");

    let result = orchestrator.execute_audit(next)?;
    assert_eq!(result.task_id, "security-target-local");
    assert_eq!(result.status, "completed");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].category, "security");

    Ok(())
}
