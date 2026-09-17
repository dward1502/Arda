use super::*;

#[test]
fn readonly_workspace_denies_mutation_but_session_storage_remains_writable() {
    for writable in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let state = temp.path().join("sessions");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&state).unwrap();
        fs::write(root.join("existing"), b"original").unwrap();
        fs::write(root.join("remove"), b"original").unwrap();
        let stage = fs::File::open(&root).unwrap();
        let fd = stage.as_raw_fd();
        let mut command = Command::new("/usr/bin/bwrap");
        command.args(["--unshare-user", "--unshare-pid", "--die-with-parent", "--cap-drop", "ALL", "--ro-bind", "/", "/"])
            .arg(workspace_mount_argument(writable)).arg(fd.to_string()).arg(&root)
            .arg("--bind").arg(&state).arg(&state)
            .arg("--chdir").arg(&root)
            .args(["/usr/bin/python3", "-c", r#"
import errno, pathlib, sys
root, state, writable = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3] == 'true'
assert (root / 'existing').read_bytes() == b'original'
for action in [lambda: (root / 'new').write_text('new'), lambda: (root / 'existing').write_text('changed'), lambda: (root / 'remove').unlink()]:
    try:
        action()
    except OSError as error:
        assert not writable and error.errno in (errno.EROFS, errno.EACCES, errno.EPERM), repr(error)
    else:
        assert writable, 'read-only workspace mutation succeeded'
(state / 'session').write_text('persisted')
"#]).arg(&root).arg(&state).arg(writable.to_string());
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(state.join("session")).unwrap(), b"persisted");
        if !writable {
            assert_eq!(fs::read(root.join("existing")).unwrap(), b"original");
            assert_eq!(fs::read(root.join("remove")).unwrap(), b"original");
            assert!(!root.join("new").exists());
        }
    }
}
