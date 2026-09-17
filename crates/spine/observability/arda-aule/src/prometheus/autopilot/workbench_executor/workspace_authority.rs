//! Process-local retained authority. No serialized workspace bypass exists.
use super::*;
#[cfg(test)]
mod tests;

pub trait ExplicitWorkspaceAuthorization: Send + Sync {
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()>;
    /// Opaque Engine-owned lease, sent only at provider dispatch. It is not part
    /// of the serialized work item and Aule does not interpret its schema.
    fn provider_lease(&self, _item: &ExplicitWorkbenchWorkItem) -> Result<Value> {
        anyhow::bail!("retained provider dispatch requires an Engine lease")
    }
    /// Engine-owned saved admission; never deserialize this from a work item.
    fn recovery_window(
        &self,
        _item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Option<ExplicitRecoveryWindow>> {
        Ok(None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplicitRecoveryWindow {
    pub event_id: String,
    pub activated_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

impl ExplicitRecoveryWindow {
    pub(super) fn validate(&self) -> Result<()> {
        let now = u64::try_from(Utc::now().timestamp_millis())?;
        anyhow::ensure!(
            !self.event_id.trim().is_empty()
                && self
                    .expires_at_unix_ms
                    .checked_sub(self.activated_at_unix_ms)
                    == Some(30 * 60_000)
                && now >= self.activated_at_unix_ms
                && now < self.expires_at_unix_ms,
            "explicit recovery window is invalid or expired"
        );
        Ok(())
    }
}
impl<F> ExplicitWorkspaceAuthorization for F
where
    F: Fn(&ExplicitWorkbenchWorkItem) -> Result<()> + Send + Sync,
{
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
        self(item)
    }
}

pub(super) fn reauthorize(
    authority: Option<&dyn ExplicitWorkspaceAuthorization>,
    item: &ExplicitWorkbenchWorkItem,
) -> Result<()> {
    if let Some(authority) = authority {
        authority.authorize(item)?;
        if let Some(window) = authority.recovery_window(item)? {
            window.validate()?;
        }
    }
    Ok(())
}
pub(super) fn provider_body(
    authority: Option<&dyn ExplicitWorkspaceAuthorization>,
    item: &ExplicitWorkbenchWorkItem,
    mut body: Value,
) -> Result<Value> {
    reauthorize(authority, item)?;
    if let Some(authority) = authority {
        body["expected_retained_lease"] = authority.provider_lease(item)?;
        if let Some(window) = authority.recovery_window(item)? {
            window.validate()?;
            body["recovery_event_id"] = json!(window.event_id);
        }
    }
    Ok(body)
}

#[derive(Clone, Copy)]
pub(super) enum Validation {
    Fresh,
    Retained,
    Reconcile,
}

pub(super) fn bound_workspace(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    mode: Validation,
) -> Result<()> {
    // Receipt reconciliation authorizes no new execution. Its caller checks the
    // retained item/graph/receipt bindings; current attachment state is irrelevant.
    if matches!(mode, Validation::Reconcile) {
        return workspace(root, &item.workspace_root, mode);
    }
    let canonical_root = root.canonicalize().context("resolve Arda authority root")?;
    let path = &item.workspace_root;
    if path.starts_with(&canonical_root) {
        return workspace(root, path, mode);
    }
    // External roots require an exact registered authority, not a task-supplied escape.
    let registry: ExecutionProjectRegistry =
        serde_json::from_slice(&std::fs::read(root.join("data/workbench/projects.json"))?)?;
    if registry.schema_version != "arda.workbench.project-registry.v1" {
        bail!("unsupported project registry version");
    }
    let matches: Vec<_> = registry
        .projects
        .iter()
        .filter(|entry| entry.contract.identity.project_id.to_string() == item.project_id)
        .collect();
    if matches.len() != 1 {
        bail!("external workspace requires one attached project");
    }
    let contract = &matches[0].contract;
    contract.validate()?;
    let digest = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(contract)?));
    if digest != item.project_contract_digest
        || Path::new(contract.workspace.root.as_str()) != path
        || !path.is_absolute()
        || path == Path::new("/")
    {
        bail!("external workspace differs from approved project authority");
    }
    match path.canonicalize() {
        Ok(resolved) if resolved == *path && resolved.is_dir() => Ok(()),
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && matches!(mode, Validation::Retained) =>
        {
            let ancestor = path
                .ancestors()
                .skip(1)
                .find(|parent| parent.exists())
                .ok_or_else(|| anyhow!("external workspace has no surviving ancestor"))?;
            if ancestor.canonicalize()? != ancestor {
                bail!("external retained workspace ancestor was rebound");
            }
            Ok(())
        }
        _ => bail!("external workspace is unavailable or was rebound"),
    }
}

pub(super) fn workspace(root: &Path, path: &Path, mode: Validation) -> Result<()> {
    if !path.is_absolute()
        || path.as_os_str()
            != path
                .components()
                .collect::<std::path::PathBuf>()
                .as_os_str()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(anyhow!(
            "explicit workspace must have a canonical absolute spelling"
        ));
    }
    if matches!(mode, Validation::Reconcile) {
        return Ok(());
    }
    let root = std::fs::canonicalize(root).context("resolve Arda root")?;
    match std::fs::canonicalize(path) {
        Ok(resolved) => {
            if !resolved.is_dir() || !resolved.starts_with(&root) {
                return Err(anyhow!(
                    "explicit workspace must be a directory within the Arda root"
                ));
            }
        }
        Err(error)
            if matches!(mode, Validation::Retained)
                && error.kind() == std::io::ErrorKind::NotFound =>
        {
            let mut ancestor = path;
            loop {
                match std::fs::symlink_metadata(ancestor) {
                    Ok(metadata) => {
                        if !metadata.is_dir()
                            || metadata.file_type().is_symlink()
                            || !std::fs::canonicalize(ancestor)?.starts_with(&root)
                        {
                            return Err(anyhow!(
                                "retained workspace has an invalid surviving parent"
                            ));
                        }
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        ancestor = ancestor
                            .parent()
                            .ok_or_else(|| anyhow!("missing workspace has no surviving parent"))?;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Err(error) => return Err(error).context("resolve explicit workspace root"),
    }
    Ok(())
}

pub(super) fn safe_run_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 1024
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
}

pub(super) fn run(item: &ExplicitWorkbenchWorkItem, value: &Value) -> Result<()> {
    let graph = &value["graph"];
    if graph["run_id"].as_str() != Some(item.run_id.as_str())
        || graph["objective_id"].as_str() != Some(item.leaf_id.as_str())
        || graph["provenance"]["project_contract_digest"].as_str()
            != Some(item.project_contract_digest.as_str())
    {
        return Err(anyhow!("explicit run identity differs from the bound item"));
    }
    let approval = item.approval_envelope["approval"]["approval_id"]
        .as_str()
        .ok_or_else(|| anyhow!("approval id missing"))?;
    let parents = graph["provenance"]["parent_receipts"]
        .as_array()
        .ok_or_else(|| anyhow!("run omitted provenance receipts"))?;
    if parents.len() != 2
        || !parents.iter().any(|p| p.as_str() == Some(approval))
        || !parents
            .iter()
            .any(|p| p.as_str() == Some(item.objective_plan_receipt.as_str()))
    {
        return Err(anyhow!(
            "explicit run provenance differs from the bound item"
        ));
    }
    let nodes = graph["nodes"]
        .as_array()
        .ok_or_else(|| anyhow!("run omitted nodes"))?;
    if item.read_only {
        let expected = explicit_run_graph(item, approval);
        for id in ["execute", "verify", "review"] {
            let actual = nodes
                .iter()
                .find(|node| node["id"] == id)
                .ok_or_else(|| anyhow!("read-only run missing {id} node"))?;
            let bound = expected["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["id"] == id)
                .unwrap();
            if actual["kind"] != bound["kind"]
                || actual["authority"] != bound["authority"]
                || actual["worker"]["role"] != bound["worker"]["role"]
                || actual["worker"]["allowed_toolsets"] != json!(["file"])
            {
                bail!("read-only run {id} node exceeds bound inspection scope");
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for node in nodes {
        let id = node["id"]
            .as_str()
            .ok_or_else(|| anyhow!("run node omitted identity"))?;
        let kind = node["kind"]
            .as_str()
            .ok_or_else(|| anyhow!("run node omitted kind"))?;
        let valid = match id {
            "execute" => matches!(kind, "execute" | "inspect"),
            "plan" | "approval" | "verify" | "review" | "close" => kind == id,
            _ => false,
        };
        if !valid || !seen.insert(id) {
            return Err(anyhow!("explicit run has invalid or duplicate stages"));
        }
    }
    if ["approval", "execute", "verify", "review", "close"]
        .iter()
        .any(|stage| !seen.contains(stage))
    {
        return Err(anyhow!("explicit run omitted a required stage"));
    }
    Ok(())
}
