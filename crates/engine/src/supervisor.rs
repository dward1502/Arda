//! Process supervision for the Arda daemon.
//!
//! `Supervisor` owns one or more named child processes (e.g. the Tauri
//! `arda-launcher`). It spawns each child, watches for exit, and restarts it
//! with exponential backoff (capped). A shared `Shutdown` signal lets the
//! daemon stop everything cleanly (ctrl-c or `--once`).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;
use tokio::sync::{watch, Mutex, RwLock};
use tracing::{error, info, warn};

/// A process the daemon keeps alive.
#[derive(Clone)]
pub struct Service {
    pub name: &'static str,
    /// Executable to spawn. If it does not exist, the service is skipped
    /// (e.g. the HUD binary only exists after its own build step).
    pub exe: PathBuf,
    pub args: Vec<String>,
    /// Working directory for the child process.
    pub cwd: Option<PathBuf>,
    /// If true, a missing exe is a hard error rather than a skip. Defaults to
    /// false for backwards compatibility with the optional-launcher behaviour.
    pub required: bool,
    pub optional: bool,
    pub health: Option<HealthProbe>,
}

#[derive(Clone)]
pub struct HealthProbe {
    pub url: String,
    pub interval: Duration,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceLifecycle {
    Starting,
    Healthy,
    Degraded,
    Restarting,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceRuntimeStatus {
    pub name: String,
    pub required: bool,
    pub optional: bool,
    pub state: ServiceLifecycle,
    pub pid: Option<u32>,
    pub restart_count: u32,
    pub backoff_ms: Option<u64>,
    pub detail: String,
}

#[derive(Clone)]
pub struct Shutdown {
    inner: watch::Sender<bool>,
}

impl Shutdown {
    pub fn new() -> Self {
        Shutdown {
            inner: watch::channel(false).0,
        }
    }
    pub fn trigger(&self) {
        self.inner.send_replace(true);
    }
    pub fn is_triggered(&self) -> bool {
        *self.inner.borrow()
    }
    pub async fn wait(&self) {
        let mut receiver = self.inner.subscribe();
        let _ = receiver.wait_for(|stopping| *stopping).await;
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone)]
pub struct Supervisor {
    inner: Arc<Inner>,
}

struct Inner {
    services: Vec<Service>,
    /// Live child PIDs, mirrored so diagnostics (e.g. `child_pids`) can read
    /// them without disturbing the `Child` owned by `supervise_one`.
    pids: Arc<RwLock<Vec<Option<u32>>>>,
    statuses: Arc<RwLock<Vec<ServiceRuntimeStatus>>>,
    /// Optional external mirror kept in sync with `pids` so the harness status
    /// surface can read live PIDs without owning the supervisor's internals.
    pid_mirror: RwLock<Option<Arc<RwLock<Vec<u32>>>>>,
    join_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    shutdown: Shutdown,
}

use tokio::task::JoinHandle;

impl Supervisor {
    pub fn new(services: Vec<Service>, shutdown: Shutdown) -> Self {
        let n = services.len();
        let mut pids = Vec::with_capacity(n);
        for _ in 0..n {
            pids.push(None);
        }
        let statuses = services
            .iter()
            .map(|service| ServiceRuntimeStatus {
                name: service.name.to_string(),
                required: service.required,
                optional: service.optional,
                state: ServiceLifecycle::Stopped,
                pid: None,
                restart_count: 0,
                backoff_ms: None,
                detail: "not started".to_string(),
            })
            .collect();
        Supervisor {
            inner: Arc::new(Inner {
                services,
                pids: Arc::new(RwLock::new(pids)),
                statuses: Arc::new(RwLock::new(statuses)),
                pid_mirror: RwLock::new(None),
                join_handles: Arc::new(Mutex::new(Vec::new())),
                shutdown,
            }),
        }
    }

    pub fn statuses(&self) -> Arc<RwLock<Vec<ServiceRuntimeStatus>>> {
        self.inner.statuses.clone()
    }

    /// Attach an external PID mirror the supervisor keeps in sync. Passing
    /// `None` detaches it.
    pub async fn set_pid_mirror(&self, mirror: Option<Arc<RwLock<Vec<u32>>>>) {
        *self.inner.pid_mirror.write().await = mirror;
    }

    /// Push the current live PIDs into the attached mirror (if any).
    async fn sync_pid_mirror(&self) {
        let mirror = self.inner.pid_mirror.read().await.clone();
        if let Some(m) = mirror {
            let live: Vec<u32> = self
                .inner
                .pids
                .read()
                .await
                .iter()
                .filter_map(|p| *p)
                .collect();
            *m.write().await = live;
        }
    }

    pub async fn run(&self) {
        let services = self.inner.services.clone();
        {
            let mut handles = self.inner.join_handles.lock().await;
            for (i, svc) in services.iter().enumerate() {
                if !svc.exe.exists() {
                    warn!(
                        "supervisor: skipping '{}' — exe not found: {}",
                        svc.name,
                        svc.exe.display()
                    );
                    continue;
                }
                info!(
                    "supervisor: starting '{}' ({})",
                    svc.name,
                    svc.exe.display()
                );
                let pids = self.inner.pids.clone();
                let statuses = self.inner.statuses.clone();
                let shutdown = self.inner.shutdown.clone();
                let svc = Service {
                    name: svc.name,
                    exe: svc.exe.clone(),
                    args: svc.args.clone(),
                    cwd: svc.cwd.clone(),
                    required: svc.required,
                    optional: svc.optional,
                    health: svc.health.clone(),
                };
                handles.push(tokio::spawn(async move {
                    supervise_one(i, svc, pids, statuses, shutdown).await;
                }));
            }
        }

        // Keep the optional external PID mirror (harness status surface) in
        // sync while children run. Stops when shutdown fires.
        let mirror_super = self.clone();
        let mirror_shutdown = self.inner.shutdown.clone();
        let mirror_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = mirror_shutdown.wait() => break,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {
                        mirror_super.sync_pid_mirror().await;
                    }
                }
            }
        });

        // Keep each in-flight JoinHandle in the collection across select!
        // cancellation. Draining it before awaiting would detach the worker.
        tokio::select! {
            _ = self.inner.shutdown.wait() => {}
            _ = async {
                let mut handles = self.inner.join_handles.lock().await;
                while let Some(h) = handles.last_mut() {
                    let _ = h.await;
                    handles.pop();
                }
            } => {}
        }

        info!("supervisor: shutdown signal received, stopping children");
        self.inner.shutdown.trigger();
        let _ = mirror_handle.await;
        let mut handles = self.inner.join_handles.lock().await;
        while let Some(h) = handles.last_mut() {
            let _ = h.await;
            handles.pop();
        }
        self.sync_pid_mirror().await;
        for status in self.inner.statuses.write().await.iter_mut() {
            status.state = ServiceLifecycle::Stopped;
            status.pid = None;
            status.backoff_ms = None;
            status.detail = "stopped by supervisor shutdown".to_string();
        }
        info!("supervisor: all children stopped");
    }

    pub async fn wait(&self) {
        let mut handles = self.inner.join_handles.lock().await;
        while let Some(h) = handles.last_mut() {
            let _ = h.await;
            handles.pop();
        }
    }

    pub async fn child_pids(&self) -> Vec<u32> {
        let g = self.inner.pids.read().await;
        g.iter().filter_map(|p| *p).collect()
    }

    pub async fn shutdown_and_wait(&self) {
        self.inner.shutdown.trigger();
        self.wait().await;
    }
}

async fn supervise_one(
    idx: usize,
    svc: Service,
    pids: Arc<RwLock<Vec<Option<u32>>>>,
    statuses: Arc<RwLock<Vec<ServiceRuntimeStatus>>>,
    shutdown: Shutdown,
) {
    let mut backoff = Duration::from_millis(250);
    const MAX_BACKOFF: Duration = Duration::from_secs(10);

    loop {
        if *shutdown.inner.borrow() {
            return;
        }
        {
            let mut runtime = statuses.write().await;
            runtime[idx].state = ServiceLifecycle::Starting;
            runtime[idx].pid = None;
            runtime[idx].backoff_ms = None;
            runtime[idx].detail = "spawning child process".to_string();
        }
        let mut cmd = Command::new(&svc.exe);
        for a in &svc.args {
            cmd.arg(a);
        }
        if let Some(cwd) = &svc.cwd {
            cmd.current_dir(cwd);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                error!("supervisor: failed to spawn '{}': {e}", svc.name);
                {
                    let mut runtime = statuses.write().await;
                    runtime[idx].state = ServiceLifecycle::Restarting;
                    runtime[idx].restart_count += 1;
                    runtime[idx].backoff_ms = Some(backoff.as_millis() as u64);
                    runtime[idx].detail = format!("spawn failed: {e}");
                }
                tokio::select! {
                    _ = shutdown.wait() => {
                        info!("supervisor: '{}' not retrying (shutdown)", svc.name);
                        return;
                    }
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };

        let pid = child.id();
        info!(
            "supervisor: '{}' spawned (pid {})",
            svc.name,
            pid.unwrap_or(0)
        );
        {
            let mut pg = pids.write().await;
            pg[idx] = pid;
        }
        {
            let mut runtime = statuses.write().await;
            runtime[idx].state = if svc.health.is_some() {
                ServiceLifecycle::Starting
            } else {
                ServiceLifecycle::Healthy
            };
            runtime[idx].pid = pid;
            runtime[idx].backoff_ms = None;
            runtime[idx].detail = if svc.health.is_some() {
                "child spawned; waiting for readiness probe".to_string()
            } else {
                "child process is running; no readiness probe declared".to_string()
            };
        }
        // Each worker owns its child. The health future is scoped to this
        // generation and is dropped before shutdown/restart publishes status.
        let (status, stopping) = tokio::select! {
            biased;
            _ = shutdown.wait() => {
                let _ = child.start_kill();
                (child.wait().await.ok(), true)
            }
            status = child.wait() => (status.ok(), false),
            _ = probe_health(idx, svc.health.as_ref(), &statuses) => unreachable!("health probe runs until cancelled"),
        };
        {
            let mut pg = pids.write().await;
            pg[idx] = None;
        }
        if stopping {
            let mut runtime = statuses.write().await;
            runtime[idx].state = ServiceLifecycle::Stopped;
            runtime[idx].pid = None;
            runtime[idx].backoff_ms = None;
            runtime[idx].detail = "stopped by supervisor shutdown".to_string();
            info!("supervisor: '{}' stopped on shutdown", svc.name);
            return;
        }
        match status {
            Some(code) => warn!("supervisor: '{}' exited (code {:?})", svc.name, code),
            None => warn!("supervisor: '{}' exited (no status)", svc.name),
        }
        {
            let mut runtime = statuses.write().await;
            runtime[idx].state = ServiceLifecycle::Restarting;
            runtime[idx].pid = None;
            runtime[idx].restart_count += 1;
            runtime[idx].backoff_ms = Some(backoff.as_millis() as u64);
            runtime[idx].detail = "child exited; bounded restart scheduled".to_string();
        }

        tokio::select! {
            _ = shutdown.wait() => {
                info!("supervisor: '{}' not restarting (shutdown)", svc.name);
                return;
            }
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
        info!(
            "supervisor: restarting '{}' (backoff {:?})",
            svc.name, backoff
        );
    }
}

/// Polled within the owning child's select, never detached or shared across
/// generations. Child exit/shutdown drops an in-flight request and interval.
async fn probe_health(
    idx: usize,
    health: Option<&HealthProbe>,
    statuses: &RwLock<Vec<ServiceRuntimeStatus>>,
) {
    let Some(health) = health else {
        return std::future::pending().await;
    };
    let client = reqwest::Client::new();
    loop {
        let result = client.get(&health.url).timeout(health.timeout).send().await;
        let mut runtime = statuses.write().await;
        let status = &mut runtime[idx];
        match result {
            Ok(response) if response.status().is_success() => {
                status.state = ServiceLifecycle::Healthy;
                status.detail = format!("readiness probe passed: {}", health.url);
            }
            Ok(response) => {
                status.state = ServiceLifecycle::Degraded;
                status.detail = format!(
                    "readiness probe returned {}: {}",
                    response.status(),
                    health.url
                );
            }
            Err(error) => {
                status.state = ServiceLifecycle::Degraded;
                status.detail = format!("readiness probe failed: {error}");
            }
        }
        drop(runtime);
        tokio::time::sleep(health.interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sleeper(name: &'static str) -> Service {
        Service {
            name,
            exe: PathBuf::from("/usr/bin/sleep"),
            args: vec!["30".into()],
            cwd: None,
            required: true,
            optional: false,
            health: None,
        }
    }

    #[tokio::test]
    async fn sibling_children_start_without_waiting_for_another_child_to_exit() {
        let shutdown = Shutdown::new();
        let supervisor =
            Supervisor::new(vec![sleeper("first"), sleeper("second")], shutdown.clone());
        let runner = supervisor.clone();
        let task = tokio::spawn(async move { runner.run().await });
        let observed = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let pids = supervisor.child_pids().await;
                if pids.len() == 2 {
                    return pids;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert!(observed.is_ok(), "one child's wait blocked sibling startup");
        assert!(supervisor.child_pids().await.is_empty());
        for pid in observed.unwrap() {
            assert!(
                !std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "child {pid} not reaped"
            );
        }
    }

    #[tokio::test]
    async fn shutdown_cancels_pending_health_request_before_returning() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let started = Arc::new(tokio::sync::Notify::new());
        let accepted = started.clone();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                header.push(stream.read_u8().await.unwrap());
            }
            accepted.notify_one();
            let mut byte = [0];
            // The probe must drop its request on shutdown, not wait for the
            // configured minute-long health timeout or publish a stale response.
            stream.read(&mut byte).await.unwrap()
        });
        let mut service = sleeper("pending-health");
        service.health = Some(HealthProbe {
            url: format!("http://{address}/health"),
            interval: Duration::from_secs(60),
            timeout: Duration::from_secs(60),
        });
        let shutdown = Shutdown::new();
        let supervisor = Supervisor::new(vec![service], shutdown.clone());
        let runner = supervisor.clone();
        let task = tokio::spawn(async move { runner.run().await });
        tokio::time::timeout(Duration::from_secs(2), started.notified())
            .await
            .unwrap();
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        let mut server = server;
        let closed = tokio::time::timeout(Duration::from_millis(300), &mut server).await;
        if closed.is_err() {
            server.abort();
            let _ = server.await;
        }
        assert_eq!(
            closed.expect("health request outlived supervisor").unwrap(),
            0
        );
        assert_eq!(
            supervisor.statuses().read().await[0].state,
            ServiceLifecycle::Stopped
        );
    }

    #[tokio::test]
    async fn shutdown_is_retained_for_late_and_repeated_waiters() {
        let shutdown = Shutdown::new();
        shutdown.trigger();
        for _ in 0..3 {
            tokio::time::timeout(Duration::from_millis(100), shutdown.clone().wait())
                .await
                .expect("shutdown must not be lost before a waiter subscribes");
        }
    }

    #[tokio::test]
    async fn shutdown_before_run_does_not_spawn_a_child() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("spawned");
        let shutdown = Shutdown::new();
        let supervisor = Supervisor::new(
            vec![Service {
                name: "must-not-spawn",
                exe: PathBuf::from("/bin/sh"),
                args: vec![
                    "-c".into(),
                    "touch \"$1\"".into(),
                    "fixture".into(),
                    marker.to_string_lossy().into_owned(),
                ],
                cwd: None,
                required: true,
                optional: false,
                health: None,
            }],
            shutdown.clone(),
        );
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(2), supervisor.run())
            .await
            .unwrap();
        assert!(!marker.exists());
        assert!(supervisor.child_pids().await.is_empty());
    }

    #[tokio::test]
    async fn shutdown_keeps_worker_handles_until_they_are_joined() {
        let shutdown = Shutdown::new();
        let supervisor = Supervisor::new(Vec::new(), shutdown.clone());
        let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_finished = finished.clone();
        let worker_shutdown = shutdown.clone();
        supervisor
            .inner
            .join_handles
            .lock()
            .await
            .push(tokio::spawn(async move {
                worker_shutdown.wait().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
                worker_finished.store(true, std::sync::atomic::Ordering::SeqCst);
            }));
        let runner = supervisor.clone();
        let task = tokio::spawn(async move { runner.run().await });
        tokio::time::timeout(Duration::from_secs(2), async {
            while supervisor.inner.join_handles.try_lock().is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert!(
            finished.load(std::sync::atomic::Ordering::SeqCst),
            "shutdown detached a worker instead of joining it"
        );
    }

    /// A supervised `sleep` child must be reaped when the supervisor receives
    /// the shutdown signal. Tracks the exact PID (not a global pgrep scan) so
    /// the test is hermetic.
    #[tokio::test]
    async fn supervises_and_reaps_child_on_shutdown() {
        let svc = Service {
            name: "test-sleeper",
            exe: PathBuf::from("/usr/bin/sleep"),
            // Plain numeric duration (no shell arithmetic — we exec sleep directly).
            args: vec!["5".into()],
            cwd: None,
            required: false,
            optional: true,
            health: None,
        };
        let exists = svc.exe.exists();
        let shutdown = Shutdown::new();
        let sup = Supervisor::new(vec![svc], shutdown.clone());

        let sup_task = sup.clone();
        let sup_handle = tokio::spawn(async move { sup_task.run().await });

        // Poll for the supervised pid (supervisor spawn may trail test startup).
        let mut pids = Vec::new();
        for _ in 0..40 {
            pids = sup.child_pids().await;
            if !pids.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            !pids.is_empty(),
            "expected a supervised child pid (exe exists: {exists}), got none"
        );

        shutdown.trigger();
        let _ = tokio::time::timeout(Duration::from_secs(5), sup_handle).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        for pid in pids {
            // signal 0 (kill -0) reports liveness without sending a signal.
            let alive = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(!alive, "child pid {pid} was not reaped on shutdown");
        }
    }

    #[tokio::test]
    async fn restart_state_reports_attempt_count_and_bounded_backoff() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("started-once");
        let script = format!(
            "if [ ! -e '{}' ]; then : > '{}'; exit 7; fi; sleep 5",
            marker.display(),
            marker.display()
        );
        let svc = Service {
            name: "restart-once",
            exe: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script],
            cwd: None,
            required: true,
            optional: false,
            health: None,
        };
        let shutdown = Shutdown::new();
        let supervisor = Supervisor::new(vec![svc], shutdown.clone());
        let statuses = supervisor.statuses();
        let task = tokio::spawn({
            let supervisor = supervisor.clone();
            async move { supervisor.run().await }
        });

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = statuses.read().await[0].clone();
                if status.restart_count == 1 && status.state == ServiceLifecycle::Healthy {
                    assert_eq!(status.backoff_ms, None);
                    assert!(status.pid.is_some());
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("service restarted within bounded backoff");

        shutdown.trigger();
        task.await.expect("supervisor join");
        let status = statuses.read().await[0].clone();
        assert_eq!(status.state, ServiceLifecycle::Stopped);
        assert_eq!(status.restart_count, 1);
        assert_eq!(status.pid, None);
    }

    #[tokio::test]
    async fn killed_child_is_reaped_and_restarted_with_visible_attribution() {
        let svc = Service {
            name: "kill-recovery",
            exe: PathBuf::from("/usr/bin/sleep"),
            args: vec!["30".into()],
            cwd: None,
            required: true,
            optional: false,
            health: None,
        };
        let shutdown = Shutdown::new();
        let supervisor = Supervisor::new(vec![svc], shutdown.clone());
        let statuses = supervisor.statuses();
        let task = tokio::spawn({
            let supervisor = supervisor.clone();
            async move { supervisor.run().await }
        });

        let original_pid = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(pid) = statuses.read().await[0].pid {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("initial supervised process");
        assert!(std::process::Command::new("/bin/kill")
            .args(["-KILL", &original_pid.to_string()])
            .status()
            .expect("kill child")
            .success());

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = statuses.read().await[0].clone();
                if status.restart_count >= 1 && status.pid.is_some_and(|pid| pid != original_pid) {
                    assert_eq!(status.state, ServiceLifecycle::Healthy);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("killed process restarted with attribution");

        shutdown.trigger();
        task.await.expect("supervisor join");
    }
}
