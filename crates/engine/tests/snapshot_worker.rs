#[test]
#[ignore = "requires Linux user/mount namespaces and bubblewrap"]
fn retained_worker_reexec_survives_executable_replacement() {
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--propagation",
            "private",
            "/usr/bin/python3",
        ])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snapshot_executable_retention.py"
        ))
        .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
        .output()
        .expect("launch executable retention fixture");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires Linux user/mount namespaces and bubblewrap"]
fn retained_snapshot_protocol_survives_clients_and_host_mount_changes() {
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--propagation",
            "private",
            "/usr/bin/python3",
        ])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snapshot_worker.py"
        ))
        .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
        .output()
        .expect("launch snapshot namespace fixture");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
