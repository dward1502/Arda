//! Keeper-held pins survive until the captured worker has been qualified.
use crate::snapshot_physical::Physical;
use anyhow::{bail, Result};
use arda_engine::objectives::{
    capture_envelope::{ExpectedCaptureEnvelope, ExpectedGrant},
    runtime_policy::{GrantAccess, GrantRole, ValidatedRuntimePolicy},
};
use std::{collections::BTreeMap, fs, io::Read, path::Path, sync::Arc};
#[cfg(test)]
mod tests;

pub(crate) struct Reservations {
    durable: Physical,
    runtime: Physical,
    pub(super) allocations: Physical,
}
impl Reservations {
    pub(crate) fn storage_pins(&self) -> Result<(fs::File, fs::File)> {
        Ok((
            self.durable.open_relative(Path::new(""))?,
            self.runtime.open_relative(Path::new(""))?,
        ))
    }
    pub(crate) fn pin(durable: &Path, runtime: &Path, allocations: &Path) -> Result<Arc<Self>> {
        let baseline = fs::read("/proc/thread-self/mountinfo")?;
        let pin = |path: &Path| Physical::pin_handle(path, Physical::open_source(path)?, &baseline);
        let result = Self {
            durable: pin(durable)?,
            runtime: pin(runtime)?,
            allocations: pin(allocations)?,
        };
        result.revalidate()?;
        if fs::read("/proc/thread-self/mountinfo")? != baseline {
            bail!("reservation topology changed during pinning");
        }
        Ok(Arc::new(result))
    }
    pub(crate) fn bind_storage(
        &self,
        durable_lock: &fs::File,
        endpoint_lock: &fs::File,
    ) -> Result<()> {
        self.revalidate()?;
        self.durable
            .matches_file(Path::new("owner.lock"), durable_lock)?;
        self.runtime
            .matches_file(Path::new("endpoint.lock"), endpoint_lock)?;
        self.revalidate()
    }

    pub(crate) fn revalidate(&self) -> Result<()> {
        self.durable.revalidate_coordinate()?;
        self.runtime.revalidate_coordinate()?;
        self.allocations.revalidate_coordinate()
    }
}

pub(super) struct PendingAdmission {
    // These pins are deliberately not serialized or inherited by the worker.
    _reservations: Arc<Reservations>,
    workspace: Physical,
    grants: BTreeMap<String, Physical>,
    pub(super) envelope: ExpectedCaptureEnvelope,
}
impl PendingAdmission {
    pub(super) fn verify_manifest(
        &self,
        policy: &ValidatedRuntimePolicy,
        manifest: &arda_engine::objectives::snapshot_protocol::Manifest,
    ) -> Result<()> {
        manifest.validate_runtime(Some(policy))?;
        let expected_digest = self.envelope.digest()?;
        if manifest.admission_digest.as_deref() != Some(expected_digest.as_str())
            || manifest.topology_digest != self.envelope.workspace.digest()?
            || (manifest.device, manifest.inode)
                != (
                    self.envelope.workspace.mounts[0].object.device,
                    self.envelope.workspace.mounts[0].object.inode,
                )
        {
            bail!("worker workspace/envelope attestation mismatch");
        }
        let bundle = manifest
            .runtime_bundle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing runtime bundle"))?;
        for ((expected, actual), grant) in self
            .envelope
            .grants
            .iter()
            .zip(&bundle.grants)
            .zip(&policy.policy().grants)
        {
            let tree = if grant.access == GrantAccess::ReadOnly {
                expected.source.readonly()?
            } else {
                expected.source.clone()
            };
            if actual.topology_digest != tree.digest()?
                || (actual.device, actual.inode)
                    != (tree.mounts[0].object.device, tree.mounts[0].object.inode)
            {
                bail!("worker grant attestation mismatch");
            }
        }
        Ok(())
    }
    pub(super) fn pin(
        reserved: Arc<Reservations>,
        workspace: &Path,
        identity: &str,
        run: &str,
        policy: &ValidatedRuntimePolicy,
    ) -> Result<Self> {
        reserved.revalidate()?;
        let baseline = fs::read("/proc/thread-self/mountinfo")?;
        let pin = |path: &Path| Physical::pin_handle(path, Physical::open_source(path)?, &baseline);
        let workspace = pin(workspace)?;
        let mut grants = BTreeMap::new();
        for grant in &policy.policy().grants {
            grants.insert(grant.id.clone(), pin(&grant.source)?);
        }
        for protected in [&reserved.durable, &reserved.runtime, &reserved.allocations] {
            if workspace.overlaps_current(protected)? {
                bail!("workspace overlaps pinned owner reservation");
            }
        }
        for grant in &policy.policy().grants {
            let source = &grants[&grant.id];
            for protected in [&reserved.durable, &reserved.runtime, &workspace] {
                if source.overlaps_current(protected)? {
                    bail!("grant overlaps pinned owner/workspace authority");
                }
            }
            if grant.role != GrantRole::SessionState
                && source.overlaps_current(&reserved.allocations)?
            {
                bail!("grant overlaps another run's state reservation");
            }
            if grant.access == GrantAccess::ReadWrite {
                for other in &policy.policy().grants {
                    if grant.id != other.id && source.overlaps_current(&grants[&other.id])? {
                        bail!("writable source overlaps another runtime grant");
                    }
                }
            }
        }
        let mut nonce = [0u8; 32];
        fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
        let envelope = ExpectedCaptureEnvelope {
            version: match serde_json::from_str::<(
                u32,
                std::path::PathBuf,
                std::path::PathBuf,
                Option<(u64, u64)>,
                String,
            )>(identity)?
            .0
            {
                2 => 1,
                3 => 2,
                _ => bail!("unsupported workspace identity version"),
            },
            admission_nonce: nonce.iter().map(|byte| format!("{byte:02x}")).collect(),
            run_id: run.to_owned(),
            workspace_identity: identity.to_owned(),
            policy_digest: policy.digest().to_owned(),
            workspace: workspace.witness()?,
            grants: policy
                .policy()
                .grants
                .iter()
                .map(|g| {
                    Ok(ExpectedGrant {
                        id: g.id.clone(),
                        source: grants[&g.id].witness()?,
                    })
                })
                .collect::<Result<_>>()?,
        };
        envelope.validate(policy, run, identity)?;
        let pending = Self {
            _reservations: reserved,
            workspace,
            grants,
            envelope,
        };
        pending.revalidate()?;
        if fs::read("/proc/thread-self/mountinfo")? != baseline {
            bail!("pending admission topology changed during pinning");
        }
        Ok(pending)
    }
    pub(super) fn revalidate(&self) -> Result<()> {
        self._reservations.revalidate()?;
        self.workspace.revalidate_coordinate()?;
        for grant in self.grants.values() {
            grant.revalidate_coordinate()?;
        }
        Ok(())
    }
}
