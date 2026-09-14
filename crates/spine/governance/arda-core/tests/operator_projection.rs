use arda_core::operator_projection::{
    DependencyHealth, ObjectiveProjection, ObjectiveStatus, OperatorProjection,
    OperatorProjectionError, ProjectionAuthority, ProjectionFreshness,
};

fn fixture() -> OperatorProjection {
    OperatorProjection::from_json_str(include_str!(
        "../../../../../spec/operator-projection/v1/fixtures/valid-operator-projection.json"
    ))
    .expect("valid P9.1 operator projection fixture")
}

#[test]
fn canonical_fixture_covers_every_p9_1_projection_lane() {
    let projection = fixture();

    assert_eq!(
        projection.schema_version,
        OperatorProjection::SCHEMA_VERSION
    );
    assert_eq!(projection.authority, ProjectionAuthority::ReadOnly);
    assert_eq!(projection.objectives.len(), 1);
    assert_eq!(projection.runs.len(), 1);
    assert_eq!(projection.runs[0].nodes.len(), 2);
    assert_eq!(projection.runs[0].workers.len(), 1);
    assert!(projection
        .capabilities
        .iter()
        .any(|capability| { capability.selected && !capability.selection_reasons.is_empty() }));
    assert_eq!(projection.pending_approvals.len(), 1);
    assert_eq!(projection.councils.len(), 1);
    assert_eq!(projection.personal_operations.captures, 3);
    assert_eq!(projection.personal_operations.resumable_items, 1);
    assert_eq!(projection.personal_operations.reminders.len(), 1);
    assert!(projection.joulework.remaining_joules >= 0.0);
    assert!(!projection.evidence.is_empty());
    assert_eq!(projection.communications.len(), 1);
    assert!(projection
        .dependencies
        .iter()
        .any(|dependency| { dependency.health == DependencyHealth::Degraded }));
    assert!(projection.capabilities.iter().any(|capability| {
        capability.optional && capability.health == DependencyHealth::Unavailable
    }));
}

#[test]
fn objective_projection_constructor_defaults_additive_control_fields() {
    let objective = ObjectiveProjection::new(
        "objective-1",
        Some("project-1".to_string()),
        "Repair the queue",
        ObjectiveStatus::Pending,
    );

    assert_eq!(objective.objective_id, "objective-1");
    assert_eq!(objective.project_id.as_deref(), Some("project-1"));
    assert_eq!(objective.title, "Repair the queue");
    assert_eq!(objective.status, ObjectiveStatus::Pending);
    assert!(objective.current_task_id.is_none());
    assert!(objective.current_run_id.is_none());
    assert!(objective.current_node_id.is_none());
    assert!(objective.evidence.is_empty());
    assert!(objective.next_continuation.is_none());
    assert!(objective.next_wake_at.is_none());
    assert!(objective.provider_route.is_none());
    assert!(objective.budget.is_none());
    assert!(objective.blocker.is_none());
}

#[test]
fn projection_is_read_only_and_rejects_stale_truth_marked_fresh() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../spec/operator-projection/v1/fixtures/valid-operator-projection.json"
    ))
    .unwrap();

    value["authority"] = serde_json::json!("transition_state");
    let error = OperatorProjection::from_json_str(&value.to_string()).unwrap_err();
    assert!(matches!(error, OperatorProjectionError::InvalidJson(_)));

    let mut projection = fixture();
    projection.dependencies[0].health = DependencyHealth::Stale;
    projection.dependencies[0].freshness = ProjectionFreshness::Fresh;
    assert_eq!(
        projection.validate().unwrap_err(),
        OperatorProjectionError::InconsistentFreshness {
            lane: "dependency:arda-harness".to_string(),
            health: DependencyHealth::Stale,
            freshness: ProjectionFreshness::Fresh,
        }
    );
}

#[test]
fn projection_rejects_duplicate_ids_and_invalid_budget_confidence() {
    let mut projection = fixture();
    projection.objectives.push(projection.objectives[0].clone());
    assert_eq!(
        projection.validate().unwrap_err(),
        OperatorProjectionError::DuplicateIdentifier {
            lane: "objective".to_string(),
            id: "objective-p9".to_string(),
        }
    );

    let mut projection = fixture();
    projection.joulework.source_confidence = 1.1;
    assert_eq!(
        projection.validate().unwrap_err(),
        OperatorProjectionError::InvalidConfidence {
            lane: "joulework".to_string(),
            value: "1.1".to_string(),
        }
    );
}

#[test]
fn objective_control_rejects_a_node_outside_its_current_run() {
    let mut projection = fixture();
    projection.objectives[0].current_run_id = Some("run-p9".to_string());
    projection.objectives[0].current_node_id = Some("missing-node".to_string());

    assert_eq!(
        projection.validate().unwrap_err(),
        OperatorProjectionError::MissingReference {
            lane: "objective:objective-p9".to_string(),
            field: "current_node_id".to_string(),
            id: "missing-node".to_string(),
        }
    );
}

#[test]
fn objective_control_rejects_a_run_owned_by_another_objective() {
    let mut projection = fixture();
    let mut foreign = projection.objectives[0].clone();
    foreign.objective_id = "different-objective".to_string();
    foreign.current_run_id = Some("run-p9".to_string());
    foreign.current_node_id = Some("plan".to_string());
    projection.objectives.push(foreign);

    assert_eq!(
        projection.validate().unwrap_err(),
        OperatorProjectionError::MissingReference {
            lane: "objective:different-objective".to_string(),
            field: "current_run_id".to_string(),
            id: "run-p9".to_string(),
        }
    );
}
