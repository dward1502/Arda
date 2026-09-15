use super::*;
use crate::objectives::tree_witness::{BackingView, MountWitness, ObjectId};
fn tree(root: &[u8], inode: u64) -> TreeWitness {
    TreeWitness {
        version: 1,
        mounts: vec![MountWitness {
            relative: vec![],
            object: ObjectId {
                device: libc::makedev(1, 2),
                inode,
                kind: libc::S_IFDIR,
            },
            backing: BackingView {
                major: 1,
                minor: 2,
                root: root.to_vec(),
            },
            options: vec![b"nosuid".to_vec(), b"rw".to_vec()],
        }],
    }
}
#[test]
fn envelope_roundtrip_binds_policy_run_and_captured_trees() {
    let policy = ValidatedRuntimePolicy::parse(&serde_json::to_vec(&serde_json::json!({
        "version":1,"entrypoint":{"interpreter":"/usr/bin/python3.12","ordered_import_roots":["/usr/lib/python3.12"]},
        "grants":[
            {"id":"system","source":"/usr","destination":"/usr","kind":"directory","access":"read_only","role":"runtime"},
            {"id":"state","source":"/var/tmp/state","destination":"/hermes/profiles/retained","kind":"directory","access":"read_write","role":"session_state"}
        ],"fixed_environment":{"HOME":"/hermes/profiles/retained","HERMES_HOME":"/hermes/profiles/retained","PATH":"/usr/bin:/bin"}
    })).unwrap()).unwrap();
    let workspace = tree(b"/workspace", 1);
    let identity = crate::objectives::store::encode_workspace_identity(
        std::path::Path::new("/workspace"),
        std::path::Path::new("/workspace"),
        Some((workspace.mounts[0].object.device, 1)),
        b"fixture",
    )
    .unwrap();
    let envelope = ExpectedCaptureEnvelope {
        version: 1,
        admission_nonce: "a".repeat(64),
        run_id: "run".into(),
        workspace_identity: identity.clone(),
        policy_digest: policy.digest().into(),
        workspace: workspace.clone(),
        grants: vec![
            ExpectedGrant {
                id: "state".into(),
                source: tree(b"/state", 2),
            },
            ExpectedGrant {
                id: "system".into(),
                source: tree(b"/usr", 3),
            },
        ],
    };
    let fd = envelope.seal(&policy, "run", &identity).unwrap();
    let mut retained: ExpectedCaptureEnvelope =
        serde_json::from_value(serde_json::to_value(&envelope).unwrap()).unwrap();
    let mut retained_tuple: serde_json::Value = serde_json::from_str(&identity).unwrap();
    retained_tuple[0] = serde_json::json!(3);
    retained_tuple[4] = serde_json::json!(workspace.digest().unwrap());
    retained.version = 2;
    retained.workspace_identity = serde_json::to_string(&retained_tuple).unwrap();
    retained
        .validate(&policy, "run", &retained.workspace_identity)
        .unwrap();
    retained.workspace.mounts[0].backing.root = b"/replacement".to_vec();
    assert!(retained
        .validate(&policy, "run", &retained.workspace_identity)
        .is_err());
    retained.workspace = workspace.clone();
    retained.version = 1;
    assert!(retained
        .validate(&policy, "run", &retained.workspace_identity)
        .is_err());
    let mut legacy: ExpectedCaptureEnvelope =
        serde_json::from_value(serde_json::to_value(&envelope).unwrap()).unwrap();
    let mut tuple: serde_json::Value = serde_json::from_str(&identity).unwrap();
    tuple[0] = serde_json::json!(1);
    legacy.workspace_identity = serde_json::to_string(&tuple).unwrap();
    assert!(legacy
        .validate(&policy, "run", &legacy.workspace_identity)
        .is_err());
    legacy.workspace_identity = identity.clone();
    legacy.run_id = "r".repeat(1024);
    let long_fd = legacy.seal(&policy, &legacy.run_id, &identity).unwrap();
    ExpectedCaptureEnvelope::read(long_fd, &policy, &legacy.run_id, &identity).unwrap();
    legacy.run_id.push('r');
    assert!(legacy.validate(&policy, &legacy.run_id, &identity).is_err());
    let decoded = ExpectedCaptureEnvelope::read(fd, &policy, "run", &identity).unwrap();
    assert_eq!(decoded.digest().unwrap(), envelope.digest().unwrap());
    let mut captures = vec![tree(b"/state", 2), tree(b"/usr", 3).readonly().unwrap()];
    decoded
        .verify_captures(&policy, &workspace, &captures)
        .unwrap();
    captures[1].mounts[0].backing.root = b"/outside".to_vec();
    assert!(decoded
        .verify_captures(&policy, &workspace, &captures)
        .is_err());
    assert!(decoded.validate(&policy, "other-run", &identity).is_err());
    let mut changed = workspace.clone();
    changed.mounts[0].object.inode = 9;
    assert!(decoded
        .verify_captures(&policy, &changed, &captures)
        .is_err());
}
#[test]
fn witness_bytes_boundaries_options_and_digest_are_strict() {
    let a = tree(b"/a", 1);
    let sibling = tree(b"/ab", 2);
    assert!(!a.overlaps(&sibling).unwrap());
    assert!(a.overlaps(&tree(b"/a/child", 3)).unwrap());
    let ro = a.readonly().unwrap();
    assert!(ro == ro.readonly().unwrap());
    let mut non_utf8 = a.clone();
    non_utf8.mounts[0].backing.root = vec![b'/', 0xff];
    non_utf8.validate().unwrap();
    assert_ne!(a.digest().unwrap(), non_utf8.digest().unwrap());
    let mut malformed = a.clone();
    malformed.mounts[0].backing.root = b"/a/.".to_vec();
    assert!(malformed.validate().is_err());
    let mut options = a.clone();
    options.mounts[0].options.push(b"rw".to_vec());
    assert!(options.validate().is_err());
    let mut other_device = a.clone();
    other_device.mounts[0].object.device = libc::makedev(1, 3);
    other_device.mounts[0].backing.minor = 3;
    assert!(!a.overlaps(&other_device).unwrap());
    assert_ne!(a.digest().unwrap(), other_device.digest().unwrap());
}
