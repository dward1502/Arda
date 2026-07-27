use arda_outpost_scout::survey;
use arda_outpost_scout::SurveyReport;

fn fixture_repo(root: std::path::PathBuf) {
    std::fs::create_dir_all(root.join("crates/sample-crate/src")).unwrap();
    std::fs::write(
        root.join("crates/sample-crate/Cargo.toml"),
        r#"[package]
name = "sample-crate"
version = "0.1.0"
description = "sample"
"#,
    )
    .unwrap();
    std::fs::write(root.join("crates/sample-crate/src/lib.rs"), "").unwrap();
    std::fs::create_dir_all(root.join("apps/sample-app/src")).unwrap();
    std::fs::write(
        root.join("apps/sample-app/Cargo.toml"),
        r#"[package]
name = "sample-app"
version = "0.1.0"
"#,
    )
    .unwrap();
}

#[test]
fn survey_repo_discovers_crates_and_apps() {
    let root = tempfile::tempdir().unwrap();
    fixture_repo(root.path().to_path_buf());
    let report = survey::survey_repo(root.path()).expect("survey");
    let names = report.observations.iter().map(|observation| observation.name.as_str()).collect::<Vec<_>>();
    assert!(names.contains(&"sample-crate"));
    assert!(names.contains(&"sample-app"));
    assert_eq!(report.source, "node-pi5-warden");
}
