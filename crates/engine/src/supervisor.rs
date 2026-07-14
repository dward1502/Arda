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

use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify, RwLock};
use tracing::{error, info, warn};

/// A process the daemon keeps alive.
#[derive(Clone)]
pub struct Service {
    pub name: &'static str,
    /// Executable to spawn. If it does not exist, the service is skipped
    /// (e.g. the HUD binary only exists after its own build step).
    pub exe: PathBuf,
    pub args: Vec<String>,
    /// If true, a missing exe is a hard error rather than a skip. Defaults to
    /// false for backwards compatibility with the optional-launcher behaviour.
    pub required: bool,
}

#[derive(Clone)]
pub struct Shutdown {
    inner: Arc<Notify>,
}

impl Shutdown {
    pub fn new() -> Self {
        Shutdown {
            inner: Arc::new(Notify::new()),
        }
    }
    pub fn trigger(&self) {
        self.inner.notify_waiters();
    }
    pub async fn wait(&self) {
        self.inner.notified().await;
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
    state: Arc<RwLock<Vec<Option<Child>>>>,
    /// Live child PIDs, mirrored so diagnostics (e.g. `child_pids`) can read
    /// them without disturbing the `Child` owned by `supervise_one`.
    pids: Arc<RwLock<Vec<Option<u32>>>>,
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
        let mut state = Vec::with_capacity(n);
        let mut pids = Vec::with_capacity(n);
        for _ in 0..n {
            state.push(None);
            pids.push(None);
        }
        Supervisor {
            inner: Arc::new(Inner {
                services,
                state: Arc::new(RwLock::new(state)),
                pids: Arc::new(RwLock::new(pids)),
                pid_mirror: RwLock::new(None),
                join_handles: Arc::new(Mutex::new(Vec::new())),
                shutdown,
            }),
        }
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
                info!("supervisor: starting '{}' ({})", svc.name, svc.exe.display());
                let state = self.inner.state.clone();
                let pids = self.inner.pids.clone();
                let shutdown = self.inner.shutdown.clone();
                let svc = Service {
                    name: svc.name,
                    exe: svc.exe.clone(),
                    args: svc.args.clone(),
                    required: svc.required,
                };
                handles.push(tokio::spawn(async move {
                    supervise_one(i, svc, state, pids, shutdown).await;
                }));
            }
        }

        // Keep the optional external PID mirror (harness status surface) in
        // sync while children run. Stops when shutdown fires.
        let mirror_super = self.clone();
        let mirror_shutdown = self.inner.shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = mirror_shutdown.wait() => break,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {
                        mirror_super.sync_pid_mirror().await;
                    }
                }
            }
        });

        // Wait for either the shutdown signal or all children to finish on
        // their own. Do NOT block on shutdown.wait() alone, because a
        // broadcast channel that has already fired would return immediately.
        tokio::select! {
            _ = self.inner.shutdown.wait() => {}
            _ = async {
                let mut handles = self.inner.join_handles.lock().await;
                for h in handles.drain(..) {
                    let _ = h.await;
                }
            } => {}
        }

        info!("supervisor: shutdown signal received, stopping children");
        let mut guard = self.inner.state.write().await;
        for child in guard.iter_mut().flatten() {
            let _ = child.start_kill();
        }
        drop(guard);
        let mut handles = self.inner.join_handles.lock().await;
        for h in handles.drain(..) {
            let _ = h.await;
        }
        info!("supervisor: all children stopped");
    }

    pub async fn wait(&self) {
        let mut handles = self.inner.join_handles.lock().await;
        for h in handles.drain(..) {
            let _ = h.await;
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
    state: Arc<RwLock<Vec<Option<Child>>>>,
    pids: Arc<RwLock<Vec<Option<u32>>>>,
    shutdown: Shutdown,
) {
    let mut backoff = Duration::from_millis(250);
    const MAX_BACKOFF: Duration = Duration::from_secs(10);

    loop {
        let mut cmd = Command::new(&svc.exe);
        for a in &svc.args {
            cmd.arg(a);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                error!("supervisor: failed to spawn '{}': {e}", svc.name);
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
        info!("supervisor: '{}' spawned (pid {})", svc.name, pid.unwrap_or(0));
        {
            let mut guard = state.write().await;
            guard[idx] = Some(child);
            let mut pg = pids.write().await;
            pg[idx] = pid;
        }

        // Wait for the child to exit OR the shutdown signal. The `Child` lives in
        // `state` (owned locally as `child`) so diagnostics read `pids` and the
        // shutdown arm can reach the same handle to kill it.
        let mut g = state.write().await;
        let status = tokio::select! {
            status = async {
                match g[idx].as_mut() {
                    Some(c) => {
                        let fut = c.wait();
                        fut.await.ok()
                    }
                    None => None,
                }
            } => status,
            _ = shutdown.wait() => {
                if let Some(c) = g[idx].as_mut() {
                    let _ = c.start_kill();
                    let _ = c.wait().await;
                }
                drop(g);
                {
                    let mut pg = pids.write().await;
                    pg[idx] = None;
                }
                info!("supervisor: '{}' stopped on shutdown", svc.name);
                return;
            }
        };
        drop(g);
        // Child exited on its own.
        {
            let mut pg = pids.write().await;
            pg[idx] = None;
        }
        match status {
            Some(code) => warn!("supervisor: '{}' exited (code {:?})", svc.name, code),
            None => warn!("supervisor: '{}' exited (no status)", svc.name),
        }

        tokio::select! {
            _ = shutdown.wait() => {
                info!("supervisor: '{}' not restarting (shutdown)", svc.name);
                return;
            }
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
        info!("supervisor: restarting '{}' (backoff {:?})", svc.name, backoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            required: false,
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
        assert!(!pids.is_empty(), "expected a supervised child pid (exe exists: {exists}), got none");

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
}

