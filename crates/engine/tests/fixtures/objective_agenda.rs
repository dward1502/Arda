use arda_engine::objectives::{NewLeaf, NewObjective, ObjectiveStore};
use std::path::Path;

pub fn seed(root: &Path, id: &str, owner: &str, state: &str, priority: i64) -> ObjectiveStore {
    let path = root.join("data/arda/objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    if store.objective(id).unwrap().is_none() {
        store
            .create_authenticated_objective(
                NewObjective {
                    id: id.into(),
                    source_id: format!("source-{id}"),
                    idempotency_key: format!("intake-{id}"),
                    operator_id: owner.into(),
                    text: format!("Current objective {id}"),
                    priority,
                    projects: vec![],
                    leaves: vec![NewLeaf {
                        id: format!("leaf-{id}"),
                        project_id: None,
                        workspace_root: root.display().to_string(),
                        authority: "read_only".into(),
                        dependencies: vec![],
                        execution: None,
                    }],
                },
                1,
            )
            .unwrap();
    }
    // Persisted state fixture: mutation/claim semantics are covered by ObjectiveStore tests.
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE objectives SET state = ?1 WHERE id = ?2",
            [state, id],
        )
        .unwrap();
    store
}
