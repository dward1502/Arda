//! Keeper-owned runtime authority. Prepare requests cannot supply these grants.
#[cfg(target_os = "linux")]
pub mod transport;
pub const MAX_POLICY_BYTES: usize = 65_536;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantKind {
    File,
    Directory,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantAccess {
    ReadOnly,
    ReadWrite,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantRole {
    Runtime,
    ProfileInput,
    SessionState,
    NetworkInput,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGrant {
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub kind: GrantKind,
    pub access: GrantAccess,
    pub role: GrantRole,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonEntrypoint {
    pub interpreter: PathBuf,
    pub ordered_import_roots: Vec<PathBuf>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePolicy {
    pub version: u32,
    pub entrypoint: PythonEntrypoint,
    pub grants: Vec<RuntimeGrant>,
    pub fixed_environment: BTreeMap<String, String>,
}

/// No Debug: policy paths and profile selectors are private authority metadata.
#[derive(Clone)]
pub struct ValidatedRuntimePolicy {
    policy: RuntimePolicy,
    digest: String,
}

fn absolute_clean(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
        && !path.as_os_str().is_empty()
        && !path.as_os_str().as_encoded_bytes().contains(&0)
        && path.as_os_str() == path.components().collect::<PathBuf>().as_os_str()
        && !path
            .as_os_str()
            .as_encoded_bytes()
            .windows(3)
            .any(|w| w == b"/./")
        && !path
            .as_os_str()
            .as_encoded_bytes()
            .windows(2)
            .any(|w| w == b"//")
}
fn readonly_runtime_covers(grants: &[RuntimeGrant], path: &Path) -> bool {
    grants.iter().any(|g| {
        g.role == GrantRole::Runtime
            && g.access == GrantAccess::ReadOnly
            && (g.destination == path
                || (g.kind == GrantKind::Directory && path.starts_with(&g.destination)))
    })
}
impl ValidatedRuntimePolicy {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_POLICY_BYTES {
            bail!("runtime policy exceeds byte limit")
        }
        let policy: RuntimePolicy =
            serde_json::from_slice(bytes).context("invalid runtime policy JSON")?;
        Self::validate(policy)
    }
    pub fn validate(mut policy: RuntimePolicy) -> Result<Self> {
        if policy.version != 1 || policy.grants.is_empty() || policy.grants.len() > 32 {
            bail!("unsupported runtime policy version or grant count")
        }
        let mut ids = BTreeSet::new();
        for g in &policy.grants {
            if g.id.is_empty()
                || g.id.len() > 64
                || !g
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !ids.insert(&g.id)
            {
                bail!("invalid or duplicate runtime grant ID")
            }
            for path in [&g.source, &g.destination] {
                if !absolute_clean(path) {
                    bail!("runtime grant path must be absolute and normalized")
                }
                if [
                    "/",
                    "/home",
                    "/var/home",
                    "/root",
                    "/etc",
                    "/run",
                    "/tmp",
                    "/var",
                    "/opt",
                ]
                .iter()
                .any(|p| path == Path::new(p))
                {
                    bail!("broad runtime grant is forbidden")
                }
                if ["/proc", "/sys", "/dev"]
                    .iter()
                    .any(|p| path.starts_with(p))
                {
                    bail!("runtime grant crosses reserved kernel paths")
                }
                if path
                    .parent()
                    .is_some_and(|p| p == Path::new("/home") || p == Path::new("/var/home"))
                {
                    bail!("whole-user-home grant is forbidden")
                }
                if path
                    .file_name()
                    .is_some_and(|n| n == ".hermes" || n == ".local")
                {
                    bail!("ambient home grant is forbidden")
                }
            }
            if g.role == GrantRole::SessionState {
                if g.access != GrantAccess::ReadWrite || g.kind != GrantKind::Directory {
                    bail!("session state must be a bounded writable directory")
                }
            } else if g.access != GrantAccess::ReadOnly {
                bail!("only session state may be writable")
            }
            if g.role == GrantRole::NetworkInput && g.kind != GrantKind::File {
                bail!("network inputs must be exact files")
            }
        }
        let states: Vec<_> = policy
            .grants
            .iter()
            .filter(|g| g.role == GrantRole::SessionState)
            .collect();
        if states.len() != 1 {
            bail!("exactly one session-state grant is required")
        }
        for (i, a) in policy.grants.iter().enumerate() {
            for b in policy.grants.iter().skip(i + 1) {
                // A readonly profile input may deliberately overlay a state child.
                // Everything else must have disjoint sandbox destinations.
                let overlay = |parent: &RuntimeGrant, child: &RuntimeGrant| {
                    parent.role == GrantRole::SessionState
                        && child.role == GrantRole::ProfileInput
                        && child.kind == GrantKind::File
                        && child.destination != parent.destination
                        && child.destination.starts_with(&parent.destination)
                };
                if (a.destination.starts_with(&b.destination)
                    || b.destination.starts_with(&a.destination))
                    && !overlay(a, b)
                    && !overlay(b, a)
                {
                    bail!("overlapping runtime destinations")
                }
            }
        }
        if !policy.grants.iter().any(|g| {
            g.source == Path::new("/usr")
                && g.destination == Path::new("/usr")
                && g.kind == GrantKind::Directory
                && g.role == GrantRole::Runtime
        }) {
            bail!("captured readonly /usr runtime is required")
        }
        if !absolute_clean(&policy.entrypoint.interpreter)
            || !policy.entrypoint.interpreter.starts_with("/usr/bin")
            || !readonly_runtime_covers(&policy.grants, &policy.entrypoint.interpreter)
        {
            bail!("interpreter must be within captured /usr/bin")
        }
        if policy.entrypoint.ordered_import_roots.is_empty()
            || policy.entrypoint.ordered_import_roots.len() > 16
        {
            bail!("invalid import-root count")
        }
        let mut roots = BTreeSet::new();
        for root in &policy.entrypoint.ordered_import_roots {
            if !absolute_clean(root)
                || !roots.insert(root)
                || !readonly_runtime_covers(&policy.grants, root)
            {
                bail!("import root is not unique readonly runtime authority")
            }
        }
        let allowed = [
            "HOME",
            "HERMES_HOME",
            "PATH",
            "SSL_CERT_FILE",
            "REQUESTS_CA_BUNDLE",
            "CURL_CA_BUNDLE",
            "LANG",
            "LC_ALL",
        ];
        for (key, value) in &policy.fixed_environment {
            if !allowed.contains(&key.as_str()) || value.len() > 4096 || value.contains('\0') {
                bail!("unsupported fixed runtime environment")
            }
        }
        for ca in ["SSL_CERT_FILE", "REQUESTS_CA_BUNDLE", "CURL_CA_BUNDLE"] {
            if let Some(path) = policy.fixed_environment.get(ca) {
                if !absolute_clean(Path::new(path))
                    || !policy.grants.iter().any(|g| {
                        g.role == GrantRole::NetworkInput
                            && g.access == GrantAccess::ReadOnly
                            && g.kind == GrantKind::File
                            && g.destination == Path::new(path)
                    })
                {
                    bail!("CA routing must select a captured readonly network input")
                }
            }
        }
        for key in ["HOME", "HERMES_HOME"] {
            if !policy
                .fixed_environment
                .get(key)
                .is_some_and(|value| absolute_clean(Path::new(value)))
            {
                bail!("profile environment path must use canonical spelling")
            }
            // Profile selection is tied to the state grant, not ambient HOME.
            if policy.fixed_environment.get(key).map(Path::new)
                != Some(states[0].destination.as_path())
            {
                bail!("HOME and HERMES_HOME must select the bounded state grant")
            }
        }
        if policy.fixed_environment.get("PATH").map(String::as_str) != Some("/usr/bin:/bin") {
            bail!("runtime PATH must be fixed")
        }
        policy.grants.sort_by(|a, b| a.id.cmp(&b.id));
        let bytes = serde_json::to_vec(&policy)?;
        let digest = format!("{:x}", Sha256::digest(bytes));
        Ok(Self { policy, digest })
    }
    pub fn policy(&self) -> &RuntimePolicy {
        &self.policy
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.policy)?)
    }
}

#[cfg(test)]
mod tests;
