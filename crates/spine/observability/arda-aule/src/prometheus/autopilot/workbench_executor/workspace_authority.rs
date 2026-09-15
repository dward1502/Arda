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
    }
    Ok(body)
}

#[derive(Clone, Copy)]
pub(super) enum Validation {
    Fresh,
    Retained,
    Reconcile,
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
