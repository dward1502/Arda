//! Managed lifetime provenance is captured before admission, never reconstructed.
use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub owner: String,
    pub uid: u32,
    pub machine: String,
    pub boot: String,
    pub unit: String,
    pub invocation: String,
    pub cgroup: String,
    pub cgroup_device: u64,
    pub cgroup_inode: u64,
    pub runtime: PathBuf,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn boot() -> Result<String> {
    let value = fs::read_to_string("/proc/sys/kernel/random/boot_id")?
        .trim()
        .to_owned();
    uuid::Uuid::parse_str(&value)?;
    Ok(value)
}
fn machine() -> Result<String> {
    Ok(digest(&fs::read("/etc/machine-id")?))
}
fn command(arguments: &[&str]) -> Result<std::process::Output> {
    let result = Command::new("/usr/bin/timeout")
        .args(["15", "/usr/bin/systemctl", "--user", "--no-pager"])
        .args(arguments)
        .output()?;
    if !result.status.success() {
        bail!("managed systemd operation failed or timed out");
    }
    Ok(result)
}
fn show(unit: &str) -> Result<BTreeMap<String, String>> {
    if !unit.ends_with(".service")
        || unit.starts_with('-')
        || unit.len() > 255
        || !unit
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
    {
        bail!("invalid managed keeper unit");
    }
    let output = command(&["show", unit, "--property=Id,LoadState,ActiveState,SubState,MainPID,InvocationID,ControlGroup,KillMode,Delegate,ProtectControlGroups,RuntimeDirectoryPreserve,UnitFileState"])?;
    let mut result = BTreeMap::new();
    for line in std::str::from_utf8(&output.stdout)?.lines() {
        let (key, value) = line.split_once('=').context("invalid manager properties")?;
        if result.insert(key.to_owned(), value.to_owned()).is_some() {
            bail!("duplicate manager property");
        }
    }
    Ok(result)
}
fn property<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    Ok(values
        .get(key)
        .context("required manager property absent")?)
}
fn cgroup_path(group: &str) -> Result<PathBuf> {
    if !group.starts_with("/user.slice/")
        || group.split('/').any(|part| part == "." || part == "..")
    {
        bail!("invalid managed cgroup coordinate");
    }
    Ok(Path::new("/sys/fs/cgroup").join(group.trim_start_matches('/')))
}

pub fn capture(db: &Connection, runtime: &Path) -> Result<Option<Binding>> {
    let Some(unit) = std::env::var_os("ARDA_KEEPER_SYSTEMD_UNIT") else {
        return Ok(None);
    };
    let unit = unit.to_str().context("invalid managed unit encoding")?;
    let properties = show(unit)?;
    for (key, expected) in [
        ("Id", unit),
        ("KillMode", "control-group"),
        ("Delegate", "no"),
        ("ProtectControlGroups", "yes"),
        ("RuntimeDirectoryPreserve", "yes"),
    ] {
        if property(&properties, key)? != expected {
            bail!("managed keeper effective policy mismatch: {key}");
        }
    }
    if property(&properties, "MainPID")? != std::process::id().to_string() {
        bail!("keeper is not the managed main process");
    }
    let invocation = property(&properties, "InvocationID")?.to_owned();
    uuid::Uuid::parse_str(&invocation)?;
    if std::env::var("INVOCATION_ID").ok().as_deref() != Some(invocation.as_str()) {
        bail!("managed invocation mismatch");
    }
    let cgroup = property(&properties, "ControlGroup")?.to_owned();
    if fs::read_to_string("/proc/self/cgroup")?.trim() != format!("0::{cgroup}") {
        bail!("keeper cgroup membership mismatch");
    }
    let metadata = fs::metadata(cgroup_path(&cgroup)?)?;
    let owner = db.query_row("SELECT id FROM owner_identity", [], |r| r.get(0))?;
    Ok(Some(Binding {
        owner,
        uid: unsafe { libc::geteuid() },
        machine: machine()?,
        boot: boot()?,
        unit: unit.to_owned(),
        invocation,
        cgroup,
        cgroup_device: metadata.dev(),
        cgroup_inode: metadata.ino(),
        runtime: runtime.to_owned(),
    }))
}

pub fn save(db: &Connection, run: &str, binding: Option<&Binding>) -> Result<()> {
    if let Some(binding) = binding {
        // Reject invocation/configuration drift before any allocation or spawn.
        if capture(db, &binding.runtime)?.as_ref() != Some(binding) {
            bail!("managed lifetime changed before admission");
        }
        db.execute_batch("CREATE TABLE IF NOT EXISTS snapshot_managed_ownership(run TEXT PRIMARY KEY, binding TEXT NOT NULL);
            CREATE TRIGGER IF NOT EXISTS managed_ownership_no_update BEFORE UPDATE ON snapshot_managed_ownership BEGIN SELECT RAISE(ABORT,'immutable managed ownership'); END;
            CREATE TRIGGER IF NOT EXISTS managed_ownership_no_delete BEFORE DELETE ON snapshot_managed_ownership BEGIN SELECT RAISE(ABORT,'immutable managed ownership'); END;")?;
        db.execute(
            "INSERT INTO snapshot_managed_ownership(run,binding) VALUES(?1,?2)",
            rusqlite::params![run, serde_json::to_string(binding)?],
        )?;
    }
    Ok(())
}
pub fn load(db: &Connection, run: &str) -> Result<Binding> {
    let text: Option<String> = db
        .query_row(
            "SELECT binding FROM snapshot_managed_ownership WHERE run=?1",
            [run],
            |r| r.get(0),
        )
        .optional()?;
    Ok(serde_json::from_str(&text.context(
        "historical managed ownership absent; reconciliation forbidden",
    )?)?)
}

pub fn stopped(binding: &Binding) -> Result<()> {
    if binding.uid != unsafe { libc::geteuid() } || binding.machine != machine()? {
        bail!("managed machine/UID mismatch");
    }
    let properties = show(&binding.unit)?;
    verify_stopped(binding, &properties, &boot()?, |group| {
        let path = cgroup_path(group)?;
        match fs::metadata(&path) {
            Ok(meta) => Ok(Some((
                meta.dev(),
                meta.ino(),
                fs::read_to_string(path.join("cgroup.events"))?,
            ))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    })
}

fn verify_stopped(
    binding: &Binding,
    properties: &BTreeMap<String, String>,
    boot: &str,
    observe: impl FnOnce(&str) -> Result<Option<(u64, u64, String)>>,
) -> Result<()> {
    // Runtime masking is the explicit operator interlock. Owner/endpoint locks
    // remain held by the caller through receipt commit, preventing admissions.
    if property(properties, "LoadState")? != "masked"
        || property(properties, "UnitFileState")? != "masked-runtime"
        || !matches!(property(properties, "ActiveState")?, "inactive" | "failed")
        || property(properties, "MainPID")? != "0"
    {
        bail!("keeper must be stopped and runtime-masked for reconciliation");
    }
    if binding.boot != boot {
        // Reboot cannot establish worker-cleanup ACK, only terminal revocation.
        return Ok(());
    }
    let invocation = property(properties, "InvocationID")?;
    if !invocation.is_empty() && invocation != binding.invocation {
        bail!("fresh or unknown invocation cannot prove historical teardown");
    }
    match observe(&binding.cgroup)? {
        Some((device, inode, events)) => {
            if invocation.is_empty() {
                bail!("existing cgroup requires its historical invocation identity");
            }
            if device != binding.cgroup_device || inode != binding.cgroup_inode {
                bail!("managed cgroup was replaced");
            }
            if !events.lines().any(|line| line == "populated 0") {
                bail!("managed descendant cgroup is still populated");
            }
        }
        None => {
            if !property(properties, "ControlGroup")?.is_empty() {
                bail!("manager has not confirmed cgroup removal");
            }
        }
    }
    Ok(())
}

pub fn confirm_stop(binding: &Binding) -> Result<()> {
    stopped(binding)?;
    command(&["stop", &binding.unit])?;
    stopped(binding)
}

#[cfg(test)]
mod tests;
