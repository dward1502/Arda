use super::*;
use std::os::unix::fs::PermissionsExt;

#[tokio::test(flavor = "current_thread")]
async fn stalled_capture_obeys_deadline_and_cancellation_without_blocking_executor() {
    for cancel in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let pid_path = temp.path().join("pid");
        let helper = temp.path().join("stalled.py");
        std::fs::write(&helper, format!("#!/usr/bin/python3\nimport os,time\nopen({:?}, 'w').write(str(os.getpid()))\ntime.sleep(30)\n", pid_path.to_str().unwrap())).unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let cancellation = crate::adapters::AdapterCancellation::new();
        let stop = super::super::CaptureStop::new(&cancellation);
        let _guard = stop.guard();
        let start = Instant::now();
        let deadline = start + Duration::from_millis(if cancel { 3000 } else { 180 });
        let worker = tokio::task::spawn_blocking(move || capture(&helper, &[], deadline, &stop));
        tokio::time::timeout(Duration::from_secs(1), async {
            while !pid_path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        if cancel {
            cancellation.cancel();
        }
        let result = tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
        let pid: i32 = std::fs::read_to_string(&pid_path).unwrap().parse().unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
        assert_eq!(
            unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
