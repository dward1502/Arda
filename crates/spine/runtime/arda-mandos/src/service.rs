// sigil: REPAIR
use crate::{OracleEngine, OracleQuery, Verdict};
use arda_economics::{JouleWorkUnit, PlutusService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;

pub const ORACLE_RUNTIME_SCHEMA_VERSION: &str = "arda.mandos.runtime.v1";
pub const VERDICT_RECORD_SCHEMA_VERSION: &str = "arda.mandos.verdict-record.v1";

#[derive(Debug, Clone, Serialize)]
pub struct VerdictRecord {
    pub schema_version: String,
    pub sequence: u64,
    pub request_digest: String,
    pub verdict_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_record_digest: Option<String>,
    pub record_digest: String,
    #[serde(flatten)]
    pub payload: serde_json::Map<String, serde_json::Value>,
}

impl VerdictRecord {
    fn new(
        sequence: u64,
        request_digest: String,
        previous_record_digest: Option<String>,
        verdict: Verdict,
    ) -> anyhow::Result<Self> {
        let payload = match serde_json::to_value(verdict)? {
            serde_json::Value::Object(payload) => payload,
            _ => anyhow::bail!("verdict payload must serialize as a JSON object"),
        };
        let verdict_digest = sha256_json(&payload)?;
        let record_digest = sha256_json(&json!({
            "schema_version": VERDICT_RECORD_SCHEMA_VERSION,
            "sequence": sequence,
            "request_digest": request_digest,
            "verdict_digest": verdict_digest,
            "previous_record_digest": previous_record_digest,
        }))?;
        Ok(Self {
            schema_version: VERDICT_RECORD_SCHEMA_VERSION.to_string(),
            sequence,
            request_digest,
            verdict_digest,
            previous_record_digest,
            record_digest,
            payload,
        })
    }

    fn verdict(&self) -> anyhow::Result<Verdict> {
        Ok(serde_json::from_value(serde_json::Value::Object(
            self.payload.clone(),
        ))?)
    }

    fn from_value(value: serde_json::Value) -> anyhow::Result<Self> {
        let mut fields = value
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("verdict record must be a JSON object"))?;
        let schema_version = take_string(&mut fields, "schema_version")?;
        let sequence = fields
            .remove("sequence")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| anyhow::anyhow!("missing or invalid sequence"))?;
        let request_digest = take_string(&mut fields, "request_digest")?;
        let verdict_digest = take_string(&mut fields, "verdict_digest")?;
        let previous_record_digest = match fields.remove("previous_record_digest") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(value)) => Some(value),
            Some(_) => anyhow::bail!("invalid previous_record_digest"),
        };
        let record_digest = take_string(&mut fields, "record_digest")?;
        Ok(Self {
            schema_version,
            sequence,
            request_digest,
            verdict_digest,
            previous_record_digest,
            record_digest,
            payload: fields,
        })
    }

    fn verify(&self, expected_sequence: u64, previous_digest: Option<&str>) -> anyhow::Result<()> {
        if self.schema_version != VERDICT_RECORD_SCHEMA_VERSION {
            anyhow::bail!("unsupported verdict record schema {}", self.schema_version);
        }
        if self.sequence != expected_sequence {
            anyhow::bail!(
                "verdict record sequence gap: expected {}, found {}",
                expected_sequence,
                self.sequence
            );
        }
        if self.previous_record_digest.as_deref() != previous_digest {
            anyhow::bail!(
                "verdict record digest chain mismatch at sequence {}",
                self.sequence
            );
        }
        if self.verdict_digest != sha256_json(&self.payload)? {
            anyhow::bail!("verdict digest mismatch at sequence {}", self.sequence);
        }
        let expected = sha256_json(&json!({
            "schema_version": self.schema_version,
            "sequence": self.sequence,
            "request_digest": self.request_digest,
            "verdict_digest": self.verdict_digest,
            "previous_record_digest": self.previous_record_digest,
        }))?;
        if self.record_digest != expected {
            anyhow::bail!("record digest mismatch at sequence {}", self.sequence);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for VerdictRecord {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_value(value).map_err(serde::de::Error::custom)
    }
}

fn take_string(
    fields: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> anyhow::Result<String> {
    fields
        .remove(name)
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("missing or invalid {name}"))
}

#[derive(Debug, Default)]
struct VerdictLedgerState {
    next_sequence: u64,
    last_record_digest: Option<String>,
    degraded_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LedgerVerificationReport {
    pub schema_version: &'static str,
    pub valid: bool,
    pub valid_records: usize,
    pub legacy_records: usize,
    pub next_sequence: u64,
    pub chain_head: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct TelemetryDeliveryState {
    permits: Arc<Semaphore>,
    tasks: StdMutex<Vec<JoinHandle<()>>>,
    pending: AtomicU64,
    delivered: AtomicU64,
    failed: AtomicU64,
    saturated: AtomicU64,
}

impl Default for TelemetryDeliveryState {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(8)),
            tasks: StdMutex::new(Vec::new()),
            pending: AtomicU64::new(0),
            delivered: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            saturated: AtomicU64::new(0),
        }
    }
}

impl TelemetryDeliveryState {
    fn snapshot(&self) -> serde_json::Value {
        json!({
            "policy": "best_effort_bounded",
            "capacity": 8,
            "pending": self.pending.load(Ordering::Relaxed),
            "delivered": self.delivered.load(Ordering::Relaxed),
            "failed": self.failed.load(Ordering::Relaxed),
            "saturated": self.saturated.load(Ordering::Relaxed),
        })
    }

    fn retain_running_tasks(&self) {
        self.tasks
            .lock()
            .expect("telemetry task mutex poisoned")
            .retain(|task| !task.is_finished());
    }
}

fn sha256_json(value: &impl Serialize) -> anyhow::Result<String> {
    let encoded = serde_json::to_vec(value)?;
    // Hash the representation produced after one JSON parse/serialize cycle. This is the
    // representation that actually survives JSONL persistence; hashing a directly serialized
    // Rust float can otherwise differ from hashing the semantically identical parsed number.
    let persisted: serde_json::Value = serde_json::from_slice(&encoded)?;
    let canonical = serde_json::to_vec(&persisted)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    let result = (|| -> anyhow::Result<()> {
        let mut temp = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        temp.write_all(bytes)?;
        temp.sync_all()?;
        drop(temp);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn verify_ledger_bytes(bytes: &[u8]) -> anyhow::Result<LedgerVerificationReport> {
    let content = std::str::from_utf8(bytes)?;
    let mut valid_records = 0usize;
    let mut legacy_records = 0usize;
    let mut expected_sequence = 1u64;
    let mut chain_head = None;
    let mut errors = Vec::new();
    let terminated = content.is_empty() || content.ends_with('\n');
    let mut lines = content.lines().collect::<Vec<_>>();
    if !terminated {
        let line_number = lines.len();
        lines.pop();
        errors.push(format!(
            "unterminated trailing record at line {line_number}"
        ));
    }

    for (index, line) in lines.into_iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line_number = index + 1;
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("malformed JSON at line {line_number}: {error}"));
                break;
            }
        };
        if value.get("schema_version").is_none() {
            if serde_json::from_value::<Verdict>(value).is_err() {
                errors.push(format!("invalid legacy verdict at line {line_number}"));
                break;
            }
            legacy_records += 1;
            valid_records += 1;
            errors.push(format!("legacy unchained verdict at line {line_number}"));
            continue;
        }
        let record = match VerdictRecord::from_value(value) {
            Ok(record) => record,
            Err(error) => {
                errors.push(format!("invalid record at line {line_number}: {error}"));
                break;
            }
        };
        if let Err(error) = record.verify(expected_sequence, chain_head.as_deref()) {
            errors.push(format!("invalid record at line {line_number}: {error}"));
            break;
        }
        if let Err(error) = record.verdict() {
            errors.push(format!("invalid verdict at line {line_number}: {error}"));
            break;
        }
        expected_sequence = expected_sequence.saturating_add(1);
        chain_head = Some(record.record_digest);
        valid_records += 1;
    }

    Ok(LedgerVerificationReport {
        schema_version: VERDICT_RECORD_SCHEMA_VERSION,
        valid: errors.is_empty(),
        valid_records,
        legacy_records,
        next_sequence: expected_sequence,
        chain_head,
        errors,
    })
}

fn read_ledger(path: &Path) -> anyhow::Result<Vec<u8>> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

fn verify_ledger_file(path: &Path) -> anyhow::Result<LedgerVerificationReport> {
    verify_ledger_bytes(&read_ledger(path)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRuntimePaths {
    pub home: String,
    pub status_path: String,
    pub verdict_ledger_path: String,
}

impl OracleRuntimePaths {
    pub fn from_home(home: impl AsRef<Path>) -> Self {
        let home = home.as_ref().to_path_buf();
        Self {
            home: home.to_string_lossy().to_string(),
            status_path: home
                .join("runtime_status.json")
                .to_string_lossy()
                .to_string(),
            verdict_ledger_path: home
                .join("verdict_history.jsonl")
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[derive(Clone)]
pub struct OracleService {
    home: PathBuf,
    status_path: PathBuf,
    verdict_ledger_path: PathBuf,
    engine: Arc<Mutex<OracleEngine>>,
    evaluation_lock: Arc<Mutex<()>>,
    ledger_state: Arc<Mutex<VerdictLedgerState>>,
    telemetry: Arc<TelemetryDeliveryState>,
    plutus_home: Option<PathBuf>,
}

impl OracleService {
    pub async fn from_home(home: impl AsRef<Path>) -> anyhow::Result<Self> {
        let home = home.as_ref().to_path_buf();
        fs::create_dir_all(&home)?;
        let service = Self {
            status_path: home.join("runtime_status.json"),
            verdict_ledger_path: home.join("verdict_history.jsonl"),
            home: home.clone(),
            engine: Arc::new(Mutex::new(OracleEngine::new())),
            evaluation_lock: Arc::new(Mutex::new(())),
            ledger_state: Arc::new(Mutex::new(VerdictLedgerState {
                next_sequence: 1,
                ..VerdictLedgerState::default()
            })),
            telemetry: Arc::new(TelemetryDeliveryState::default()),
            plutus_home: if cfg!(test) {
                Some(home.join("plutus-test"))
            } else {
                None
            },
        };
        service.hydrate_from_ledger().await?;
        Ok(service)
    }

    pub async fn from_default_or_workspace_fallback() -> anyhow::Result<Self> {
        let home = std::env::var("ARDA_MANDOS_HOME").unwrap_or_else(|_| "data/oracle".into());
        Self::from_home(home).await
    }

    pub fn runtime_paths(&self) -> OracleRuntimePaths {
        OracleRuntimePaths::from_home(&self.home)
    }

    /// Direct best-effort telemetry to an injected Plutus home instead of process-global state.
    pub fn with_plutus_home(mut self, home: impl AsRef<Path>) -> Self {
        self.plutus_home = Some(home.as_ref().to_path_buf());
        self
    }

    pub async fn evaluate(&self, query: OracleQuery) -> anyhow::Result<Verdict> {
        let _evaluation_guard = self.evaluation_lock.lock().await;
        let request_digest = query.request_identity_digest();
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
        if let Err(error) = self.append_verdict(&verdict, request_digest).await {
            self.engine.lock().await.rollback_verdict(&verdict.query_id);
            return Err(error);
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
        if let Err(error) = self.persist_snapshot(&self.snapshot().await?) {
            tracing::warn!(error = %error, "ORACLE status projection could not be refreshed");
        }
        Ok(verdict)
    }

    pub async fn status(&self) -> anyhow::Result<serde_json::Value> {
        let snapshot = self.snapshot().await?;
        if !self.status_path.exists() {
            self.persist_snapshot(&snapshot)?;
        }
        Ok(snapshot)
    }

    /// Return the most recent persisted verdicts, oldest first.
    ///
    /// Note:
    /// - A zero `limit` returns no entries; positive limits are capped at 100.
    /// - `rename` atomicity depends on both temp and final paths living on the same filesystem.
    pub fn recent_verdicts(&self, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit = limit.min(100);
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
                if values.len() >= limit {
                    break;
                }
            }
        }
        values.reverse();
        Ok(values)
    }

    pub async fn verify_ledger(&self) -> anyhow::Result<LedgerVerificationReport> {
        let _guard = self.evaluation_lock.lock().await;
        verify_ledger_file(&self.verdict_ledger_path)
    }

    pub async fn export_verified_ledger(
        &self,
        destination: impl AsRef<Path>,
    ) -> anyhow::Result<LedgerVerificationReport> {
        let _guard = self.evaluation_lock.lock().await;
        let bytes = read_ledger(&self.verdict_ledger_path)?;
        let report = verify_ledger_bytes(&bytes)?;
        anyhow::ensure!(
            report.valid,
            "refusing to export an unverified verdict ledger: {}",
            report.errors.join("; ")
        );
        let destination = destination.as_ref();
        anyhow::ensure!(
            destination != self.verdict_ledger_path,
            "export destination must differ from the authoritative ledger"
        );
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_bytes(destination, &bytes)?;
        Ok(report)
    }

    async fn snapshot(&self) -> anyhow::Result<serde_json::Value> {
        let status = self.engine.lock().await.status_snapshot();
        let recent_verdicts = self.recent_verdicts(10)?;
        let ledger_state = self.ledger_state.lock().await;
        Ok(json!({
            "schema_version": ORACLE_RUNTIME_SCHEMA_VERSION,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "authority": "oracle_service",
            "authority_mode": "authoritative_persisted_verdicts_advisory_decisions",
            "paths": self.runtime_paths(),
            "verdict_runtime": status,
            "evidence_plane": {
                "verdict_ledger_entries": recent_verdicts.len(),
                "recent_persisted_verdicts": recent_verdicts,
                "next_sequence": ledger_state.next_sequence,
                "last_record_digest": ledger_state.last_record_digest,
                "degraded": !ledger_state.degraded_reasons.is_empty(),
                "degraded_reasons": ledger_state.degraded_reasons,
            },
            "telemetry_delivery": self.telemetry.snapshot(),
        }))
    }

    async fn append_verdict(
        &self,
        verdict: &Verdict,
        request_digest: String,
    ) -> anyhow::Result<()> {
        let mut ledger_state = self.ledger_state.lock().await;
        let record = VerdictRecord::new(
            ledger_state.next_sequence,
            request_digest,
            ledger_state.last_record_digest.clone(),
            verdict.redacted_for_export(),
        )?;
        let line = serde_json::to_string(&record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.verdict_ledger_path)?;
        if file.metadata()?.len() > 0 {
            let content = fs::read(&self.verdict_ledger_path)?;
            if content.last() != Some(&b'\n') {
                file.write_all(b"\n")?;
            }
        }
        writeln!(file, "{line}")?;
        file.sync_all()?;
        ledger_state.next_sequence = ledger_state.next_sequence.saturating_add(1);
        ledger_state.last_record_digest = Some(record.record_digest);
        Ok(())
    }

    fn persist_snapshot(&self, snapshot: &serde_json::Value) -> anyhow::Result<()> {
        let data = serde_json::to_string_pretty(snapshot)?.into_bytes();
        let temp_path = self.status_path.with_extension("json.tmp");
        let mut temp = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)?;
        temp.write_all(&data)?;
        temp.sync_all()?;
        fs::rename(&temp_path, &self.status_path)?;
        Ok(())
    }

    async fn hydrate_from_ledger(&self) -> anyhow::Result<()> {
        if !self.verdict_ledger_path.exists() {
            return Ok(());
        }

        let content = match fs::read_to_string(&self.verdict_ledger_path) {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };

        let line_count = content.lines().count();
        let has_terminated_last_line = content.ends_with('\n');
        let mut line_sequence = 0usize;
        let mut expected_record_sequence = 1u64;
        let mut previous_record_digest: Option<String> = None;
        let mut degraded_reasons = Vec::new();
        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            line_sequence = line_sequence.saturating_add(1);
            let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(value) => value,
                Err(_) if line_sequence == line_count && !has_terminated_last_line => {
                    let partial_path = self.verdict_ledger_path.with_extension("jsonl.partial");
                    fs::write(&partial_path, trimmed)?;
                    let partial_start = content.rfind(raw_line).unwrap_or(content.len());
                    let retained = &content[..partial_start];
                    fs::write(&self.verdict_ledger_path, retained)?;
                    degraded_reasons.push(format!(
                        "unterminated trailing fragment quarantined at line {line_sequence}"
                    ));
                    tracing::warn!(
                        path = %partial_path.display(),
                        "ORACLE quarantined an unterminated trailing ledger fragment"
                    );
                    break;
                }
                Err(_) => {
                    degraded_reasons.push(format!(
                        "malformed terminated ledger line at sequence {line_sequence}"
                    ));
                    break;
                }
            };

            if value.get("schema_version").is_some() {
                let record = match VerdictRecord::from_value(value) {
                    Ok(record) => record,
                    Err(error) => {
                        degraded_reasons.push(format!(
                            "invalid verdict record at line {line_sequence}: {error}"
                        ));
                        break;
                    }
                };
                if let Err(error) =
                    record.verify(expected_record_sequence, previous_record_digest.as_deref())
                {
                    degraded_reasons.push(error.to_string());
                    break;
                }
                let verdict = match record.verdict() {
                    Ok(verdict) => verdict,
                    Err(error) => {
                        degraded_reasons.push(format!(
                            "invalid verdict payload at line {line_sequence}: {error}"
                        ));
                        break;
                    }
                };
                self.engine
                    .lock()
                    .await
                    .record_restart_verdict(verdict, Some(record.request_digest.clone()))?;
                expected_record_sequence = expected_record_sequence.saturating_add(1);
                previous_record_digest = Some(record.record_digest);
            } else {
                match serde_json::from_value::<Verdict>(value) {
                    Ok(verdict) => {
                        self.engine
                            .lock()
                            .await
                            .record_restart_verdict(verdict, None)?;
                        if !degraded_reasons
                            .iter()
                            .any(|reason| reason.contains("legacy unchained"))
                        {
                            degraded_reasons.push(
                                "legacy unchained verdict records require export migration"
                                    .to_string(),
                            );
                        }
                    }
                    Err(error) => {
                        degraded_reasons.push(format!(
                            "unsupported ledger record at line {line_sequence}: {error}"
                        ));
                        break;
                    }
                }
            }
        }
        let mut ledger_state = self.ledger_state.lock().await;
        ledger_state.next_sequence = expected_record_sequence;
        ledger_state.last_record_digest = previous_record_digest;
        ledger_state.degraded_reasons = degraded_reasons;
        Ok(())
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> anyhow::Result<()> {
        let plutus = self.plutus_service()?;
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
        let plutus = self.plutus_service()?;
        plutus
            .record_relationship("oracle", to, resonance, attention, reciprocity)
            .await?;
        Ok(())
    }

    fn plutus_service(&self) -> anyhow::Result<PlutusService> {
        match &self.plutus_home {
            Some(home) => PlutusService::from_home(home),
            None => PlutusService::from_default_or_workspace_fallback(),
        }
    }

    fn spawn_telemetry<F>(&self, kind: &'static str, future: F)
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        self.telemetry.retain_running_tasks();
        let permit = match self.telemetry.permits.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                self.telemetry.saturated.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(kind, "ORACLE telemetry delivery saturated; signal dropped");
                return;
            }
        };
        self.telemetry.pending.fetch_add(1, Ordering::Relaxed);
        let telemetry = self.telemetry.clone();
        let task = tokio::spawn(async move {
            let result = future.await;
            telemetry.pending.fetch_sub(1, Ordering::Relaxed);
            match result {
                Ok(()) => {
                    telemetry.delivered.fetch_add(1, Ordering::Relaxed);
                }
                Err(error) => {
                    telemetry.failed.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(kind, %error, "ORACLE best-effort telemetry delivery failed");
                }
            }
            drop(permit);
        });
        self.telemetry
            .tasks
            .lock()
            .expect("telemetry task mutex poisoned")
            .push(task);
    }

    /// Wait for every currently queued best-effort telemetry delivery to finish.
    pub async fn drain_telemetry(&self) {
        loop {
            let tasks = {
                let mut tasks = self
                    .telemetry
                    .tasks
                    .lock()
                    .expect("telemetry task mutex poisoned");
                if tasks.is_empty() {
                    break;
                }
                std::mem::take(&mut *tasks)
            };
            for task in tasks {
                if let Err(error) = task.await {
                    tracing::warn!(%error, "ORACLE telemetry task join failed");
                }
            }
        }
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
        self.spawn_telemetry("work", async move {
            service
                .record_work_signal_async(&agent_id, amount, unit, task_id)
                .await
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
        self.spawn_telemetry("relationship", async move {
            service
                .record_relationship_signal_async(&from, resonance, attention, reciprocity)
                .await
                .map_err(|error| anyhow::anyhow!("task {task_id}: {error}"))
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
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service")
            .with_plutus_home(&plutus_home);
        let verdict = service
            .evaluate(query(
                "oracle-query-1",
                "Should we deploy this with evidence?",
            ))
            .await
            .expect("evaluate");

        assert!(!verdict.reasoning.is_empty());
        service.drain_telemetry().await;
        let status = service.status().await.expect("status");
        assert_eq!(status["authority"], "oracle_service");
        assert_eq!(status["verdict_runtime"]["history_total"], 1);
        assert_eq!(status["telemetry_delivery"]["pending"], 0);
        assert_eq!(status["telemetry_delivery"]["delivered"], 2);
        assert_eq!(status["telemetry_delivery"]["failed"], 0);
        assert!(temp.path().join("verdict_history.jsonl").exists());
        assert!(temp.path().join("runtime_status.json").exists());
        let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
        let total = plutus.status().await.expect("plutus status")["joulework"]["total"]
            .as_f64()
            .unwrap_or(0.0);
        assert!(total > 0.0);
    }

    #[tokio::test]
    async fn invalid_query_is_rejected_before_persistence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");

        let error = service
            .evaluate(query("invalid-query", "   "))
            .await
            .expect_err("blank tasks must be rejected");

        assert!(error.to_string().contains("task"));
        assert!(!temp.path().join("verdict_history.jsonl").exists());
    }

    #[tokio::test]
    async fn append_failure_rolls_back_cached_verdict_and_request_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        fs::create_dir(&service.verdict_ledger_path).expect("blocking ledger directory");
        let oracle_query = query("append-retry", "review durable evidence");

        service
            .evaluate(oracle_query.clone())
            .await
            .expect_err("directory path must reject ledger append");
        assert_eq!(service.engine.lock().await.get_history().len(), 0);
        fs::remove_dir(&service.verdict_ledger_path).expect("remove blocking directory");

        let verdict = service
            .evaluate(oracle_query)
            .await
            .expect("same request must succeed after persistence recovers");
        assert_eq!(verdict.query_id, "append-retry");
        assert_eq!(
            fs::read_to_string(&service.verdict_ledger_path)
                .expect("ledger")
                .lines()
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn conflicting_duplicate_query_id_is_rejected_without_second_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");

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
    }

    #[tokio::test]
    async fn identical_retry_reuses_verdict_without_second_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
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
    }

    #[tokio::test]
    async fn multiple_verdicts_survive_restart_in_ledger_order() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");

        service
            .evaluate(query("restart-one", "review first evidence"))
            .await
            .expect("first verdict");
        service
            .evaluate(query("restart-two", "review second evidence"))
            .await
            .expect("second verdict");

        let ledger = fs::read_to_string(temp.path().join("verdict_history.jsonl")).expect("ledger");
        assert_eq!(ledger.lines().count(), 2);
        let restarted = OracleService::from_home(temp.path())
            .await
            .expect("restart");
        let recent = restarted.recent_verdicts(10).expect("recent verdicts");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0]["query_id"], "restart-one");
        assert_eq!(recent[1]["query_id"], "restart-two");
        let restarted_status = restarted.status().await.expect("status");
        assert_eq!(
            restarted_status["verdict_runtime"]["history_total"], 2,
            "{restarted_status:#}"
        );
    }

    #[tokio::test]
    async fn request_identity_survives_restart_for_retry_and_conflict_detection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let original = query("restart-identity", "review stable evidence");
        let first = OracleService::from_home(temp.path())
            .await
            .expect("service")
            .evaluate(original.clone())
            .await
            .expect("first verdict");
        let restarted = OracleService::from_home(temp.path())
            .await
            .expect("restart");
        let mut retry = original;
        retry.timestamp += chrono::TimeDelta::seconds(10);

        let retried = restarted.evaluate(retry).await.expect("retry verdict");
        assert_eq!(retried.timestamp, first.timestamp);
        let conflict = restarted
            .evaluate(query("restart-identity", "review different evidence"))
            .await
            .expect_err("conflicting duplicate after restart");
        assert!(conflict.to_string().contains("restart-identity"));
        let ledger =
            fs::read_to_string(temp.path().join("verdict_history.jsonl")).expect("verdict ledger");
        assert_eq!(ledger.lines().count(), 1);
    }

    #[tokio::test]
    async fn integrity_failures_degrade_with_valid_prefix_retained() {
        let fixtures = [
            ("payload_tamper", "verdict digest mismatch"),
            ("record_digest_tamper", "record digest mismatch"),
            ("sequence_gap", "verdict record sequence gap"),
            ("chain_mismatch", "verdict record digest chain mismatch"),
            ("future_schema", "unsupported verdict record schema"),
        ];

        for (fixture, expected_reason) in fixtures {
            let temp = tempfile::tempdir().expect("tempdir");
            let service = OracleService::from_home(temp.path())
                .await
                .expect("service");
            service
                .evaluate(query("integrity-prefix", "review prefix evidence"))
                .await
                .expect("prefix verdict");
            service
                .evaluate(query("integrity-tail", "review tail evidence"))
                .await
                .expect("tail verdict");
            drop(service);

            let ledger_path = temp.path().join("verdict_history.jsonl");
            let mut records: Vec<serde_json::Value> = fs::read_to_string(&ledger_path)
                .expect("ledger")
                .lines()
                .map(|line| serde_json::from_str(line).expect("record"))
                .collect();
            match fixture {
                "payload_tamper" => records[1]["query_id"] = json!("tampered-tail"),
                "record_digest_tamper" => records[1]["record_digest"] = json!("sha256:tampered"),
                "sequence_gap" => records[1]["sequence"] = json!(3),
                "chain_mismatch" => {
                    records[1]["previous_record_digest"] = json!("sha256:wrong-parent")
                }
                "future_schema" => {
                    records[1]["schema_version"] = json!("arda.mandos.verdict-record.v999")
                }
                _ => unreachable!(),
            }
            let rewritten = records
                .iter()
                .map(|record| serde_json::to_string(record).expect("serialize record"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            fs::write(&ledger_path, rewritten).expect("rewrite fixture");

            let restarted = OracleService::from_home(temp.path())
                .await
                .expect("degraded restart");
            let status = restarted.status().await.expect("status");
            assert_eq!(
                status["verdict_runtime"]["history_total"], 1,
                "fixture {fixture} must retain exactly the valid prefix"
            );
            assert_eq!(status["evidence_plane"]["degraded"], true);
            assert!(status["evidence_plane"]["degraded_reasons"][0]
                .as_str()
                .expect("degraded reason")
                .contains(expected_reason));
        }
    }

    #[tokio::test]
    async fn restart_quarantines_only_an_unterminated_trailing_fragment() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        service
            .evaluate(query("valid-before-partial", "review valid evidence"))
            .await
            .expect("valid verdict");
        let ledger_path = temp.path().join("verdict_history.jsonl");
        OpenOptions::new()
            .append(true)
            .open(&ledger_path)
            .expect("ledger")
            .write_all(b"{\"query_id\":\"interrupted")
            .expect("partial write");

        let restarted = OracleService::from_home(temp.path())
            .await
            .expect("recover restart");

        assert_eq!(restarted.recent_verdicts(10).expect("recent").len(), 1);
        assert!(temp.path().join("verdict_history.jsonl.partial").exists());
        assert!(fs::read_to_string(ledger_path)
            .expect("repaired ledger")
            .ends_with('\n'));
        let status = restarted.status().await.expect("status");
        assert_eq!(status["verdict_runtime"]["history_total"], 1);
        assert_eq!(status["evidence_plane"]["degraded"], true);
        assert!(status["evidence_plane"]["degraded_reasons"][0]
            .as_str()
            .expect("degraded reason")
            .contains("unterminated trailing fragment quarantined"));
    }

    #[tokio::test]
    async fn malformed_terminated_ledger_record_degrades_restart() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("verdict_history.jsonl"), "not-json\n")
            .expect("malformed fixture");

        let restarted = OracleService::from_home(temp.path())
            .await
            .expect("degraded restart");
        let status = restarted.status().await.expect("status");

        assert_eq!(status["verdict_runtime"]["history_total"], 0);
        assert_eq!(status["evidence_plane"]["degraded"], true);
        assert!(status["evidence_plane"]["degraded_reasons"][0]
            .as_str()
            .expect("degraded reason")
            .contains("malformed terminated ledger line at sequence 1"));
    }

    #[tokio::test]
    async fn verdict_preserves_caller_timestamp() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
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
    }

    #[tokio::test]
    async fn persisted_exports_redact_sensitive_excerpts_but_retain_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
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
    }

    #[tokio::test]
    async fn recent_verdict_exports_redact_legacy_excerpt_fields() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
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

    #[tokio::test]
    async fn recent_verdicts_honors_limit_and_returns_oldest_first() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _ = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");

        let mut engine = service.engine.lock().await;
        let ledger_path = service.verdict_ledger_path.clone();
        let total = 12usize;
        let limit = 5usize;
        for index in 0..total {
            let verdict = engine
                .evaluate(query(
                    &format!("cap-query-{index}"),
                    &format!("review deployment evidence {index}"),
                ))
                .expect("evaluate");
            let line = serde_json::to_string(&verdict.redacted_for_export()).expect("serialize");
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ledger_path)
                .expect("ledger file")
                .write_all(line.as_bytes())
                .expect("write line");
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ledger_path)
                .expect("ledger file")
                .write_all(b"\n")
                .expect("write newline");
        }
        drop(engine);

        let recent = service.recent_verdicts(limit).expect("recent verdicts");
        assert_eq!(recent.len(), limit);
        assert_eq!(recent[0]["query_id"].as_str().unwrap(), "cap-query-7");
        assert_eq!(
            recent[recent.len() - 1]["query_id"].as_str().unwrap(),
            "cap-query-11"
        );
    }

    #[tokio::test]
    async fn verified_ledger_export_preserves_exact_authoritative_bytes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        service
            .evaluate(query("export-one", "review first export record"))
            .await
            .expect("first verdict");
        service
            .evaluate(query("export-two", "review second export record"))
            .await
            .expect("second verdict");
        let authoritative = fs::read(&service.verdict_ledger_path).expect("authoritative ledger");
        let destination = temp.path().join("exports/verdict_history.jsonl");

        let report = service
            .export_verified_ledger(&destination)
            .await
            .expect("verified export");

        assert!(report.valid);
        assert_eq!(report.valid_records, 2);
        assert_eq!(report.next_sequence, 3);
        assert!(report.chain_head.is_some());
        assert_eq!(
            fs::read(destination).expect("exported ledger"),
            authoritative
        );
    }

    #[tokio::test]
    async fn verified_ledger_export_refuses_an_authoritative_path_alias() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        service
            .evaluate(query("export-alias", "export alias"))
            .await
            .expect("verdict");
        let before = fs::read(&service.verdict_ledger_path).expect("authoritative ledger");
        let aliased_authoritative = temp.path().join(".").join("verdict_history.jsonl");

        let error = service
            .export_verified_ledger(&aliased_authoritative)
            .await
            .expect_err("an alias of the authoritative ledger must be refused");

        assert!(error
            .to_string()
            .contains("export destination must differ from the authoritative ledger"));
        assert_eq!(
            fs::read(&service.verdict_ledger_path).expect("preserved authoritative ledger"),
            before
        );
    }

    #[tokio::test]
    async fn verification_and_export_refuse_integrity_and_schema_failures() {
        let fixtures = [
            ("digest_tamper", "verdict digest mismatch"),
            ("sequence_gap", "verdict record sequence gap"),
            ("future_schema", "unsupported verdict record schema"),
        ];

        for (fixture, expected_error) in fixtures {
            let temp = tempfile::tempdir().expect("tempdir");
            let service = OracleService::from_home(temp.path())
                .await
                .expect("service");
            service
                .evaluate(query("verify-one", "review first verification record"))
                .await
                .expect("first verdict");
            service
                .evaluate(query("verify-two", "review second verification record"))
                .await
                .expect("second verdict");
            let mut records: Vec<serde_json::Value> =
                fs::read_to_string(&service.verdict_ledger_path)
                    .expect("ledger")
                    .lines()
                    .map(|line| serde_json::from_str(line).expect("record"))
                    .collect();
            match fixture {
                "digest_tamper" => records[1]["query_id"] = json!("tampered-query"),
                "sequence_gap" => records[1]["sequence"] = json!(3),
                "future_schema" => {
                    records[1]["schema_version"] = json!("arda.mandos.verdict-record.v999")
                }
                _ => unreachable!(),
            }
            let corrupted = records
                .iter()
                .map(|record| serde_json::to_string(record).expect("serialize record"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            fs::write(&service.verdict_ledger_path, corrupted).expect("corrupt ledger fixture");
            let destination = temp.path().join(format!("{fixture}.jsonl"));

            let report = service.verify_ledger().await.expect("verification report");
            assert!(!report.valid, "fixture {fixture} must be invalid");
            assert_eq!(report.valid_records, 1);
            assert!(report
                .errors
                .iter()
                .any(|error| error.contains(expected_error)));
            let export_error = service
                .export_verified_ledger(&destination)
                .await
                .expect_err("corrupt ledger must not export");
            assert!(export_error.to_string().contains(expected_error));
            assert!(!destination.exists());
        }
    }

    #[tokio::test]
    async fn failed_export_preserves_destination_and_removes_temporary_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        service
            .evaluate(query("atomic-export", "review atomic export behavior"))
            .await
            .expect("verdict");
        let destination = temp.path().join("existing-directory");
        fs::create_dir(&destination).expect("destination directory");
        fs::write(destination.join("sentinel"), "preserve-me").expect("sentinel");

        service
            .export_verified_ledger(&destination)
            .await
            .expect_err("renaming a file over a directory must fail");

        assert_eq!(
            fs::read_to_string(destination.join("sentinel")).expect("preserved sentinel"),
            "preserve-me"
        );
        let leaked_temps = fs::read_dir(temp.path())
            .expect("temp root")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("existing-directory.tmp.")
            })
            .count();
        assert_eq!(leaked_temps, 0, "failed export must clean temporary files");
    }
}
