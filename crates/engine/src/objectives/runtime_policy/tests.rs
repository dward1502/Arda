use super::*;
#[test]
fn manifests_preserve_legacy_bytes_and_reject_runtime_downgrades() {
    use crate::objectives::snapshot_protocol::{CapturedRuntimeGrant, Manifest, RuntimeBundle};
    let legacy = br#"{"version":1,"capability":"fixture","root":"/workspace","device":1,"inode":2,"topology_digest":"legacy"}"#;
    let mut manifest: Manifest = serde_json::from_slice(legacy).unwrap();
    assert_eq!(serde_json::to_vec(&manifest).unwrap(), legacy);
    assert!(manifest.validate_runtime(None).is_ok());
    let policy = ValidatedRuntimePolicy::validate(fixture()).unwrap();
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
    manifest.version = 2;
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
    manifest.runtime_bundle = Some(RuntimeBundle {
        version: 1,
        policy_digest: policy.digest().into(),
        grants: policy
            .policy()
            .grants
            .iter()
            .map(|g| CapturedRuntimeGrant {
                id: g.id.clone(),
                destination: g.destination.clone(),
                kind: g.kind.clone(),
                access: g.access.clone(),
                device: 1,
                inode: 2,
                topology_digest: "a".repeat(64),
            })
            .collect(),
    });
    assert!(manifest.validate_runtime(Some(&policy)).is_ok());
    manifest.runtime_bundle.as_mut().unwrap().policy_digest = "b".repeat(64);
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
    manifest.runtime_bundle.as_mut().unwrap().policy_digest = policy.digest().into();
    let destination = manifest.runtime_bundle.as_ref().unwrap().grants[0]
        .destination
        .clone();
    manifest.runtime_bundle.as_mut().unwrap().grants[0].destination = destination.join(".");
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
    manifest.runtime_bundle.as_mut().unwrap().grants[0].destination = destination;
    assert!(manifest.validate_runtime(None).is_err());
    manifest.version = 1;
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
    manifest.version = 2;
    manifest.runtime_bundle.as_mut().unwrap().grants[0].destination = "/substituted".into();
    assert!(manifest.validate_runtime(Some(&policy)).is_err());
}

#[test]
fn canonical_spelling_is_required_for_paths_and_profile_selectors() {
    for suffix in ["/", "/."] {
        let mut policy = fixture();
        policy.grants[1].source = format!("/opt/installed-hermes{suffix}").into();
        assert!(ValidatedRuntimePolicy::validate(policy).is_err());
        for key in ["HOME", "HERMES_HOME"] {
            let mut policy = fixture();
            policy
                .fixed_environment
                .insert(key.into(), format!("/hermes/profiles/retained{suffix}"));
            assert!(ValidatedRuntimePolicy::validate(policy).is_err());
        }
    }
}

pub(super) fn fixture() -> RuntimePolicy {
    RuntimePolicy {
        version: 1,
        entrypoint: PythonEntrypoint {
            interpreter: "/usr/bin/python3.12".into(),
            ordered_import_roots: vec![
                "/opt/installed-hermes".into(),
                "/opt/python-packages".into(),
            ],
        },
        grants: vec![
            RuntimeGrant {
                id: "usr".into(),
                source: "/usr".into(),
                destination: "/usr".into(),
                kind: GrantKind::Directory,
                access: GrantAccess::ReadOnly,
                role: GrantRole::Runtime,
            },
            RuntimeGrant {
                id: "code".into(),
                source: "/opt/installed-hermes".into(),
                destination: "/opt/installed-hermes".into(),
                kind: GrantKind::Directory,
                access: GrantAccess::ReadOnly,
                role: GrantRole::Runtime,
            },
            RuntimeGrant {
                id: "packages".into(),
                source: "/opt/python-packages".into(),
                destination: "/opt/python-packages".into(),
                kind: GrantKind::Directory,
                access: GrantAccess::ReadOnly,
                role: GrantRole::Runtime,
            },
            RuntimeGrant {
                id: "state".into(),
                source: "/tmp/private-hermes-state".into(),
                destination: "/hermes/profiles/retained".into(),
                kind: GrantKind::Directory,
                access: GrantAccess::ReadWrite,
                role: GrantRole::SessionState,
            },
            RuntimeGrant {
                id: "profile".into(),
                source: "/opt/private-profile/config.yaml".into(),
                destination: "/hermes/profiles/retained/config.yaml".into(),
                kind: GrantKind::File,
                access: GrantAccess::ReadOnly,
                role: GrantRole::ProfileInput,
            },
        ],
        fixed_environment: BTreeMap::from([
            ("HOME".into(), "/hermes/profiles/retained".into()),
            ("HERMES_HOME".into(), "/hermes/profiles/retained".into()),
            ("PATH".into(), "/usr/bin:/bin".into()),
        ]),
    }
}
#[test]
fn canonical_digest_sorts_grants_but_preserves_import_order() {
    let original = ValidatedRuntimePolicy::validate(fixture()).unwrap();
    let mut changed = fixture();
    changed.grants.reverse();
    assert_eq!(
        original.digest(),
        ValidatedRuntimePolicy::validate(changed).unwrap().digest()
    );
    let mut changed = fixture();
    changed.entrypoint.ordered_import_roots.reverse();
    assert_ne!(
        original.digest(),
        ValidatedRuntimePolicy::validate(changed).unwrap().digest()
    );
    let reparsed = ValidatedRuntimePolicy::parse(&original.bytes().unwrap()).unwrap();
    assert_eq!(original.digest(), reparsed.digest());
}
#[test]
fn rejects_untrusted_authority_shapes() {
    let mutations: &[fn(&mut RuntimePolicy)] = &[
        |p| p.version = 2,
        |p| p.grants[0].access = GrantAccess::ReadWrite,
        |p| p.grants[1].source = "/home".into(),
        |p| p.grants[1].source = "/var/home/operator/.hermes".into(),
        |p| p.grants[1].destination = "/proc/secret".into(),
        |p| p.grants[1].destination = "/usr/overlap".into(),
        |p| p.grants[1].id = "usr".into(),
        |p| p.grants[1].source = "relative".into(),
        |p| p.entrypoint.ordered_import_roots.push("/workspace".into()),
        |p| p.entrypoint.interpreter = "/workspace/python".into(),
        |p| {
            p.fixed_environment
                .insert("PYTHONPATH".into(), "/workspace".into());
        },
        |p| {
            p.fixed_environment
                .insert("API_KEY".into(), "not-a-secret-fixture".into());
        },
        |p| {
            p.fixed_environment
                .insert("HERMES_HOME".into(), "/home/operator".into());
        },
        |p| {
            p.fixed_environment
                .insert("PATH".into(), "/workspace:/usr/bin".into());
        },
    ];
    for (index, mutation) in mutations.iter().enumerate() {
        let mut policy = fixture();
        mutation(&mut policy);
        assert!(
            ValidatedRuntimePolicy::validate(policy).is_err(),
            "mutation {index}"
        );
    }
}
#[test]
fn rejects_unknown_fields_at_nested_and_top_level() {
    for nested in [false, true] {
        let mut value = serde_json::to_value(fixture()).unwrap();
        if nested {
            value["entrypoint"]["function"] = "arbitrary".into();
        } else {
            value["unapproved_grants"] = serde_json::json!([]);
        }
        assert!(ValidatedRuntimePolicy::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    assert!(ValidatedRuntimePolicy::parse(&vec![b' '; 65_537]).is_err());
}
