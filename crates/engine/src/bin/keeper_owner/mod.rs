use super::*;

pub struct Owner {
    pub db: Connection,
    pub durable: PathBuf,
    pub runtime: PathBuf,
    pub worker: PathBuf,
    pub children: BTreeMap<String, Child>,
}
#[derive(Deserialize)]
struct Inspection {
    ok: bool,
    manifest: Manifest,
    manifest_digest: String,
}
type SavedAdmission = (String, String, String, Option<String>);
impl Owner {
    fn saved(&self, run: &str) -> Result<Option<SavedAdmission>> {
        Ok(self
            .db
            .query_row(
                "SELECT workspace,identity,state,authority FROM snapshots WHERE run=?1",
                [run],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?)
    }
    fn check_saved(&self, run: &str, snapshot: &RetainedSnapshot) -> Result<String> {
        let (_, _, state, saved) = self.saved(run)?.context("unknown retained admission")?;
        let saved: RetainedSnapshot =
            serde_json::from_str(&saved.context("incomplete retained admission")?)?;
        if &saved != snapshot {
            bail!("retained authority mismatch");
        }
        Ok(state)
    }
    fn live(&mut self, run: &str) -> Result<()> {
        let child = self
            .children
            .get_mut(run)
            .context("keeper lost; explicit reconciliation required")?;
        if child.try_wait()?.is_some() {
            bail!("snapshot worker lost; explicit reconciliation required");
        }
        Ok(())
    }
    pub fn handle(&mut self, request: KeeperRequest) -> Result<KeeperResponse> {
        let snapshot = match request {
            KeeperRequest::Prepare {
                run,
                workspace,
                identity,
            } => {
                if run.is_empty() || run.len() > 1024 {
                    bail!("invalid admission identifier");
                }
                if let Some((root, prior, state, saved)) = self.saved(&run)? {
                    if root != workspace.to_str().context("UTF-8 workspace required")?
                        || prior != identity
                        || state != "prepared"
                    {
                        bail!("admission requires reconciliation");
                    }
                    self.live(&run)?;
                    Some(serde_json::from_str(
                        &saved.context("missing retained authority")?,
                    )?)
                } else {
                    arda_engine::objectives::validate_snapshot_owner_paths(
                        Path::new("/usr"),
                        &[&self.durable, &self.runtime],
                    )?;
                    arda_engine::objectives::validate_snapshot_owner_paths(
                        &workspace,
                        &[&self.durable, &self.runtime],
                    )?;
                    // FULL synchronous commit before the first process side effect.
                    self.db.execute("INSERT INTO snapshots(run,workspace,identity,state) VALUES(?1,?2,?3,'preparing')", params![run,workspace.to_str().context("UTF-8 workspace required")?,identity])?;
                    let state = self.runtime.join(uuid::Uuid::new_v4().to_string());
                    let endpoint = state.join("control.sock");
                    let parent = unsafe { libc::getpid() };
                    let mut command = Command::new(&self.worker);
                    command
                        .env_clear()
                        .arg(&state)
                        .arg(&workspace)
                        .arg(&identity)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    // Async-signal-safe parent-death arm, closing the spawn race.
                    unsafe {
                        command.pre_exec(move || {
                            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                                return Err(std::io::Error::last_os_error());
                            }
                            if libc::getppid() != parent {
                                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                            }
                            Ok(())
                        });
                    }
                    self.children.insert(run.clone(), command.spawn()?);
                    let deadline = Instant::now() + Duration::from_secs(4);
                    let inspection: Inspection = loop {
                        self.live(&run)?;
                        if endpoint.exists() {
                            break exchange(
                                &endpoint,
                                &Request::Inspect,
                                deadline
                                    .checked_duration_since(Instant::now())
                                    .context("snapshot preparation timeout")?,
                            )?;
                        }
                        if Instant::now() >= deadline {
                            bail!("snapshot preparation timeout; reconciliation required");
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    };
                    if !inspection.ok
                        || inspection.manifest.version != 1
                        || inspection.manifest.root != workspace
                    {
                        bail!("snapshot manifest rejected");
                    }
                    use sha2::{Digest, Sha256};
                    if format!(
                        "{:x}",
                        Sha256::digest(serde_json::to_vec(&inspection.manifest)?)
                    ) != inspection.manifest_digest
                    {
                        bail!("snapshot manifest digest rejected");
                    }
                    let snapshot = RetainedSnapshot {
                        endpoint: endpoint.to_str().context("UTF-8 endpoint required")?.into(),
                        capability: inspection.manifest.capability,
                        manifest_digest: inspection.manifest_digest,
                    };
                    self.db.execute(
                        "UPDATE snapshots SET state='prepared',authority=?2 WHERE run=?1",
                        params![run, serde_json::to_string(&snapshot)?],
                    )?;
                    Some(snapshot)
                }
            }
            KeeperRequest::Commit { snapshot, lease } => {
                if self.check_saved(&lease.run_id, &snapshot)? != "prepared" {
                    bail!("snapshot is not reusable");
                }
                self.live(&lease.run_id)?;
                let response: serde_json::Value = exchange(
                    Path::new(&snapshot.endpoint),
                    &Request::Commit {
                        capability: snapshot.capability.clone(),
                        manifest_digest: snapshot.manifest_digest.clone(),
                        lease,
                    },
                    Duration::from_secs(5),
                )?;
                if response.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                    bail!("worker commit refused");
                }
                None
            }
            KeeperRequest::Release { snapshot, run } => {
                let state = self.check_saved(&run, &snapshot)?;
                if state != "released" {
                    if state != "prepared" {
                        bail!("release requires explicit reconciliation");
                    }
                    self.live(&run)?;
                    self.db.execute(
                        "UPDATE snapshots SET state='releasing' WHERE run=?1",
                        [&run],
                    )?;
                    let response: serde_json::Value = exchange(
                        Path::new(&snapshot.endpoint),
                        &Request::Release {
                            capability: snapshot.capability.clone(),
                        },
                        Duration::from_secs(5),
                    )?;
                    if response.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                        bail!("worker release refused");
                    }
                    let deadline = Instant::now() + Duration::from_secs(1);
                    let child = self.children.get_mut(&run).context("release lost owner")?;
                    while child.try_wait()?.is_none() {
                        if Instant::now() >= deadline {
                            bail!("release reap timeout");
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    self.children.remove(&run);
                    // Only a successful teardown ACK and reap justify this record.
                    self.db
                        .execute("UPDATE snapshots SET state='released' WHERE run=?1", [&run])?;
                }
                None
            }
        };
        Ok(KeeperResponse { ok: true, snapshot })
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        for child in self.children.values_mut() {
            let _ = child.kill();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            self.children
                .retain(|_, child| !matches!(child.try_wait(), Ok(Some(_))));
            if self.children.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // Unproven teardown remains unreleased in the journal.
    }
}
