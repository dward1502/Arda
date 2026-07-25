#![cfg(feature = "full-cli")]

use arda_aule::prometheus::autopilot::{Objective, Priority};

#[test]
fn autopilot_surface_is_attached_to_aule() {
    let objective = Objective {
        id: "obj-smoke".to_string(),
        statement: "verify consolidated autopilot surface".to_string(),
        constraints: Vec::new(),
        deadline: None,
        success_criteria: Vec::new(),
        tags: vec!["observability".to_string()],
    };

    assert_eq!(objective.id, "obj-smoke");
    assert!(matches!(Priority::High, Priority::High));
}
