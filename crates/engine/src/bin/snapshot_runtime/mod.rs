//! Complete runtime admission/capture; no partial bundle is publishable.
mod dispatch;
use crate::snapshot_admission::grant::{CapturedGrant, GrantAdmission};
use anyhow::{bail, Result};
use arda_engine::objectives::{
    runtime_policy::ValidatedRuntimePolicy, snapshot_protocol::RuntimeBundle,
};
use std::{collections::BTreeSet, path::Path};

pub struct RuntimeAdmission {
    workspace: std::path::PathBuf,
    policy: ValidatedRuntimePolicy,
    grants: Vec<GrantAdmission>,
}
pub struct CapturedRuntime {
    pub policy: ValidatedRuntimePolicy,
    pub grants: Vec<CapturedGrant>,
}
impl RuntimeAdmission {
    pub fn policy(&self) -> &ValidatedRuntimePolicy {
        &self.policy
    }
    fn validate_sources(policy: &ValidatedRuntimePolicy, workspace: &Path) -> Result<()> {
        use arda_engine::objectives::{runtime_policy::GrantAccess, validate_snapshot_owner_paths};
        for grant in &policy.policy().grants {
            validate_snapshot_owner_paths(&grant.source, &[workspace])?;
            if grant.access == GrantAccess::ReadWrite {
                for other in &policy.policy().grants {
                    if grant.id != other.id {
                        validate_snapshot_owner_paths(&grant.source, &[&other.source])?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn before_clone(policy: ValidatedRuntimePolicy, workspace: &Path) -> Result<Self> {
        let before = std::fs::read("/proc/thread-self/mountinfo")?;
        Self::validate_sources(&policy, workspace)?;
        let mut grants = Vec::new();
        for grant in &policy.policy().grants {
            if grant.destination.starts_with(workspace) || workspace.starts_with(&grant.destination)
            {
                bail!("runtime destination collides with workspace authority");
            }
            // The current bubblewrap layout owns these symlink destinations.
            if ["/bin", "/sbin", "/lib", "/lib64"]
                .iter()
                .any(|p| grant.destination.starts_with(p))
            {
                bail!("runtime destination collides with bootstrap layout");
            }
            grants.push(GrantAdmission::before_clone(grant)?);
        }
        if before != std::fs::read("/proc/thread-self/mountinfo")? {
            bail!("runtime bundle admission topology changed");
        }
        Ok(Self {
            workspace: workspace.to_owned(),
            policy,
            grants,
        })
    }
    pub fn exposed_devices(&self) -> Result<BTreeSet<u64>> {
        let mut devices = BTreeSet::new();
        for grant in &self.grants {
            devices.extend(grant.exposed_devices()?);
        }
        Ok(devices)
    }
    pub fn after_clone(&self) -> Result<()> {
        Self::validate_sources(&self.policy, &self.workspace)?;
        for grant in &self.grants {
            grant.after_clone()?;
        }
        Ok(())
    }
    pub fn capture(self, staging: &Path, excluded_device: u64) -> Result<CapturedRuntime> {
        let mut grants = Vec::new();
        for (index, (admission, grant)) in self
            .grants
            .iter()
            .zip(&self.policy.policy().grants)
            .enumerate()
        {
            let capture = admission.capture(grant, &staging.join(format!("grant-{index}")))?;
            if admission.exposed_devices()?.contains(&excluded_device) {
                bail!("runtime capture shares keeper control filesystem");
            }
            grants.push(capture);
        }
        Ok(CapturedRuntime {
            policy: self.policy,
            grants,
        })
    }
}
impl CapturedRuntime {
    pub fn manifest(&self) -> RuntimeBundle {
        RuntimeBundle {
            version: 1,
            policy_digest: self.policy.digest().to_owned(),
            grants: self
                .grants
                .iter()
                .map(|grant| grant.identity.clone())
                .collect(),
        }
    }
}
