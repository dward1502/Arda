use super::*;
#[test]
fn teardown_requires_historical_identity_and_no_descendants() {
    let binding = Binding {
        owner: "owner".into(),
        uid: 1,
        machine: "machine".into(),
        boot: "boot".into(),
        unit: "test.service".into(),
        invocation: "original".into(),
        cgroup: "/user.slice/test".into(),
        cgroup_device: 7,
        cgroup_inode: 8,
        runtime: "/runtime".into(),
    };
    let properties: BTreeMap<String, String> = [
        ("LoadState", "masked"),
        ("UnitFileState", "masked-runtime"),
        ("ActiveState", "inactive"),
        ("MainPID", "0"),
        ("InvocationID", "original"),
        ("ControlGroup", ""),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect();
    let check = |p: &BTreeMap<String, String>, observation| {
        verify_stopped(&binding, p, "boot", |_| Ok(observation))
    };
    assert!(check(&properties, None).is_ok());
    assert!(check(&properties, Some((7, 8, "populated 0\n".into()))).is_ok());
    for observation in [
        Some((7, 8, "populated 1\n".into())),
        Some((7, 9, "populated 0\n".into())),
        Some((9, 8, "populated 0\n".into())),
        Some((7, 8, "frozen 0\n".into())),
    ] {
        assert!(check(&properties, observation).is_err());
    }
    let mut empty = properties.clone();
    empty.insert("InvocationID".into(), "".into());
    assert!(check(&empty, None).is_ok());
    assert!(check(&empty, Some((7, 8, "populated 0\n".into()))).is_err());
    assert!(check(&empty, Some((7, 8, "populated 1\n".into()))).is_err());
    for (key, value) in [
        ("InvocationID", "replacement"),
        ("ControlGroup", "/still-present"),
        ("ActiveState", "deactivating"),
        ("ActiveState", "activating"),
        ("MainPID", "42"),
        ("LoadState", "loaded"),
        ("UnitFileState", "enabled"),
    ] {
        let mut changed = properties.clone();
        changed.insert(key.into(), value.into());
        assert!(check(&changed, None).is_err(), "accepted {key}={value}");
    }
    // A reboot proves teardown, not a worker ACK, and still requires masking.
    assert!(
        verify_stopped(&binding, &properties, "new-boot", |_| panic!(
            "old cgroup irrelevant after reboot"
        ))
        .is_ok()
    );
    let mut active = properties;
    active.insert("ActiveState".into(), "active".into());
    assert!(verify_stopped(&binding, &active, "new-boot", |_| Ok(None)).is_err());
    let db = Connection::open_in_memory().unwrap();
    assert!(load(&db, "missing-history").is_err());
}
