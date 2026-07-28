use anyhow::Result;
use arda_mandos::{OracleQuery, OracleService};

fn query(id: &str) -> OracleQuery {
    let mut query = OracleQuery::new(
        id,
        "Should target-local audit coverage rely on isolated runtime state?",
        "oracle-target-local",
    );
    query.context = vec!["target-local evidence".to_string()];
    query
}

#[tokio::test]
async fn status_uses_target_local_home_without_workspace_state() -> Result<()> {
    let home = tempfile::tempdir()?;
    let service = OracleService::from_home(home.path()).await?;

    let paths = service.runtime_paths();
    assert_eq!(paths.home, home.path().to_string_lossy());
    assert!(paths.status_path.ends_with("runtime_status.json"));
    assert!(paths.verdict_ledger_path.ends_with("verdict_history.jsonl"));

    let status = service.status().await?;
    assert_eq!(status["schema_version"], "arda.mandos.runtime.v1");
    assert_eq!(status["authority"], "oracle_service");
    assert_eq!(status["verdict_runtime"]["history_total"], 0);
    assert!(home.path().join("runtime_status.json").exists());
    assert!(!home.path().join("verdict_history.jsonl").exists());

    Ok(())
}

#[tokio::test]
async fn verdict_persists_and_reloads_from_target_local_home() -> Result<()> {
    let home = tempfile::tempdir()?;
    let service = OracleService::from_home(home.path())
        .await?
        .with_plutus_home(home.path().join("plutus"));
    let verdict = service.evaluate(query("oracle-target-local-1")).await?;
    assert_eq!(verdict.query_id, "oracle-target-local-1");

    let ledger_path = home.path().join("verdict_history.jsonl");
    assert!(ledger_path.exists());
    assert!(std::fs::read_to_string(&ledger_path)?.contains("oracle-target-local-1"));

    let restarted = OracleService::from_home(home.path()).await?;
    let recent = restarted.recent_verdicts(10)?;
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0]["query_id"], "oracle-target-local-1");

    let status = restarted.status().await?;
    assert_eq!(status["evidence_plane"]["verdict_ledger_entries"], 1);
    assert_eq!(
        status["evidence_plane"]["recent_persisted_verdicts"][0]["query_id"],
        "oracle-target-local-1"
    );

    Ok(())
}
