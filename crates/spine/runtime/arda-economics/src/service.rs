// sigil: REPAIR
use crate::{
    CostModelConfig, EconomicsEngine, JouleWorkTracker, JouleWorkUnit, LoveEquation, PlutusLedger,
};
use arda_core::{JouleWorkMeasurementSource, Task};
use arda_governance::{
    bacon_lite_validate, calculate_resonance_basic, triad_validate, BaconLiteResult,
    ResonanceScore, TriadResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;

pub const PLUTUS_RUNTIME_SCHEMA_VERSION: &str = "arda.plutus.runtime.v1";

fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlutusRuntimePaths {
    pub home: String,
    pub status_path: String,
}

#[derive(Clone)]
pub struct PlutusService {
    home: PathBuf,
    status_path: PathBuf,
    persist_lock: Arc<Mutex<()>>,
    economics: Arc<Mutex<EconomicsEngine>>,
    tracker: JouleWorkTracker,
    ledger: Arc<Mutex<PlutusLedger>>,
    love: Arc<Mutex<LoveEquation>>,
    governance_history: Arc<Mutex<Vec<PlutusGovernanceRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlutusGovernanceRecord {
    pub action: String,
    pub subject: String,
    pub triad: TriadResult,
    pub bacon_lite: BaconLiteResult,
    pub resonance: ResonanceScore,
    pub recorded_at_utc: String,
}

impl PlutusService {
    pub fn from_home(home: impl AsRef<Path>) -> anyhow::Result<Self> {
        let home = home.as_ref().to_path_buf();
        fs::create_dir_all(&home)?;
        Ok(Self {
            status_path: home.join("runtime_status.json"),
            home,
            persist_lock: global_persist_lock(),
            economics: Arc::new(Mutex::new(EconomicsEngine::new())),
            tracker: JouleWorkTracker::new(),
            ledger: Arc::new(Mutex::new(PlutusLedger::new())),
            love: Arc::new(Mutex::new(LoveEquation::new())),
            governance_history: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn from_default_or_workspace_fallback() -> anyhow::Result<Self> {
        let home = std::env::var("ARDA_PLUTUS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| arda_root().join("data/plutus"));
        Self::from_home(home)
    }

    pub fn runtime_paths(&self) -> PlutusRuntimePaths {
        PlutusRuntimePaths {
            home: self.home.to_string_lossy().into_owned(),
            status_path: self.status_path.to_string_lossy().into_owned(),
        }
    }

    pub async fn register_model(&self, config: CostModelConfig) -> anyhow::Result<()> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        let governance = governance_record_for(
            "register_model",
            &config.provider,
            format!("register cost model for provider {} because runtime budget accounting depends on it", config.provider),
            "economics",
            0.5,
            0.45,
        );
        self.economics.lock().await.register_model(config);
        self.governance_history.lock().await.push(governance);
        self.persist_snapshot_inner(&self.snapshot().await?)?;
        Ok(())
    }

    pub async fn record_spend(
        &self,
        provider: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> anyhow::Result<Option<f64>> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        let mut economics = self.economics.lock().await;
        let cost = economics.calculate_cost(provider, input_tokens, output_tokens);
        if let Some(amount) = cost {
            economics.record_spend(amount);
            let governance = governance_record_for(
                "record_spend",
                provider,
                format!("record spend for provider {} because {} input and {} output tokens were consumed", provider, input_tokens, output_tokens),
                "economics",
                amount.max(0.25),
                amount,
            );
            self.governance_history.lock().await.push(governance);
        }
        drop(economics);
        self.persist_snapshot_inner(&self.snapshot().await?)?;
        Ok(cost)
    }

    pub async fn track_work(
        &self,
        agent: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> anyhow::Result<()> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        self.tracker
            .track_work_with_source(
                agent,
                amount,
                unit,
                task_id,
                JouleWorkMeasurementSource::OperatorEstimate,
                0.5,
            )
            .await;
        let governance = governance_record_for(
            "track_work",
            agent,
            format!(
                "track joulework for agent {} because {:?} work was recorded",
                agent, unit
            ),
            "monitor",
            amount.max(0.25),
            amount,
        );
        self.governance_history.lock().await.push(governance);
        self.persist_snapshot_inner(&self.snapshot().await?)?;
        Ok(())
    }

    pub async fn credit(&self, account: &str, amount: f64) -> anyhow::Result<()> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        self.ledger.lock().await.credit(account, amount);
        let governance = governance_record_for(
            "credit",
            account,
            format!(
                "credit account {} because balance changed by {:.3}",
                account, amount
            ),
            "dispatch",
            amount.max(0.25),
            amount,
        );
        self.governance_history.lock().await.push(governance);
        self.persist_snapshot_inner(&self.snapshot().await?)?;
        Ok(())
    }

    pub async fn record_relationship(
        &self,
        from: &str,
        to: &str,
        trust: f64,
        reciprocity: f64,
        longevity: f64,
    ) -> anyhow::Result<f64> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        let mut love = self.love.lock().await;
        let score = love.calculate(from, to, trust, reciprocity, longevity);
        love.record_relationship(from, to, score);
        drop(love);
        let governance = governance_record_for(
            "record_relationship",
            &format!("{from}:{to}"),
            format!("record love_equation relationship between {} and {} because repeated collaboration needs continuity", from, to),
            "query",
            score.max(0.25),
            score.max(0.25),
        );
        self.governance_history.lock().await.push(governance);
        self.persist_snapshot_inner(&self.snapshot().await?)?;
        Ok(score)
    }

    pub async fn status(&self) -> anyhow::Result<serde_json::Value> {
        let _persist_guard = self.persist_lock.lock().await;
        self.load_persisted_snapshot_into_memory_inner().await?;
        let snapshot = self.snapshot().await?;
        if !self.status_path.exists() {
            self.persist_snapshot_inner(&snapshot)?;
        }
        Ok(snapshot)
    }

    async fn snapshot(&self) -> anyhow::Result<serde_json::Value> {
        let economics = self.economics.lock().await.status_snapshot();
        let ledger = self.ledger.lock().await.snapshot();
        let love = self.love.lock().await.snapshot(10);
        let joulework = self.tracker.status_snapshot().await;
        let governance = self.governance_snapshot().await;
        Ok(json!({
            "schema_version": PLUTUS_RUNTIME_SCHEMA_VERSION,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "authority": "plutus_service",
            "paths": self.runtime_paths(),
            "economics": economics,
            "joulework": joulework,
            "ledger": ledger,
            "love_equation": love,
            "governance": governance,
        }))
    }

    fn persist_snapshot_inner(&self, snapshot: &serde_json::Value) -> anyhow::Result<()> {
        let payload = serde_json::to_string_pretty(snapshot)? + "\n";
        let tmp_path = self.status_path.with_extension("json.tmp");
        fs::write(&tmp_path, payload)?;
        fs::rename(tmp_path, &self.status_path)?;
        Ok(())
    }

    async fn load_persisted_snapshot_into_memory_inner(&self) -> anyhow::Result<()> {
        if !self.status_path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(&self.status_path)?;
        let snapshot: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    path = %self.status_path.display(),
                    "PLUTUS ignored invalid persisted snapshot and will rebuild from memory"
                );
                return Ok(());
            }
        };
        if let Some(economics) = snapshot.get("economics") {
            self.economics.lock().await.restore_from_snapshot(economics);
        }
        if let Some(ledger) = snapshot.get("ledger") {
            self.ledger.lock().await.restore_from_snapshot(ledger);
        }
        if let Some(love) = snapshot.get("love_equation") {
            self.love.lock().await.restore_from_snapshot(love);
        }
        if let Some(joulework) = snapshot.get("joulework") {
            self.tracker.restore_from_snapshot(joulework).await;
        }
        if let Some(governance) = snapshot
            .get("governance")
            .and_then(|v| v.get("recent_records"))
            .and_then(|v| v.as_array())
        {
            let mut history = self.governance_history.lock().await;
            history.clear();
            for row in governance {
                if let Ok(record) = serde_json::from_value::<PlutusGovernanceRecord>(row.clone()) {
                    history.push(record);
                }
            }
        }
        Ok(())
    }

    async fn governance_snapshot(&self) -> serde_json::Value {
        let history = self.governance_history.lock().await;
        let recent = history.iter().rev().take(25).cloned().collect::<Vec<_>>();
        let triad_passed_total = history.iter().filter(|row| row.triad.passed).count();
        let bacon_lite_passed_total = history.iter().filter(|row| row.bacon_lite.passed).count();
        json!({
            "records_total": history.len(),
            "triad_passed_total": triad_passed_total,
            "bacon_lite_passed_total": bacon_lite_passed_total,
            "recent_records": recent,
        })
    }
}

fn governance_record_for(
    action: &str,
    subject: &str,
    description: String,
    task_type: &str,
    estimated: f64,
    actual: f64,
) -> PlutusGovernanceRecord {
    let mut task = Task::new(description, task_type);
    task.assign("plutus");
    task.execution_started_at = Some(task.created_at + chrono::TimeDelta::seconds(1));
    task.updated_at = task.created_at + chrono::TimeDelta::seconds(2);
    task.joule_cost_estimated = estimated.max(0.25);
    task.joule_cost_actual = actual.max(0.25);
    task.clarifications_requested = 0;
    task.clarifications_resolved = 1;
    task.status = arda_core::task::TaskStatus::Complete;
    PlutusGovernanceRecord {
        action: action.to_owned(),
        subject: subject.to_owned(),
        triad: triad_validate(&task, None),
        bacon_lite: bacon_lite_validate(&task),
        resonance: calculate_resonance_basic(&task),
        recorded_at_utc: chrono::Utc::now().to_rfc3339(),
    }
}

fn global_persist_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persists_runtime_status_with_joulework_and_ledger() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = PlutusService::from_home(temp.path()).expect("service");
        service
            .register_model(CostModelConfig {
                provider: "openai".to_string(),
                input_rate: 0.001,
                output_rate: 0.002,
                batch_size: 1000,
            })
            .await
            .expect("register");
        service
            .record_spend("openai", 100, 50)
            .await
            .expect("spend");
        service
            .track_work(
                "athena",
                2.0,
                JouleWorkUnit::Reasoning,
                Some("task1".to_string()),
            )
            .await
            .expect("track");
        service.credit("athena", 4.0).await.expect("credit");
        service
            .record_relationship("athena", "hermes", 0.8, 0.7, 0.9)
            .await
            .expect("love");

        let status = service.status().await.expect("status");
        assert_eq!(status["authority"], "plutus_service");
        assert_eq!(status["economics"]["providers"][0], "openai");
        assert_eq!(status["ledger"]["accounts_total"], 1);
        assert!(
            status["governance"]["records_total"]
                .as_u64()
                .unwrap_or_default()
                >= 4
        );
        assert!(
            status["governance"]["triad_passed_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );
        assert!(temp.path().join("runtime_status.json").exists());
    }
}
