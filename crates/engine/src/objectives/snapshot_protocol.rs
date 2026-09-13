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
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
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
