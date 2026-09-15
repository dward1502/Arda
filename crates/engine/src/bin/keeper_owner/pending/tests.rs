use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;

#[test]
fn pending_pins_reject_protected_sources_and_coordinate_replacement() {
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let durable = temp.path().join("durable");
    let runtime = temp.path().join("control");
    let base = temp.path().join("sessions");
    let session = base.join("run");
    let root = temp.path().join("workspace");
    for path in [&durable, &runtime, &session, &root] {
        fs::create_dir_all(path).unwrap();
    }
    let reserved = Reservations::pin(&durable, &runtime, &base).unwrap();
    let metadata = root.metadata().unwrap();
    let identity = serde_json::to_string(&(
        2,
        &root,
        &root,
        Some((metadata.dev(), metadata.ino())),
        format!(
            "{:x}",
            Sha256::digest(fs::read("/proc/thread-self/mountinfo").unwrap())
        ),
    ))
    .unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({
        "version":1,"entrypoint":{"interpreter":"/usr/bin/python3.12","ordered_import_roots":["/usr/lib/python3.12"]},
        "grants":[
            {"id":"system","source":"/usr","destination":"/usr","kind":"directory","access":"read_only","role":"runtime"},
            {"id":"state","source":session,"destination":"/hermes/profiles/retained","kind":"directory","access":"read_write","role":"session_state"}
        ],"fixed_environment":{"HOME":"/hermes/profiles/retained","HERMES_HOME":"/hermes/profiles/retained","PATH":"/usr/bin:/bin"}
    })).unwrap();
    let policy = ValidatedRuntimePolicy::parse(&bytes).unwrap();
    let pending =
        PendingAdmission::pin(reserved.clone(), &root, &identity, "run", &policy).unwrap();
    pending.revalidate().unwrap();
    pending.envelope.seal(&policy, "run", &identity).unwrap();
    let mut bad = policy.policy().clone();
    bad.grants
        .push(arda_engine::objectives::runtime_policy::RuntimeGrant {
            id: "leak".into(),
            source: durable.clone(),
            destination: "/runtime-leak".into(),
            kind: arda_engine::objectives::runtime_policy::GrantKind::Directory,
            access: GrantAccess::ReadOnly,
            role: GrantRole::Runtime,
        });
    let bad = ValidatedRuntimePolicy::validate(bad).unwrap();
    assert!(PendingAdmission::pin(reserved.clone(), &root, &identity, "bad", &bad).is_err());
    fs::rename(&durable, temp.path().join("old-durable")).unwrap();
    fs::create_dir(&durable).unwrap();
    assert!(reserved.revalidate().is_err());
    assert!(pending.revalidate().is_err());
}
