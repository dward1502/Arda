// sigil: REPAIR
use crate::{OracleEngine, OracleQuery, Verdict};
use annunimas_plutus::{JouleWorkUnit, PlutusService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const ORACLE_RUNTIME_SCHEMA_VERSION: &str = "annunimas.oracle.runtime.v1";

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
        let home = std::env::var("ANNUNIMAS_ORACLE_HOME").unwrap_or_else(|_| "data/oracle".into());
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
        let verdict = self.engine.lock().await.evaluate(query);
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
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
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
        writeln!(file, "{}", serde_json::to_string(verdict)?)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use annunimas_plutus::PlutusService;
    use chrono::Utc;

    #[tokio::test]
    async fn persists_runtime_status_and_verdict_ledger() {
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ANNUNIMAS_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(temp.path()).expect("service");
        let verdict = service
            .evaluate(OracleQuery {
                id: "oracle-query-1".to_string(),
                task: "Should we deploy this with evidence?".to_string(),
                context: vec!["test evidence".to_string()],
                requester: "prometheus".to_string(),
                timestamp: Utc::now(),
            })
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
        std::env::remove_var("ANNUNIMAS_PLUTUS_HOME");
    }
}
