use arda_engine::{
    next_action::publish_next_action_projection, operator_projection::publish_operator_projection,
};
use chrono::Utc;
use std::fs;

#[test]
fn live_projections_never_parse_retired_queue_or_schedule_history() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("data/runs")).unwrap();
    let legacy = root.path().join("core/projects/tasks");
    fs::create_dir_all(&legacy).unwrap();
    for file in ["queue.jsonl", "schedules.jsonl"] {
        fs::write(
            legacy.join(file),
            "not valid JSON: retired history is not live input\n",
        )
        .unwrap();
    }
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert!(projection.objectives.is_empty());
    assert!(projection
        .dependencies
        .iter()
        .any(|d| d.dependency_id == "objective_store"
            && d.health == arda_core::operator_projection::DependencyHealth::NotConfigured));
    let next = publish_next_action_projection(root.path(), "operator:test", Utc::now()).unwrap();
    assert!(next.selected.is_none());
    assert!(
        !root.path().join("data/arda/objectives.sqlite3").exists(),
        "a read must not create authority"
    );
}

#[test]
fn historical_checkpoints_are_not_loaded_into_the_current_projection() {
    let root = tempfile::tempdir().unwrap();
    let history = root.path().join("data/runs/historical");
    fs::create_dir_all(&history).unwrap();
    fs::write(
        history.join("checkpoint.json"),
        "retained historical checkpoint, not a current input",
    )
    .unwrap();
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert!(projection.runs.is_empty());
}
