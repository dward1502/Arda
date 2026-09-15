//! Private keeper/worker transport. Never place capability-bearing values in
//! operator projections, prompts, execution receipts, or Debug output.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const MAX_TIMEOUT_MS: u64 = 300_000;
pub const MAX_OUTPUT_BYTES: usize = 1_048_576;
pub const MAX_REQUEST_BYTES: usize = 2_097_152;
// JSON escaping can expand each raw byte to six bytes in both output streams.
pub const MAX_RESPONSE_BYTES: usize = 12 * MAX_OUTPUT_BYTES + 65_536;

pub fn default_output_limit() -> usize {
    65_536
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    pub run_id: String,
    pub generation: i64,
    pub owner: String,
    pub expires_ms: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub capability: String,
    pub root: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub topology_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_bundle: Option<RuntimeBundle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_digest: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBundle {
    pub version: u32,
    pub policy_digest: String,
    pub grants: Vec<CapturedRuntimeGrant>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedRuntimeGrant {
    pub id: String,
    pub destination: PathBuf,
    pub kind: super::runtime_policy::GrantKind,
    pub access: super::runtime_policy::GrantAccess,
    pub device: u64,
    pub inode: u64,
    pub topology_digest: String,
}
impl Manifest {
    pub fn validate_runtime(
        &self,
        expected: Option<&super::runtime_policy::ValidatedRuntimePolicy>,
    ) -> anyhow::Result<()> {
        match (self.version, &self.runtime_bundle, expected) {
            (1, None, None) if self.admission_digest.is_none() => Ok(()),
            (2, Some(bundle), Some(policy))
                if bundle.version == 1 && bundle.policy_digest == policy.digest() =>
            {
                anyhow::ensure!(
                    bundle.grants.len() == policy.policy().grants.len(),
                    "runtime grant count mismatch"
                );
                for (actual, expected) in bundle.grants.iter().zip(&policy.policy().grants) {
                    anyhow::ensure!(
                        actual.id == expected.id
                            && actual.destination.as_os_str() == expected.destination.as_os_str()
                            && actual.kind == expected.kind
                            && actual.access == expected.access,
                        "runtime grant set mismatch"
                    );
                    anyhow::ensure!(
                        actual.inode != 0
                            && actual.topology_digest.len() == 64
                            && actual
                                .topology_digest
                                .bytes()
                                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                        "invalid captured runtime identity"
                    );
                }
                Ok(())
            }
            _ => anyhow::bail!("runtime manifest version or policy mismatch"),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Runtime {
        capability: String,
        lease: Lease,
        operation: super::runtime_operation::RuntimeOperation,
        timeout_ms: u64,
        max_output_bytes: usize,
    },
    Inspect,
    Commit {
        capability: String,
        manifest_digest: String,
        lease: Lease,
    },
    Execute {
        capability: String,
        lease: Lease,
        argv: Vec<String>,
        environment: BTreeMap<String, String>,
        timeout_ms: u64,
        #[serde(default = "default_output_limit")]
        max_output_bytes: usize,
    },
    Release {
        capability: String,
    },
}
