use super::super::*;

pub(crate) fn handle(command: AipkgCommands) -> anyhow::Result<()> {
    match command {
        AipkgCommands::ValidateManifest { manifest_path } => {
            let manifest = load_aipkg_manifest(&manifest_path)?;
            manifest.validate()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema_version": "annunimas.aipkg.validate-result.v1",
                    "generated_at_utc": Utc::now().to_rfc3339(),
                    "manifest_path": manifest_path,
                    "package_id": manifest.package_id,
                    "version": manifest.version,
                    "runtime_profile": manifest.runtime_profile,
                    "status": "valid"
                }))?
            );
        }
        AipkgCommands::Preflight {
            manifest_path,
            runtime_profile,
            out,
        } => {
            let manifest = load_aipkg_manifest(&manifest_path)?;
            manifest.validate()?;
            let receipt = build_aipkg_preflight_receipt(
                &manifest_path,
                &manifest,
                runtime_profile.as_deref(),
            )?;
            if let Some(path) = out {
                let path = expand_home(&path);
                if let Some(parent) = std::path::Path::new(&path).parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, serde_json::to_string_pretty(&receipt)? + "\n")?;
            }
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
    }
    Ok(())
}
