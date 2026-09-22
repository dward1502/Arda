use super::*;
pub(crate) mod pending;
#[cfg(test)]
mod preparation_tests;
mod qualification;
mod runtime_state;

pub struct Owner {
    pub managed: Option<super::keeper_managed::Binding>,
    pub reservations: Option<std::sync::Arc<pending::Reservations>>,
    pub runtime_policy: Option<ValidatedRuntimePolicy>,
    pub db: super::keeper_storage::PinnedConnection,
    pub durable: PathBuf,
    pub runtime: PathBuf,
    pub worker: PathBuf,
    pub children: BTreeMap<String, Child>,
    pub(crate) failed_qualifications: qualification::Cleanup,
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
        let _ = std::fs::write("/tmp/keeper-debug.log", "H1\n");
        let cleanup_pending = self.failed_qualifications.pending();
        let _ = std::fs::write("/tmp/keeper-debug.log", "H2\n");
        let snapshot = match request {
            KeeperRequest::Prepare {
                run,
                workspace,
                identity,
            } => {
                if cleanup_pending {
                    let _ = std::fs::write("/tmp/keeper-debug.log", "REJECT_cleanup_pending\n");
                    bail!("preparation cleanup remains unproven; new admission refused");
                }
                if run.is_empty() || run.len() > 1024 {
                    let _ = std::fs::write("/tmp/keeper-debug.log", "REJECT_run_length\n");
                    bail!("invalid admission identifier");
                }
                if let Some((root, prior, state, saved)) = self.saved(&run)? {
                    let _ = std::fs::write("/tmp/keeper-debug.log", format!("H_saved root={} prior={} state={}\n", root, prior, state));
                    if root != workspace.to_str().context("UTF-8 workspace required")?
                        || prior != identity
                        || state != "prepared"
                    {
                        let _ = std::fs::write("/tmp/keeper-debug.log", "REJECT_saved_mismatch\n");
                        bail!("admission requires reconciliation");
                    }
                    self.live(&run)?;
                    Some(serde_json::from_str(
                        &saved.context("missing retained authority")?,
                    )?)
                } else {
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_no_saved\n");
                    arda_engine::objectives::validate_snapshot_owner_paths(
                        Path::new("/usr"),
                        &[&self.durable, &self.runtime],
                    )?;
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_usr_ok\n");
                    arda_engine::objectives::validate_snapshot_owner_paths(
                        &workspace,
                        &[&self.durable, &self.runtime],
                    )?;
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_ws_ok\n");
                    // FULL synchronous commit before the first process side effect.
                    if let Some(policy) = &self.runtime_policy {
                        let allocation_base = &policy.policy().grants.iter()
                            .find(|g| g.role == arda_engine::objectives::runtime_policy::GrantRole::SessionState)
                            .context("session-state allocation base absent")?.source;
                        arda_engine::objectives::validate_snapshot_owner_paths(
                            &workspace,
                            &[allocation_base],
                        )?;
                        let _ = std::fs::write("/tmp/keeper-debug.log", "H_alloc_ok\n");
                        for grant in &policy.policy().grants {
                            if grant.role
                                != arda_engine::objectives::runtime_policy::GrantRole::SessionState
                            {
                                arda_engine::objectives::validate_snapshot_owner_paths(
                                    &grant.source,
                                    &[allocation_base],
                                )?;
                            }
                            arda_engine::objectives::validate_snapshot_owner_paths(
                                &grant.source,
                                &[&self.durable, &self.runtime],
                            )?;
                        }
                        let _ = std::fs::write("/tmp/keeper-debug.log", "H_grants_ok\n");
                    }
                    // Persist the lifetime and admission together before any
                    // allocation or worker side effect; uncertain rows stay fenced.
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_txn_start\n");
                    let transaction = self.db.transaction()?;
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_txn_ok\n");
                    super::keeper_managed::save(&transaction, &run, self.managed.as_ref())
                        .context("managed_binding")?;
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_save_ok\n");
                    transaction.execute("INSERT INTO snapshots(run,workspace,identity,state) VALUES(?1,?2,?3,'preparing')", params![run,workspace.to_str().context("UTF-8 workspace required")?,identity])?;
                    let _ = std::fs::write("/tmp/keeper-debug.log", "H_insert_ok\n");
                    transaction.commit()?;
                    #[cfg(test)]
                    preparation_tests::crash("preparing");
                    let (run_policy, _allocation_pin) = {
                        match self.run_policy(&run).context("runtime_allocation")? {
                            Some((policy, pin)) => (Some(policy), Some(pin)),
                            None => (None, None),
                        }
                    };
                    let pending = run_policy
                        .as_ref()
                        .map(|policy| {
                            pending::PendingAdmission::pin(
                                self.reservations
                                    .as_ref()
                                    .context("missing reservations")?
                                    .clone(),
                                &workspace,
                                &identity,
                                &run,
                                policy,
                            )
                        })
                        .transpose()
                        .context("pending_pins")?;

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
                    let policy_descriptor = run_policy
                        .as_ref()
                        .map(arda_engine::objectives::runtime_policy::transport::seal)
                        .transpose()?;
                    use std::os::fd::AsRawFd;
                    let policy_fd = policy_descriptor.as_ref().map(|file| file.as_raw_fd());
                    let envelope_descriptor = pending
                        .as_ref()
                        .map(|p| {
                            p.envelope.seal(
                                run_policy.as_ref().context("missing policy")?,
                                &run,
                                &identity,
                            )
                        })
                        .transpose()?;
                    let envelope_fd = envelope_descriptor.as_ref().map(|file| file.as_raw_fd());
                    if let Some(fd) = policy_fd {
                        command.arg(fd.to_string());
                    }
                    if let Some(fd) = envelope_fd {
                        command.arg(fd.to_string());
                    }
                    // Async-signal-safe parent-death arm, closing the spawn race.
                    unsafe {
                        command.pre_exec(move || {
                            if let Some(fd) = envelope_fd {
                                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                                    return Err(std::io::Error::last_os_error());
                                }
                            }
                            if let Some(fd) = policy_fd {
                                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                                    return Err(std::io::Error::last_os_error());
                                }
                            }
                            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                                return Err(std::io::Error::last_os_error());
                            }
                            if libc::getppid() != parent {
                                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                            }
                            Ok(())
                        });
                    }
                    let mut qualification = qualification::Qualification {
                        child: Some(command.spawn().context("worker_spawn")?),
                        pending,
                        cleanup: self.failed_qualifications.clone(),
                    };
                    let deadline = Instant::now() + Duration::from_secs(4);
                    #[cfg(test)]
                    preparation_tests::crash("worker_spawned");
                    let inspection: Inspection = loop {
                        if qualification.child.as_mut().unwrap().try_wait()?.is_some() {
                            return Err(anyhow::anyhow!(
                                "snapshot worker lost during qualification"
                            )
                            .context("worker_qualification"));
                        }
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
                    self.validate_runtime_inspection(
                        &run,
                        &inspection.manifest,
                        run_policy.as_ref(),
                    )?;
                    if let Some(pending) = &qualification.pending {
                        pending.verify_manifest(
                            run_policy.as_ref().context("missing run policy")?,
                            &inspection.manifest,
                        )?;
                    }
                    if !inspection.ok || inspection.manifest.root != workspace {
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
                    #[cfg(test)]
                    preparation_tests::crash("qualified");
                    self.db.execute(
                        "UPDATE snapshots SET state='prepared',authority=?2 WHERE run=?1",
                        params![run, serde_json::to_string(&snapshot)?],
                    )?;
                    self.children
                        .insert(run.clone(), qualification.into_child());
                    #[cfg(test)]
                    preparation_tests::crash("prepared");
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
                if state == "reconciled_revoked" {
                    super::keeper_reconcile::release_ack(&self.db, &run, &self.runtime)?;
                } else if state != "released" {
                    if state != "prepared" && state != "releasing" {
                        bail!("release requires explicit reconciliation");
                    }
                    // A failed delivery/refusal is retryable while this owner
                    // still holds the live child. Absence is never cleanup proof;
                    // worker ACK loss/death still requires explicit reconciliation.
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
                    let transaction = self.db.transaction()?;
                    transaction
                        .execute("UPDATE snapshots SET state='released' WHERE run=?1", [&run])?;
                    if self.runtime_policy.is_some() {
                        // Retain session artifacts, but retire their executable
                        // allocation atomically with the snapshot authority.
                        transaction.execute(
                            "UPDATE runtime_allocations SET state='released' WHERE run=?1",
                            [&run],
                        )?;
                    }
                    transaction.commit()?;
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
            if self.children.is_empty() && !self.failed_qualifications.pending() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // Unproven teardown remains unreleased in the journal.
    }
}
