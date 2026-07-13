use annunimas_human::HumanKnowledge;
use anyhow::Result;

fn write_note(path: &std::path::Path, title: &str, body: &str) -> Result<()> {
    std::fs::write(
        path,
        format!(
            r#"---
title: {title}
created: 2026-05-25
updated: 2026-05-25
tags: [target-local]
status: active
confidence: 80
source: null
related: []
references: []
---

{body}
"#
        ),
    )?;
    Ok(())
}

#[test]
fn scan_vault_resolves_existing_wikilinks_without_following_missing_notes() -> Result<()> {
    let vault = tempfile::tempdir()?;
    let main_path = vault.path().join("Main.md");
    let known_path = vault.path().join("Known.md");

    write_note(
        &main_path,
        "Main",
        "This human vault note links to [[Known]] and [[Missing]].",
    )?;
    write_note(&known_path, "Known", "Known supporting note.")?;

    let notes = HumanKnowledge::new(vault.path())?.scan_vault()?;
    let main = notes
        .iter()
        .find(|note| note.frontmatter.title == "Main")
        .ok_or_else(|| anyhow::anyhow!("Main note was not scanned"))?;

    assert_eq!(main.frontmatter.tags, vec!["target-local"]);
    assert_eq!(main.backlinks, vec![known_path]);

    Ok(())
}

#[test]
fn new_rejects_missing_vault_path() -> Result<()> {
    let vault = tempfile::tempdir()?;
    let missing = vault.path().join("missing-vault");

    let error = HumanKnowledge::new(&missing)
        .expect_err("missing vault path should be rejected")
        .to_string();

    assert!(error.contains("Human vault path does not exist"));
    Ok(())
}
