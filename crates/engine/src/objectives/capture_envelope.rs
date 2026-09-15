//! Expected captures, sent only through the keeper-owned sealed-FD handoff.
//! This envelope is not a signature and does not replace the keeper's live pins.
use super::{
    runtime_policy::{transport, GrantAccess, GrantKind, ValidatedRuntimePolicy},
    tree_witness::TreeWitness,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::os::fd::OwnedFd;

pub const MAX_ENVELOPE_BYTES: usize = 1_048_576;
#[cfg(test)]
mod tests;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedGrant {
    pub id: String,
    pub source: TreeWitness,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCaptureEnvelope {
    pub version: u32,
    pub admission_nonce: String,
    pub run_id: String,
    pub workspace_identity: String,
    pub policy_digest: String,
    pub workspace: TreeWitness,
    pub grants: Vec<ExpectedGrant>,
}
impl ExpectedCaptureEnvelope {
    /// Worker-side decoding. The keeper separately compares the expected digest
    /// for its run; a value received on an arbitrary FD is not authentication.
    pub fn read_worker(
        fd: std::os::fd::OwnedFd,
        policy: &ValidatedRuntimePolicy,
        identity: &str,
    ) -> Result<Self> {
        let bytes = transport::read_bytes(fd, "arda-capture-envelope", MAX_ENVELOPE_BYTES)?;
        let envelope: Self = serde_json::from_slice(&bytes)?;
        envelope.validate(policy, &envelope.run_id, identity)?;
        Ok(envelope)
    }
    pub fn validate(
        &self,
        policy: &ValidatedRuntimePolicy,
        run: &str,
        identity: &str,
    ) -> Result<()> {
        if !matches!(self.version, 1 | 2)
            || self.run_id.is_empty()
            || self.run_id.len() > 1024
            || self.run_id != run
            || self.workspace_identity != identity
            || identity.len() > 16384
            || self.policy_digest != policy.digest()
            || self.admission_nonce.len() != 64
            || !self
                .admission_nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.grants.len() != policy.policy().grants.len()
        {
            bail!("capture envelope binding mismatch");
        }
        self.workspace.validate()?;
        let admitted: (
            u32,
            std::path::PathBuf,
            std::path::PathBuf,
            Option<(u64, u64)>,
            String,
        ) = serde_json::from_str(identity)?;
        let root = &self.workspace.mounts[0].object;
        if !matches!((self.version, admitted.0), (1, 2) | (2, 3))
            || admitted.1 != admitted.2
            || admitted.3 != Some((root.device, root.inode))
            || root.kind != libc::S_IFDIR
        {
            bail!("workspace witness differs from admitted identity");
        }
        if admitted.0 == 3 && self.workspace.digest()? != admitted.4 {
            bail!("workspace tree differs from original retained admission");
        }
        for (actual, expected) in self.grants.iter().zip(&policy.policy().grants) {
            actual.source.validate()?;
            let kind = if expected.kind == GrantKind::Directory {
                libc::S_IFDIR
            } else {
                libc::S_IFREG
            };
            if actual.id != expected.id || actual.source.mounts[0].object.kind != kind {
                bail!("capture grant set/type mismatch");
            }
            if expected.kind == GrantKind::File && actual.source.mounts.len() != 1 {
                bail!("file grant has descendants");
            }
        }
        if self.bytes()?.len() > MAX_ENVELOPE_BYTES {
            bail!("capture envelope exceeds bound");
        }
        Ok(())
    }
    fn bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
    pub fn digest(&self) -> Result<String> {
        Ok(format!("{:x}", Sha256::digest(self.bytes()?)))
    }
    pub fn seal(
        &self,
        policy: &ValidatedRuntimePolicy,
        run: &str,
        identity: &str,
    ) -> Result<OwnedFd> {
        self.validate(policy, run, identity)?;
        transport::seal_bytes(&self.bytes()?, c"arda-capture-envelope", MAX_ENVELOPE_BYTES)
    }
    pub fn read(
        fd: OwnedFd,
        policy: &ValidatedRuntimePolicy,
        run: &str,
        identity: &str,
    ) -> Result<Self> {
        let bytes = transport::read_bytes(fd, "arda-capture-envelope", MAX_ENVELOPE_BYTES)?;
        let result: Self = serde_json::from_slice(&bytes)?;
        result.validate(policy, run, identity)?;
        Ok(result)
    }
    pub fn verify_captures(
        &self,
        policy: &ValidatedRuntimePolicy,
        workspace: &TreeWitness,
        grants: &[TreeWitness],
    ) -> Result<()> {
        self.validate(policy, &self.run_id, &self.workspace_identity)?;
        workspace.validate()?;
        if self.workspace != *workspace || grants.len() != self.grants.len() {
            bail!("workspace/capture set changed");
        }
        for ((expected, actual), grant) in
            self.grants.iter().zip(grants).zip(&policy.policy().grants)
        {
            actual.validate()?;
            let expected = if grant.access == GrantAccess::ReadOnly {
                expected.source.readonly()?
            } else {
                expected.source.clone()
            };
            if expected != *actual {
                bail!("captured grant differs from keeper witness");
            }
        }
        Ok(())
    }
}
