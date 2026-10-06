#[test]
fn test_validate_snapshot_owner_paths_same_mount() {
    use std::path::Path;

    let workspace = Path::new("/var/home/mythos/Eregion/Arda");
    let state = Path::new("/var/home/mythos/.local/share/arda/retained/keeper");

    let result = arda_engine::objectives::validate_snapshot_owner_paths(workspace, &[state]);
    println!("Result: {:?}", result);

    // The function should NOT reject same-mount paths that don't overlap
    assert!(
        result.is_ok(),
        "Same-mount paths should not overlap: {:?}",
        result
    );
}
