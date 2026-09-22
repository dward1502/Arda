use super::*;
#[test]
fn exact_lookup_isolates_legacy_contracts_but_rejects_target_ambiguity() {
    let root = tempfile::tempdir().unwrap();
    let contract: ProjectContract = serde_json::from_str(include_str!(
        "../../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    let id = contract.identity.project_id.to_string();
    let good = serde_json::to_value(AttachedProject {
        contract,
        approval_id: "fixture".into(),
        proposal_id: "fixture".into(),
        idempotency_key: "fixture".into(),
    })
    .unwrap();
    let legacy = serde_json::json!({"contract":{"identity":{"project_id":"unrelated-legacy"}},"approval_id":"legacy"});
    fs::create_dir_all(root.path().join("data/workbench")).unwrap();
    let save = |entries: Vec<serde_json::Value>| {
        let bytes = serde_json::to_vec(
            &serde_json::json!({"schema_version":PROJECT_REGISTRY_VERSION,"projects":entries}),
        )
        .unwrap();
        fs::write(registry_path(root.path()), &bytes).unwrap();
        bytes
    };
    let before = save(vec![legacy.clone(), good.clone()]);
    assert_eq!(
        find_attached_project(root.path(), &id).unwrap().approval_id,
        "fixture"
    );
    assert_eq!(fs::read(registry_path(root.path())).unwrap(), before);
    assert!(load_registry(root.path()).is_err());
    assert!(find_attached_project(root.path(), "unrelated-legacy").is_err());
    save(vec![good.clone(), good.clone()]);
    assert!(find_attached_project(root.path(), &id).is_err());
    let mut malformed = good.clone();
    malformed["contract"]
        .as_object_mut()
        .unwrap()
        .remove("memory");
    save(vec![malformed]);
    assert!(find_attached_project(root.path(), &id).is_err());
    save(vec![good.clone(), serde_json::json!({"contract":{}})]);
    assert!(find_attached_project(root.path(), &id).is_err());
    let bytes = save(vec![good.clone()]);
    let raw = String::from_utf8(bytes).unwrap();
    for (needle, replacement) in [
        (
            "\"memory\":".to_string(),
            "\"memory\":null,\"memory\":".to_string(),
        ),
        (
            "\"project_id\":".to_string(),
            "\"project_id\":\"ambiguous\",\"project_id\":".to_string(),
        ),
        (
            "\"contract\":".to_string(),
            "\"contract\":null,\"contract\":".to_string(),
        ),
    ] {
        let duplicate = raw.replacen(&needle, &replacement, 1);
        assert_ne!(duplicate, raw);
        fs::write(registry_path(root.path()), duplicate).unwrap();
        assert!(find_attached_project(root.path(), &id).is_err());
    }
    let mut invalid = good.clone();
    let duplicate_command = invalid["contract"]["commands"][0].clone();
    invalid["contract"]["commands"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_command);
    save(vec![invalid]);
    assert!(find_attached_project(root.path(), &id).is_err());
    save(vec![good]);
    assert!(find_attached_project(root.path(), "absent").is_err());
}
