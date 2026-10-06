//! Durable per-run session allocations, separate from keeper control storage.
use super::Owner;
use anyhow::{bail, Context, Result};
use arda_engine::objectives::runtime_policy::{GrantRole, ValidatedRuntimePolicy};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf};

type SavedAllocation = (
    String,
    String,
    String,
    Option<u64>,
    Option<u64>,
    Option<String>,
);

impl Owner {
    pub(super) fn validate_runtime_inspection(
        &self,
        run: &str,
        manifest: &arda_engine::objectives::snapshot_protocol::Manifest,
        policy: Option<&ValidatedRuntimePolicy>,
    ) -> Result<()> {
        manifest.validate_runtime(policy)?;
        if let Some(policy) = policy {
            let (device, inode): (u64, u64) = self.db.query_row(
                "SELECT device,inode FROM runtime_allocations WHERE run=?1 AND state='allocated'",
                [run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let state = policy
                .policy()
                .grants
                .iter()
                .find(|g| g.role == GrantRole::SessionState)
                .context("session-state grant absent")?;
            let captured = manifest
                .runtime_bundle
                .as_ref()
                .context("runtime bundle missing")?
                .grants
                .iter()
                .find(|g| g.id == state.id)
                .context("session-state capture missing")?;
            if captured.device != device || captured.inode != inode {
                bail!("session-state capture differs from durable allocation");
            }
        }
        Ok(())
    }
    pub(super) fn run_policy(
        &self,
        run: &str,
    ) -> Result<Option<(ValidatedRuntimePolicy, fs::File)>> {
        if self.abandoned(run)? {
            return Err(
                arda_engine::objectives::keeper_client::KeeperFailure::OperatorAbandoned.into(),
            );
        }
        let Some(template) = &self.runtime_policy else {
            return Ok(None);
        };
        let reserved = self
            .reservations
            .as_ref()
            .context("runtime reservations missing")?;
        reserved.revalidate()?;
        let basename = format!("run-{:x}", Sha256::digest(run.as_bytes()));
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS runtime_allocations(
            run TEXT PRIMARY KEY, template_digest TEXT NOT NULL, source TEXT NOT NULL,
            state TEXT NOT NULL, device INTEGER, inode INTEGER, policy TEXT)",
        )?;
        // A creation/open substitution must never adopt another run's recorded
        // directory, including released-but-retained session artifacts.
        self.db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS runtime_allocation_object ON runtime_allocations(device,inode) WHERE device IS NOT NULL AND inode IS NOT NULL")?;
        let saved: Option<SavedAllocation> =
            self.db.query_row("SELECT template_digest,source,state,device,inode,policy FROM runtime_allocations WHERE run=?1", [run],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
        if let Some((digest, source, state, device, inode, policy)) = saved {
            if digest != template.digest() || state != "allocated" {
                bail!("runtime allocation is not recoverable under this policy");
            }
            let allocation = reserved
                .allocations
                .open_relative(std::path::Path::new(&basename))?;
            let metadata = allocation.metadata()?;
            if !metadata.is_dir()
                || Some(metadata.dev()) != device
                || Some(metadata.ino()) != inode
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.mode() & 0o077 != 0
            {
                bail!("saved runtime session allocation identity changed");
            }
            let saved_policy = ValidatedRuntimePolicy::parse(
                policy
                    .context("runtime allocation missing policy")?
                    .as_bytes(),
            )?;
            let mut expected = template.policy().clone();
            let state = expected
                .grants
                .iter_mut()
                .find(|g| g.role == GrantRole::SessionState)
                .context("session-state grant absent")?;
            let expected_source = state
                .source
                .join(format!("run-{:x}", Sha256::digest(run.as_bytes())));
            if expected_source.as_os_str() != std::ffi::OsStr::new(&source) {
                bail!("saved allocation source differs from the configured run binding");
            }
            state.source = expected_source;
            let expected = ValidatedRuntimePolicy::parse(&serde_json::to_vec(&expected)?)?;
            if saved_policy.digest() != expected.digest() {
                bail!("saved allocation policy differs from its template and source");
            }
            reserved.revalidate()?;
            return Ok(Some((saved_policy, allocation)));
        }
        let base = &template
            .policy()
            .grants
            .iter()
            .find(|g| g.role == GrantRole::SessionState)
            .context("session-state grant absent")?
            .source;
        let metadata = fs::symlink_metadata(base)?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            bail!("runtime session allocation parent must be private and owned");
        }
        let source: PathBuf = base.join(format!("run-{:x}", Sha256::digest(run.as_bytes())));
        self.db.execute("INSERT INTO runtime_allocations(run,template_digest,source,state) VALUES(?1,?2,?3,'allocating')",
            params![run, template.digest(), source.to_str().context("UTF-8 runtime allocation required")?])?;
        // Never adopt an existing directory after an uncertain creation. The
        // allocating journal entry leaves an explicit fail-closed recovery case.
        #[cfg(test)]
        super::preparation_tests::crash("allocation_intent");
        let allocation = reserved.allocations.mkdir_private_child(&basename)?;
        #[cfg(test)]
        super::preparation_tests::crash("allocation_directory");
        let metadata = allocation.metadata()?;
        let mut effective = template.policy().clone();
        effective
            .grants
            .iter_mut()
            .find(|g| g.role == GrantRole::SessionState)
            .unwrap()
            .source = source;
        let effective = ValidatedRuntimePolicy::parse(&serde_json::to_vec(&effective)?)?;
        self.db.execute("UPDATE runtime_allocations SET state='allocated',device=?2,inode=?3,policy=?4 WHERE run=?1 AND state='allocating'",
            params![run,metadata.dev(),metadata.ino(),serde_json::to_string(effective.policy())?])?;
        reserved.revalidate()?;
        #[cfg(test)]
        super::preparation_tests::crash("allocation_saved");
        Ok(Some((effective, allocation)))
    }
}
