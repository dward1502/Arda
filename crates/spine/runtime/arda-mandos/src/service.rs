// sigil: REPAIR
use crate::{OracleEngine, OracleQuery, Verdict};
use arda_economics::{JouleWorkUnit, PlutusService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const ORACLE_RUNTIME_SCHEMA_VERSION: &str = "arda.mandos.runtime.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRuntimePaths {
    pub home: String,
    pub status_path: String,
    pub verdict_ledger_path: String,
}

#[derive(Clone)]
pub struct OracleService {
    home: PathBuf,
    status_path: PathBuf,
    verdict_ledger_path: PathBuf,
    engine: Arc<Mutex<OracleEngine>>,
}

impl OracleService {
    pub fn from_home(home: impl AsRef<Path>) -> anyhow::Result<Self> {
        let home = home.as_ref().to_path_buf();
        fs::create_dir_all(&home)?;
        Ok(Self {
            status_path: home.join("runtime_status.json"),
            verdict_ledger_path: home.join("verdict_history.jsonl"),
            home,
            engine: Arc::new(Mutex::new(OracleEngine::new())),
        })
    }

    pub fn from_default_or_workspace_fallback() -> anyhow::Result<Self> {
        let home = std::env::var("ARDA_MANDOS_HOME").unwrap_or_else(|_| "data/oracle".into());
        Self::from_home(home)
    }

    pub fn runtime_paths(&self) -> OracleRuntimePaths {
        OracleRuntimePaths {
            home: self.home.to_string_lossy().to_string(),
            status_path: self.status_path.to_string_lossy().to_string(),
            verdict_ledger_path: self.verdict_ledger_path.to_string_lossy().to_string(),
        }
    }

    pub async fn evaluate(&self, query: OracleQuery) -> anyhow::Result<Verdict> {
        let (verdict, is_new) = {
            let mut engine = self.engine.lock().await;
            let history_before = engine.get_history().len();
            let verdict = engine.evaluate(query)?;
            let is_new = engine.get_history().len() > history_before;
            (verdict, is_new)
        };
        if !is_new {
            return Ok(verdict);
        }
        let triad_average = (verdict.gates.aurelius.score
            + verdict.gates.bacon.score
            + verdict.gates.sun_tzu.score)
            / 3.0;
        let work_amount = ((triad_average + verdict.resonance_score) / 2.0).clamp(0.25, 1.0);
        self.emit_work_signal_background(
            "oracle",
            work_amount,
            JouleWorkUnit::Reasoning,
            Some(verdict.query_id.clone()),
        );
        self.emit_relationship_signal_background(
            "oracle",
            &verdict.governance.love_equation_guard,
            verdict.query_id.clone(),
        );
        self.append_verdict(&verdict)?;
        self.persist_snapshot(&self.snapshot().await?)?;
        Ok(verdict)
    }

    pub async fn status(&self) -> anyhow::Result<serde_json::Value> {
        let snapshot = self.snapshot().await?;
        if !self.status_path.exists() {
            self.persist_snapshot(&snapshot)?;
        }
        Ok(snapshot)
    }

    pub fn recent_verdicts(&self, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
        let content = match fs::read_to_string(&self.verdict_ledger_path) {
            Ok(v) => v,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let mut values = Vec::new();
        for line in content.lines().rev() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) {
                redact_excerpt_fields(&mut value);
                values.push(value);
                if values.len() >= limit.max(1) {
                    break;
                }
            }
        }
        values.reverse();
        Ok(values)
    }

    async fn snapshot(&self) -> anyhow::Result<serde_json::Value> {
        let status = self.engine.lock().await.status_snapshot();
        let recent_verdicts = self.recent_verdicts(10)?;
        Ok(json!({
            "schema_version": ORACLE_RUNTIME_SCHEMA_VERSION,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "authority": "oracle_service",
            "paths": self.runtime_paths(),
            "verdict_runtime": status,
            "evidence_plane": {
                "verdict_ledger_entries": recent_verdicts.len(),
                "recent_persisted_verdicts": recent_verdicts
            }
        }))
    }

    fn append_verdict(&self, verdict: &Verdict) -> anyhow::Result<()> {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.verdict_ledger_path)?;
        writeln!(
            file,
            "{}",
            serde_json::to_string(&verdict.redacted_for_export())?
        )?;
        Ok(())
    }

    fn persist_snapshot(&self, snapshot: &serde_json::Value) -> anyhow::Result<()> {
        fs::write(
            &self.status_path,
            serde_json::to_string_pretty(snapshot)? + "\n",
        )?;
        Ok(())
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> anyhow::Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback()?;
        plutus.track_work(agent_id, amount, unit, task_id).await?;
        Ok(())
    }

    async fn record_relationship_signal_async(
        &self,
        to: &str,
        resonance: f64,
        attention: f64,
        reciprocity: f64,
    ) -> anyhow::Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback()?;
        plutus
            .record_relationship("oracle", to, resonance, attention, reciprocity)
            .await?;
        Ok(())
    }

    fn emit_work_signal_background(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) {
        let service = self.clone();
        let agent_id = agent_id.to_string();
        tokio::spawn(async move {
            if let Err(err) = service
                .record_work_signal_async(&agent_id, amount, unit, task_id)
                .await
            {
                tracing::debug!(error = %err, "ORACLE plutus work signal failed");
            }
        });
    }

    fn emit_relationship_signal_background(
        &self,
        from: &str,
        guard: &crate::reasoning::LoveEquationGuard,
        task_id: String,
    ) {
        let service = self.clone();
        let from = from.to_string();
        let resonance = guard.resonance;
        let attention = guard.attention;
        let reciprocity = guard.reciprocity;
        tokio::spawn(async move {
            if let Err(err) = service
                .record_relationship_signal_async(&from, resonance, attention, reciprocity)
                .await
            {
                tracing::debug!(error = %err, task_id = %task_id, "ORACLE plutus relationship signal failed");
            }
        });
    }
}

fn redact_excerpt_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for (field, value) in fields {
                if field == "excerpt" && !value.is_null() {
                    *value = serde_json::Value::String("[REDACTED]".to_string());
                } else {
                    redact_excerpt_fields(value);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_excerpt_fields(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_economics::PlutusService;
    use chrono::{TimeZone, Utc};

    fn query(id: &str, task: &str) -> OracleQuery {
        let mut query = OracleQuery::new(id, task, "prometheus");
        query.context = vec!["test evidence".to_string()];
        query
    }

    #[tokio::test]
    async fn persists_runtime_status_and_verdict_ledger() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");
        let verdict = service
            .evaluate(query(
                "oracle-query-1",
                "Should we deploy this with evidence?",
            ))
            .await
            .expect("evaluate");

        assert!(!verdict.reasoning.is_empty());
        let status = service.status().await.expect("status");
        assert_eq!(status["authority"], "oracle_service");
        assert_eq!(status["verdict_runtime"]["history_total"], 1);
        assert!(temp.path().join("verdict_history.jsonl").exists());
        assert!(temp.path().join("runtime_status.json").exists());
        let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
        let mut total = 0.0;
        for _ in 0..20 {
            total = plutus.status().await.expect("plutus status")["joulework"]["total"]
                .as_f64()
                .unwrap_or(0.0);
            if total > 0.0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(total > 0.0);
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[tokio::test]
    async fn invalid_query_is_rejected_before_persistence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path()).expect("service");

        let error = service
            .evaluate(query("invalid-query", "   "))
            .await
            .expect_err("blank tasks must be rejected");

        assert!(error.to_string().contains("task"));
        assert!(!temp.path().join("verdict_history.jsonl").exists());
    }

    #[tokio::test]
    async fn conflicting_duplicate_query_id_is_rejected_without_second_record() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");

        service
            .evaluate(query("duplicate-query", "review deployment evidence"))
            .await
            .expect("first evaluation");
        let error = service
            .evaluate(query("duplicate-query", "perform a different review"))
            .await
            .expect_err("conflicting duplicate must fail");

        assert!(error.to_string().contains("duplicate-query"));
        let ledger = std::fs::read_to_string(temp.path().join("verdict_history.jsonl"))
            .expect("verdict ledger");
        assert_eq!(ledger.lines().count(), 1);
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[tokio::test]
    async fn identical_retry_reuses_verdict_without_second_record() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");
        let oracle_query = query("retry-query", "review deployment evidence");
        let mut retry_query = oracle_query.clone();
        retry_query.timestamp += chrono::TimeDelta::seconds(1);

        let first = service
            .evaluate(oracle_query.clone())
            .await
            .expect("first evaluation");
        let second = service
            .evaluate(retry_query)
            .await
            .expect("idempotent retry");

        assert_eq!(first.timestamp, second.timestamp);
        let ledger = std::fs::read_to_string(temp.path().join("verdict_history.jsonl"))
            .expect("verdict ledger");
        assert_eq!(ledger.lines().count(), 1);
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[tokio::test]
    async fn verdict_preserves_caller_timestamp() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");
        let caller_timestamp = Utc
            .with_ymd_and_hms(2025, 1, 2, 3, 4, 5)
            .single()
            .expect("fixed timestamp");
        let mut oracle_query = query("timestamp-query", "review deployment evidence");
        oracle_query.timestamp = caller_timestamp;

        let verdict = service.evaluate(oracle_query).await.expect("evaluation");

        assert_eq!(verdict.timestamp, caller_timestamp);
        assert_eq!(verdict.query_timestamp, caller_timestamp);
        assert!(verdict.evaluated_at > caller_timestamp);
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[tokio::test]
    async fn persisted_exports_redact_sensitive_excerpts_but_retain_provenance() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");
        let mut oracle_query = query("redaction-query", "review deployment evidence");
        oracle_query.context.clear();
        oracle_query.evidence = vec![crate::evidence::EvidenceRef::supplied(
            "sensitive-report",
            "vault://reports/secret",
            oracle_query.timestamp,
            "operator transplant details",
        )
        .with_sensitive_excerpt(false)];

        service.evaluate(oracle_query).await.expect("evaluation");

        let ledger = std::fs::read_to_string(temp.path().join("verdict_history.jsonl"))
            .expect("verdict ledger");
        assert!(!ledger.contains("operator transplant details"));
        assert!(ledger.contains("[REDACTED]"));
        assert!(ledger.contains("sensitive-report"));
        assert!(ledger.contains("sha256:"));
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[test]
    fn recent_verdict_exports_redact_legacy_excerpt_fields() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path()).expect("service");
        std::fs::write(
            temp.path().join("verdict_history.jsonl"),
            serde_json::json!({
                "query_id": "legacy-sensitive",
                "reasoning": [{
                    "evidence": [{
                        "source_id": "legacy-report",
                        "digest": "sha256:retained",
                        "excerpt": "legacy private excerpt"
                    }]
                }]
            })
            .to_string()
                + "\n",
        )
        .expect("legacy ledger fixture");

        let recent = service.recent_verdicts(10).expect("recent verdicts");

        assert_eq!(
            recent[0]["reasoning"][0]["evidence"][0]["excerpt"],
            "[REDACTED]"
        );
        assert_eq!(
            recent[0]["reasoning"][0]["evidence"][0]["digest"],
            "sha256:retained"
        );
        assert!(!recent[0].to_string().contains("legacy private excerpt"));
    }
}
