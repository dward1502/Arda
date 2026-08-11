// sigil: REPAIR
use crate::operator::{CompactPhilosopherEvidence, GovernanceDecisionConfidenceBand};
use crate::paths::GovernancePaths;
use crate::triad::{
    triad_validate, GateOutcome, GovernanceReviewMode, GovernanceVetoReason, TriadConfig,
    TriadResult,
};
use crate::versions::{legacy_bacon_lite_policy_version, BACON_LITE_POLICY_VERSION};
use crate::{calculate_resonance_with_triad, GovernanceScoringSource};
use arda_core::ledger::{GovernanceLedgerSink, LedgerEnqueueError};
use arda_core::task::Task;
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Explicit destinations for Bacon-Lite machine and operator evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaconLiteLogPaths {
    pub machine: PathBuf,
    pub human: PathBuf,
}

impl BaconLiteLogPaths {
    pub fn from_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let paths = GovernancePaths::new(base_dir);
        Self {
            machine: paths.bacon_lite_machine_log(),
            human: paths.bacon_lite_human_log(),
        }
    }
}

fn unknown_source_maturity() -> String {
    "unknown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaconLiteResult {
    #[serde(default = "legacy_bacon_lite_policy_version")]
    pub policy_version: String,
    pub passed: bool,
    pub confidence: f64,
    pub mode: String,
    pub rationale: String,
    pub triad: TriadResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaconLiteEvent {
    #[serde(default = "legacy_bacon_lite_policy_version")]
    pub policy_version: String,
    #[serde(default)]
    pub scorer_version: String,
    #[serde(default)]
    pub review_mode: GovernanceReviewMode,
    #[serde(default = "unknown_source_maturity")]
    pub source_maturity: String,
    #[serde(default)]
    pub evidence_source: Option<GovernanceScoringSource>,
    pub ts_utc: String,
    pub crate_name: String,
    pub action: String,
    pub task_id: String,
    pub task_type: String,
    pub description: String,
    pub passed: bool,
    pub confidence: f64,
    pub rationale: String,
    pub triad_passed: bool,
    #[serde(default)]
    pub typed_veto: Option<GovernanceVetoReason>,
    #[serde(default)]
    pub confidence_band: GovernanceDecisionConfidenceBand,
    #[serde(default)]
    pub philosopher_evidence: Option<CompactPhilosopherEvidence>,
    #[serde(default)]
    pub aurelius_outcome: Option<GateOutcome>,
    #[serde(default)]
    pub bacon_outcome: Option<GateOutcome>,
    #[serde(default)]
    pub sun_tzu_outcome: Option<GateOutcome>,
    pub aurelius_score: f64,
    pub bacon_score: f64,
    pub sun_tzu_score: f64,
    pub context: Value,
}

impl BaconLiteEvent {
    pub fn validate(&self) -> std::io::Result<()> {
        if self.crate_name.trim().is_empty() || self.action.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Bacon-Lite crate and action must be non-empty",
            ));
        }
        if !self.confidence.is_finite()
            || !self.aurelius_score.is_finite()
            || !self.bacon_score.is_finite()
            || !self.sun_tzu_score.is_finite()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Bacon-Lite scores must be finite",
            ));
        }
        DateTime::parse_from_rfc3339(&self.ts_utc).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid Bacon-Lite timestamp: {error}"),
            )
        })?;
        Ok(())
    }
}

pub fn bacon_lite_validate(task: &Task) -> BaconLiteResult {
    let triad = triad_validate(
        task,
        Some(&TriadConfig {
            strict: false,
            required_passes: Some(1),
        }),
    );

    let bacon_ok = triad.bacon != GateOutcome::Fail;
    let support_ok = triad.aurelius != GateOutcome::Fail || triad.sun_tzu != GateOutcome::Fail;
    let passed = bacon_ok && support_ok;
    let confidence =
        ((triad.bacon_score * 0.6) + (triad.aurelius_score * 0.2) + (triad.sun_tzu_score * 0.2))
            .clamp(0.0, 1.0);
    let rationale = if passed {
        "bacon-lite pass: evidence gate plus one support gate".to_string()
    } else {
        "bacon-lite fail: insufficient evidence/support gates".to_string()
    };

    BaconLiteResult {
        policy_version: BACON_LITE_POLICY_VERSION.to_string(),
        passed,
        confidence,
        mode: "bacon_lite".to_string(),
        rationale,
        triad,
    }
}

/// Construct and validate an event without performing persistence work.
pub fn build_bacon_lite_event(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
) -> std::io::Result<BaconLiteEvent> {
    build_bacon_lite_event_with_description(
        crate_name,
        action,
        task,
        task.description.clone(),
        context,
    )
}

/// Construct an event while keeping the full task available to governance
/// scoring and persisting only the caller-provided safe description.
pub fn build_bacon_lite_event_with_description(
    crate_name: &str,
    action: &str,
    task: &Task,
    persisted_description: String,
    context: Value,
) -> std::io::Result<BaconLiteEvent> {
    let result = bacon_lite_validate(task);
    let resonance = calculate_resonance_with_triad(task, &result.triad, None, None);
    let event = BaconLiteEvent {
        policy_version: BACON_LITE_POLICY_VERSION.to_string(),
        scorer_version: result.triad.policy_version.clone(),
        review_mode: result.triad.review_mode,
        source_maturity: result.triad.profile_maturity.clone(),
        evidence_source: Some(result.triad.evidence.scoring_source),
        ts_utc: Utc::now().to_rfc3339(),
        crate_name: crate_name.to_string(),
        action: action.to_string(),
        task_id: task.id.to_string(),
        task_type: task.task_type.clone(),
        description: persisted_description,
        passed: result.passed,
        confidence: result.confidence,
        rationale: result.rationale,
        triad_passed: result.triad.passed,
        typed_veto: result.triad.veto.clone(),
        confidence_band: GovernanceDecisionConfidenceBand::from_confidence(result.confidence),
        philosopher_evidence: resonance.triad_philosopher.map(Into::into),
        aurelius_outcome: Some(result.triad.aurelius),
        bacon_outcome: Some(result.triad.bacon),
        sun_tzu_outcome: Some(result.triad.sun_tzu),
        aurelius_score: result.triad.aurelius_score,
        bacon_score: result.triad.bacon_score,
        sun_tzu_score: result.triad.sun_tzu_score,
        context,
    };
    event.validate()?;
    crate::global_governance_metrics().observe_bacon_lite(&event);
    Ok(event)
}

/// Cold-path synchronous compatibility adapter for tests and migrations.
pub fn record_bacon_lite(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
) -> std::io::Result<BaconLiteEvent> {
    let paths = default_log_paths()?;
    record_bacon_lite_to(crate_name, action, task, context, &paths)
}

/// Cold-path synchronous compatibility adapter with caller-selected destinations.
pub fn record_bacon_lite_to(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
    paths: &BaconLiteLogPaths,
) -> std::io::Result<BaconLiteEvent> {
    let event = build_bacon_lite_event(crate_name, action, task, context)?;
    persist_batch(
        paths,
        std::slice::from_ref(&event),
        &BaconLiteWriterConfig::default(),
    )?;
    Ok(event)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaconLiteBackpressurePolicy {
    DropNewest,
}

#[derive(Debug, Clone)]
pub struct BaconLiteWriterConfig {
    pub queue_capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub max_file_bytes: u64,
    pub retained_files: usize,
    pub backpressure: BaconLiteBackpressurePolicy,
}

impl Default for BaconLiteWriterConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 4_096,
            batch_size: 128,
            flush_interval: Duration::from_millis(100),
            max_file_bytes: 64 * 1024 * 1024,
            retained_files: 7,
            backpressure: BaconLiteBackpressurePolicy::DropNewest,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BaconLiteWriterCounters {
    pub accepted_events: u64,
    pub written_events: u64,
    pub dropped_events: u64,
    pub failed_events: u64,
    pub write_errors: u64,
}

#[derive(Default)]
struct WriterCounters {
    accepted_events: AtomicU64,
    written_events: AtomicU64,
    dropped_events: AtomicU64,
    failed_events: AtomicU64,
    write_errors: AtomicU64,
}

impl WriterCounters {
    fn snapshot(&self) -> BaconLiteWriterCounters {
        BaconLiteWriterCounters {
            accepted_events: self.accepted_events.load(Ordering::Relaxed),
            written_events: self.written_events.load(Ordering::Relaxed),
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
            failed_events: self.failed_events.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
        }
    }
}

enum WriterCommand {
    Event(Box<BaconLiteEvent>),
    Flush(mpsc::SyncSender<std::io::Result<()>>),
    Shutdown(mpsc::SyncSender<std::io::Result<()>>),
}

struct WriterInner {
    sender: mpsc::SyncSender<WriterCommand>,
    counters: Arc<WriterCounters>,
    closed: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct BaconLiteWriter {
    inner: Arc<WriterInner>,
}

impl BaconLiteWriter {
    /// Start a writer with queue and batching settings sourced from the
    /// standard Bacon-Lite environment variables.
    pub fn start_from_env(paths: BaconLiteLogPaths) -> std::io::Result<Self> {
        Self::start(paths, config_from_env())
    }

    pub fn start(paths: BaconLiteLogPaths, config: BaconLiteWriterConfig) -> std::io::Result<Self> {
        if config.queue_capacity == 0 || config.batch_size == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Bacon-Lite queue capacity and batch size must be non-zero",
            ));
        }
        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let counters = Arc::new(WriterCounters::default());
        let worker_counters = Arc::clone(&counters);
        let worker = thread::Builder::new()
            .name("bacon-lite-ledger".to_string())
            .spawn(move || writer_loop(receiver, paths, config, worker_counters))?;
        Ok(Self {
            inner: Arc::new(WriterInner {
                sender,
                counters,
                closed: AtomicBool::new(false),
                worker: Mutex::new(Some(worker)),
            }),
        })
    }

    pub fn counters(&self) -> BaconLiteWriterCounters {
        self.inner.counters.snapshot()
    }

    pub fn flush(&self) -> std::io::Result<()> {
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(closed_writer_error());
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        self.inner
            .sender
            .send(WriterCommand::Flush(sender))
            .map_err(|_| closed_writer_error())?;
        receiver.recv().map_err(|_| closed_writer_error())?
    }

    pub fn shutdown(&self) -> std::io::Result<()> {
        if self.inner.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let send_result = self.inner.sender.send(WriterCommand::Shutdown(sender));
        let result = match send_result {
            Ok(()) => receiver.recv().map_err(|_| closed_writer_error())?,
            Err(_) => Err(closed_writer_error()),
        };
        if let Some(worker) = self
            .inner
            .worker
            .lock()
            .expect("writer mutex poisoned")
            .take()
        {
            if worker.join().is_err() {
                return Err(std::io::Error::other("Bacon-Lite writer thread panicked"));
            }
        }
        result
    }
}

impl GovernanceLedgerSink<BaconLiteEvent> for BaconLiteWriter {
    fn try_enqueue(&self, event: BaconLiteEvent) -> Result<(), LedgerEnqueueError> {
        if self.inner.closed.load(Ordering::Acquire) {
            self.inner
                .counters
                .dropped_events
                .fetch_add(1, Ordering::Relaxed);
            return Err(LedgerEnqueueError::Closed);
        }
        match self
            .inner
            .sender
            .try_send(WriterCommand::Event(Box::new(event)))
        {
            Ok(()) => {
                self.inner
                    .counters
                    .accepted_events
                    .fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.inner
                    .counters
                    .dropped_events
                    .fetch_add(1, Ordering::Relaxed);
                Err(LedgerEnqueueError::Saturated)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.inner
                    .counters
                    .dropped_events
                    .fetch_add(1, Ordering::Relaxed);
                Err(LedgerEnqueueError::Closed)
            }
        }
    }
}

#[derive(Debug)]
pub enum BaconLiteEnqueueError {
    Invalid(std::io::Error),
    Transport(LedgerEnqueueError),
    Startup(std::io::Error),
}

impl std::fmt::Display for BaconLiteEnqueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(error) => write!(formatter, "invalid Bacon-Lite event: {error}"),
            Self::Transport(error) => error.fmt(formatter),
            Self::Startup(error) => write!(formatter, "Bacon-Lite writer startup failed: {error}"),
        }
    }
}

impl std::error::Error for BaconLiteEnqueueError {}

static GLOBAL_WRITER: OnceLock<BaconLiteWriter> = OnceLock::new();

pub fn enqueue_bacon_lite(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
) -> Result<BaconLiteEvent, BaconLiteEnqueueError> {
    let writer = global_writer().map_err(BaconLiteEnqueueError::Startup)?;
    enqueue_bacon_lite_with(writer, crate_name, action, task, context)
}

/// Enqueue an event through an application-owned writer with explicit paths.
pub fn enqueue_bacon_lite_with(
    writer: &BaconLiteWriter,
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
) -> Result<BaconLiteEvent, BaconLiteEnqueueError> {
    let event = build_bacon_lite_event(crate_name, action, task, context)
        .map_err(BaconLiteEnqueueError::Invalid)?;
    writer
        .try_enqueue(event.clone())
        .map_err(BaconLiteEnqueueError::Transport)?;
    Ok(event)
}

/// Enqueue an event while keeping sensitive task text out of the ledger.
/// Governance scoring still evaluates the original task.
pub fn enqueue_bacon_lite_with_description(
    writer: &BaconLiteWriter,
    crate_name: &str,
    action: &str,
    task: &Task,
    persisted_description: String,
    context: Value,
) -> Result<BaconLiteEvent, BaconLiteEnqueueError> {
    let event = build_bacon_lite_event_with_description(
        crate_name,
        action,
        task,
        persisted_description,
        context,
    )
    .map_err(BaconLiteEnqueueError::Invalid)?;
    writer
        .try_enqueue(event.clone())
        .map_err(BaconLiteEnqueueError::Transport)?;
    Ok(event)
}

pub fn global_bacon_lite_counters() -> Option<BaconLiteWriterCounters> {
    GLOBAL_WRITER.get().map(BaconLiteWriter::counters)
}

fn global_writer() -> std::io::Result<&'static BaconLiteWriter> {
    if let Some(writer) = GLOBAL_WRITER.get() {
        return Ok(writer);
    }
    let paths = default_log_paths()?;
    let writer = BaconLiteWriter::start(paths, config_from_env())?;
    let _ = GLOBAL_WRITER.set(writer);
    GLOBAL_WRITER
        .get()
        .ok_or_else(|| std::io::Error::other("failed to initialize Bacon-Lite writer"))
}

fn config_from_env() -> BaconLiteWriterConfig {
    let mut config = BaconLiteWriterConfig::default();
    if let Some(value) = env_usize("ARDA_BACON_LITE_QUEUE_CAPACITY") {
        config.queue_capacity = value.max(1);
    }
    if let Some(value) = env_usize("ARDA_BACON_LITE_BATCH_SIZE") {
        config.batch_size = value.max(1);
    }
    if let Some(value) = env_u64("ARDA_BACON_LITE_FLUSH_MS") {
        config.flush_interval = Duration::from_millis(value.max(1));
    }
    if let Some(value) = env_u64("ARDA_BACON_LITE_MAX_BYTES") {
        config.max_file_bytes = value;
    }
    if let Some(value) = env_usize("ARDA_BACON_LITE_RETAINED_FILES") {
        config.retained_files = value;
    }
    config
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok()?.parse().ok()
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

fn writer_loop(
    receiver: mpsc::Receiver<WriterCommand>,
    paths: BaconLiteLogPaths,
    config: BaconLiteWriterConfig,
    counters: Arc<WriterCounters>,
) {
    let mut pending = Vec::with_capacity(config.batch_size);
    loop {
        let command = if pending.is_empty() {
            match receiver.recv() {
                Ok(command) => command,
                Err(_) => break,
            }
        } else {
            match receiver.recv_timeout(config.flush_interval) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    persist_pending(&paths, &config, &mut pending, &counters);
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    persist_pending(&paths, &config, &mut pending, &counters);
                    break;
                }
            }
        };

        match command {
            WriterCommand::Event(event) => {
                pending.push(*event);
                while pending.len() < config.batch_size {
                    match receiver.try_recv() {
                        Ok(WriterCommand::Event(event)) => pending.push(*event),
                        Ok(WriterCommand::Flush(reply)) => {
                            let result =
                                persist_pending_result(&paths, &config, &mut pending, &counters);
                            let _ = reply.send(result);
                        }
                        Ok(WriterCommand::Shutdown(reply)) => {
                            let result =
                                persist_pending_result(&paths, &config, &mut pending, &counters);
                            let _ = reply.send(result);
                            return;
                        }
                        Err(_) => break,
                    }
                }
                if pending.len() >= config.batch_size {
                    persist_pending(&paths, &config, &mut pending, &counters);
                }
            }
            WriterCommand::Flush(reply) => {
                let result = persist_pending_result(&paths, &config, &mut pending, &counters);
                let _ = reply.send(result);
            }
            WriterCommand::Shutdown(reply) => {
                let result = persist_pending_result(&paths, &config, &mut pending, &counters);
                let _ = reply.send(result);
                break;
            }
        }
    }
}

fn persist_pending(
    paths: &BaconLiteLogPaths,
    config: &BaconLiteWriterConfig,
    pending: &mut Vec<BaconLiteEvent>,
    counters: &WriterCounters,
) {
    let _ = persist_pending_result(paths, config, pending, counters);
}

fn persist_pending_result(
    paths: &BaconLiteLogPaths,
    config: &BaconLiteWriterConfig,
    pending: &mut Vec<BaconLiteEvent>,
    counters: &WriterCounters,
) -> std::io::Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let event_count = pending.len() as u64;
    let result = persist_batch(paths, pending, config);
    match &result {
        Ok(()) => {
            counters
                .written_events
                .fetch_add(event_count, Ordering::Relaxed);
        }
        Err(_) => {
            counters.write_errors.fetch_add(1, Ordering::Relaxed);
            counters
                .failed_events
                .fetch_add(event_count, Ordering::Relaxed);
        }
    }
    pending.clear();
    result
}

fn persist_batch(
    paths: &BaconLiteLogPaths,
    events: &[BaconLiteEvent],
    config: &BaconLiteWriterConfig,
) -> std::io::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    let mut machine_bytes = Vec::new();
    let mut human_bytes = Vec::new();
    for event in events {
        event.validate()?;
        serde_json::to_writer(&mut machine_bytes, event)?;
        machine_bytes.push(b'\n');
        writeln!(
            human_bytes,
            "- {} | crate=`{}` action=`{}` passed=`{}` confidence=`{:.3}` task_type=`{}`",
            event.ts_utc,
            event.crate_name,
            event.action,
            event.passed,
            event.confidence,
            event.task_type
        )?;
    }

    if let Some(parent) = paths.machine.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = paths.human.parent() {
        fs::create_dir_all(parent)?;
    }
    let lock_path = paths.machine.with_extension("jsonl.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;

    rotate_if_needed(
        &paths.machine,
        machine_bytes.len() as u64,
        config.max_file_bytes,
        config.retained_files,
    )?;
    rotate_if_needed(
        &paths.human,
        human_bytes.len() as u64,
        config.max_file_bytes,
        config.retained_files,
    )?;

    let mut machine = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.machine)?;
    machine.write_all(&machine_bytes)?;
    machine.flush()?;

    let human_is_new = !paths.human.exists() || fs::metadata(&paths.human)?.len() == 0;
    let mut human = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.human)?;
    if human_is_new {
        human.write_all(b"# Bacon-Lite Validation Log\n\n")?;
    }
    human.write_all(&human_bytes)?;
    human.flush()?;
    FileExt::unlock(&lock)?;
    Ok(())
}

fn rotate_if_needed(
    path: &Path,
    incoming_bytes: u64,
    max_file_bytes: u64,
    retained_files: usize,
) -> std::io::Result<()> {
    if max_file_bytes == 0 || !path.exists() {
        return Ok(());
    }
    if fs::metadata(path)?.len().saturating_add(incoming_bytes) <= max_file_bytes {
        return Ok(());
    }
    if retained_files == 0 {
        fs::remove_file(path)?;
        return Ok(());
    }
    let oldest = rotated_path(path, retained_files);
    if oldest.exists() {
        fs::remove_file(oldest)?;
    }
    for generation in (1..retained_files).rev() {
        let source = rotated_path(path, generation);
        if source.exists() {
            fs::rename(source, rotated_path(path, generation + 1))?;
        }
    }
    fs::rename(path, rotated_path(path, 1))?;
    Ok(())
}

fn rotated_path(path: &Path, generation: usize) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{generation}"));
    PathBuf::from(name)
}

fn closed_writer_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "Bacon-Lite writer is closed",
    )
}

fn default_log_paths() -> std::io::Result<BaconLiteLogPaths> {
    let base = std::env::var_os("ARDA_ROOT")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;
    let mut paths = BaconLiteLogPaths::from_base_dir(base);
    if let Some(path) = std::env::var_os("ARDA_BACON_LITE_LOG_PATH") {
        paths.machine = PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("ARDA_BACON_LITE_HUMAN_PATH") {
        paths.human = PathBuf::from(path);
    }
    Ok(paths)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedLineBehavior {
    CountAndSkip,
    Fail,
}

#[derive(Debug, Clone)]
pub struct BaconLiteReadWindow {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub malformed: MalformedLineBehavior,
    pub include_rotated: bool,
}

impl Default for BaconLiteReadWindow {
    fn default() -> Self {
        Self {
            since: None,
            until: None,
            malformed: MalformedLineBehavior::CountAndSkip,
            include_rotated: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BaconLiteAggregate {
    pub record_count: u64,
    pub passed_count: u64,
    pub pass_rate: f64,
    pub mean_confidence: f64,
    pub lens_outcomes: BTreeMap<String, BTreeMap<String, u64>>,
    pub scorer_versions: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BaconLiteLedgerSummary {
    pub records: u64,
    pub malformed_records: u64,
    pub groups: BTreeMap<String, BTreeMap<String, BaconLiteAggregate>>,
}

#[derive(Default)]
struct AggregateAccumulator {
    count: u64,
    passed: u64,
    confidence_sum: f64,
    lens_outcomes: BTreeMap<String, BTreeMap<String, u64>>,
    scorer_versions: BTreeMap<String, u64>,
}

pub fn read_bacon_lite_summary(
    machine_path: impl AsRef<Path>,
    window: &BaconLiteReadWindow,
) -> std::io::Result<BaconLiteLedgerSummary> {
    let machine_path = machine_path.as_ref();
    let mut files = vec![machine_path.to_path_buf()];
    if window.include_rotated {
        let mut generation = 1;
        loop {
            let candidate = rotated_path(machine_path, generation);
            if !candidate.exists() {
                break;
            }
            files.push(candidate);
            generation += 1;
        }
    }

    let mut malformed_records = 0u64;
    let mut accumulators = BTreeMap::<String, BTreeMap<String, AggregateAccumulator>>::new();
    for path in files.into_iter().rev() {
        if !path.exists() {
            continue;
        }
        let file = File::open(path)?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event = match serde_json::from_str::<BaconLiteEvent>(&line) {
                Ok(event) if event.validate().is_ok() => event,
                Ok(_) | Err(_) => {
                    malformed_records += 1;
                    if window.malformed == MalformedLineBehavior::Fail {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "malformed Bacon-Lite ledger record",
                        ));
                    }
                    continue;
                }
            };
            let timestamp = DateTime::parse_from_rfc3339(&event.ts_utc)
                .expect("validated timestamp")
                .with_timezone(&Utc);
            if window.since.is_some_and(|since| timestamp < since)
                || window.until.is_some_and(|until| timestamp > until)
            {
                continue;
            }
            let aggregate = accumulators
                .entry(event.crate_name.clone())
                .or_default()
                .entry(event.action.clone())
                .or_default();
            aggregate.count += 1;
            aggregate.passed += u64::from(event.passed);
            aggregate.confidence_sum += event.confidence;
            let scorer_version = if event.scorer_version.is_empty() {
                event.policy_version.clone()
            } else {
                event.scorer_version.clone()
            };
            *aggregate.scorer_versions.entry(scorer_version).or_default() += 1;
            add_outcome(
                &mut aggregate.lens_outcomes,
                "aurelius",
                event.aurelius_outcome,
            );
            add_outcome(&mut aggregate.lens_outcomes, "bacon", event.bacon_outcome);
            add_outcome(
                &mut aggregate.lens_outcomes,
                "sun_tzu",
                event.sun_tzu_outcome,
            );
        }
    }

    let records = accumulators
        .values()
        .flat_map(BTreeMap::values)
        .map(|aggregate| aggregate.count)
        .sum();
    let groups = accumulators
        .into_iter()
        .map(|(crate_name, actions)| {
            let actions = actions
                .into_iter()
                .map(|(action, accumulator)| {
                    let count = accumulator.count;
                    (
                        action,
                        BaconLiteAggregate {
                            record_count: count,
                            passed_count: accumulator.passed,
                            pass_rate: if count == 0 {
                                0.0
                            } else {
                                accumulator.passed as f64 / count as f64
                            },
                            mean_confidence: if count == 0 {
                                0.0
                            } else {
                                accumulator.confidence_sum / count as f64
                            },
                            lens_outcomes: accumulator.lens_outcomes,
                            scorer_versions: accumulator.scorer_versions,
                        },
                    )
                })
                .collect();
            (crate_name, actions)
        })
        .collect();
    Ok(BaconLiteLedgerSummary {
        records,
        malformed_records,
        groups,
    })
}

/// Return the newest valid event, skipping malformed records and optionally
/// walking retained generations when the active file has no valid entries.
pub fn read_latest_bacon_lite_event(
    machine_path: impl AsRef<Path>,
    include_rotated: bool,
) -> std::io::Result<Option<BaconLiteEvent>> {
    let machine_path = machine_path.as_ref();
    let mut files = vec![machine_path.to_path_buf()];
    if include_rotated {
        let mut generation = 1;
        loop {
            let candidate = rotated_path(machine_path, generation);
            if !candidate.exists() {
                break;
            }
            files.push(candidate);
            generation += 1;
        }
    }
    for path in files {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(path)?;
        for line in content.lines().rev() {
            let Ok(event) = serde_json::from_str::<BaconLiteEvent>(line) else {
                continue;
            };
            if event.validate().is_ok() {
                return Ok(Some(event));
            }
        }
    }
    Ok(None)
}

fn add_outcome(
    outcomes: &mut BTreeMap<String, BTreeMap<String, u64>>,
    lens: &str,
    outcome: Option<GateOutcome>,
) {
    let label = match outcome {
        Some(GateOutcome::Pass) => "pass",
        Some(GateOutcome::Fail) => "fail",
        Some(GateOutcome::Conditional) => "conditional",
        None => "unknown",
    };
    *outcomes
        .entry(lens.to_string())
        .or_default()
        .entry(label.to_string())
        .or_default() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::Task;
    use std::sync::Barrier;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn temp_paths(prefix: &str) -> (PathBuf, BaconLiteLogPaths) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
        let paths = BaconLiteLogPaths {
            machine: root.join("bacon-lite.jsonl"),
            human: root.join("bacon-lite.md"),
        };
        (root, paths)
    }

    fn event(sequence: usize) -> BaconLiteEvent {
        let task = Task::new(format!("process evidence {sequence}"), "test");
        build_bacon_lite_event(
            "test-crate",
            if sequence.is_multiple_of(2) {
                "even"
            } else {
                "odd"
            },
            &task,
            serde_json::json!({"sequence": sequence}),
        )
        .expect("event")
    }

    #[test]
    fn bacon_lite_validate_returns_confidence_and_triads() {
        let task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        let result = bacon_lite_validate(&task);
        assert!(result.confidence >= 0.0);
        assert_eq!(result.mode, "bacon_lite");
    }

    #[test]
    fn safe_description_does_not_change_governance_scoring() {
        let task = Task::new(
            "private prompt with evidence https://example.com",
            "dispatch",
        );
        let original =
            build_bacon_lite_event("manwe", "route", &task, Value::Null).expect("original event");
        let redacted = build_bacon_lite_event_with_description(
            "manwe",
            "route",
            &task,
            "route prompt=[redacted]".to_string(),
            Value::Null,
        )
        .expect("redacted event");

        assert_eq!(redacted.description, "route prompt=[redacted]");
        assert_eq!(redacted.passed, original.passed);
        assert_eq!(redacted.confidence, original.confidence);
        assert_eq!(redacted.triad_passed, original.triad_passed);
    }

    #[test]
    fn synchronous_adapter_writes_machine_and_human_logs() {
        let (root, paths) = temp_paths("bacon-lite-sync");
        let task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        let recorded = record_bacon_lite_to(
            "athena",
            "ingest",
            &task,
            serde_json::json!({"source":"example"}),
            &paths,
        )
        .expect("recorded");
        assert!(fs::read_to_string(&paths.machine)
            .expect("machine log")
            .contains(&recorded.task_id));
        assert!(fs::read_to_string(&paths.human)
            .expect("human log")
            .contains("Bacon-Lite Validation Log"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn burst_is_batched_and_flushed() {
        let (root, paths) = temp_paths("bacon-lite-burst");
        let writer = BaconLiteWriter::start(
            paths.clone(),
            BaconLiteWriterConfig {
                queue_capacity: 512,
                batch_size: 64,
                ..BaconLiteWriterConfig::default()
            },
        )
        .expect("writer");
        for sequence in 0..400 {
            writer.try_enqueue(event(sequence)).expect("enqueue");
        }
        writer.flush().expect("flush");
        assert_eq!(writer.counters().written_events, 400);
        assert_eq!(
            fs::read_to_string(&paths.machine).unwrap().lines().count(),
            400
        );
        writer.shutdown().expect("shutdown");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn saturation_and_hot_path_latency_are_visible() {
        let (root, paths) = temp_paths("bacon-lite-saturation");
        fs::create_dir_all(paths.machine.parent().unwrap()).unwrap();
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(paths.machine.with_extension("jsonl.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();
        let writer = BaconLiteWriter::start(
            paths,
            BaconLiteWriterConfig {
                queue_capacity: 1,
                batch_size: 1,
                ..BaconLiteWriterConfig::default()
            },
        )
        .unwrap();
        writer.try_enqueue(event(0)).unwrap();
        thread::sleep(Duration::from_millis(20));
        writer.try_enqueue(event(1)).unwrap();
        let started = Instant::now();
        assert_eq!(
            writer.try_enqueue(event(2)),
            Err(LedgerEnqueueError::Saturated)
        );
        assert!(started.elapsed() < Duration::from_millis(50));
        assert_eq!(writer.counters().dropped_events, 1);
        FileExt::unlock(&lock).unwrap();
        writer.shutdown().unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn disk_errors_increment_failed_and_error_counters() {
        let (root, mut paths) = temp_paths("bacon-lite-disk-error");
        fs::create_dir_all(&root).unwrap();
        let blocker = root.join("not-a-directory");
        fs::write(&blocker, b"block").unwrap();
        paths.machine = blocker.join("ledger.jsonl");
        paths.human = root.join("human.md");
        let writer = BaconLiteWriter::start(paths, BaconLiteWriterConfig::default()).unwrap();
        writer.try_enqueue(event(0)).unwrap();
        assert!(writer.flush().is_err());
        let counters = writer.counters();
        assert_eq!(counters.failed_events, 1);
        assert_eq!(counters.write_errors, 1);
        let _ = writer.shutdown();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_recovers_by_appending_to_existing_ledger() {
        let (root, paths) = temp_paths("bacon-lite-restart");
        let first =
            BaconLiteWriter::start(paths.clone(), BaconLiteWriterConfig::default()).unwrap();
        first.try_enqueue(event(0)).unwrap();
        first.shutdown().unwrap();
        let second =
            BaconLiteWriter::start(paths.clone(), BaconLiteWriterConfig::default()).unwrap();
        second.try_enqueue(event(1)).unwrap();
        second.shutdown().unwrap();
        assert_eq!(
            fs::read_to_string(paths.machine).unwrap().lines().count(),
            2
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_producers_preserve_every_record() {
        let (root, paths) = temp_paths("bacon-lite-concurrent");
        let writer = BaconLiteWriter::start(
            paths.clone(),
            BaconLiteWriterConfig {
                queue_capacity: 1_024,
                ..BaconLiteWriterConfig::default()
            },
        )
        .unwrap();
        let barrier = Arc::new(Barrier::new(5));
        let mut threads = Vec::new();
        for producer in 0..4 {
            let writer = writer.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                barrier.wait();
                for sequence in 0..100 {
                    writer
                        .try_enqueue(event(producer * 100 + sequence))
                        .unwrap();
                }
            }));
        }
        barrier.wait();
        for producer in threads {
            producer.join().unwrap();
        }
        writer.shutdown().unwrap();
        assert_eq!(
            fs::read_to_string(paths.machine).unwrap().lines().count(),
            400
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rotation_enforces_retention() {
        let (root, paths) = temp_paths("bacon-lite-rotation");
        let writer = BaconLiteWriter::start(
            paths.clone(),
            BaconLiteWriterConfig {
                batch_size: 1,
                max_file_bytes: 600,
                retained_files: 2,
                ..BaconLiteWriterConfig::default()
            },
        )
        .unwrap();
        for sequence in 0..8 {
            writer.try_enqueue(event(sequence)).unwrap();
        }
        writer.shutdown().unwrap();
        assert!(rotated_path(&paths.machine, 1).exists());
        assert!(rotated_path(&paths.machine, 2).exists());
        assert!(!rotated_path(&paths.machine, 3).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn summary_aggregates_mixed_versions_and_malformed_records() {
        let (root, paths) = temp_paths("bacon-lite-summary");
        fs::create_dir_all(&root).unwrap();
        let mut first = event(0);
        first.passed = true;
        first.confidence = 0.8;
        first.scorer_version = "scorer-v1".to_string();
        first.aurelius_outcome = Some(GateOutcome::Pass);
        let mut second = event(2);
        second.passed = false;
        second.confidence = 0.4;
        second.scorer_version = "scorer-v2".to_string();
        second.aurelius_outcome = Some(GateOutcome::Fail);
        let mut file = File::create(&paths.machine).unwrap();
        writeln!(file, "{}", serde_json::to_string(&first).unwrap()).unwrap();
        writeln!(file, "malformed").unwrap();
        writeln!(file, "{}", serde_json::to_string(&second).unwrap()).unwrap();
        let latest = read_latest_bacon_lite_event(&paths.machine, true)
            .unwrap()
            .expect("latest event");
        assert_eq!(latest.scorer_version, "scorer-v2");
        let summary =
            read_bacon_lite_summary(&paths.machine, &BaconLiteReadWindow::default()).unwrap();
        let aggregate = &summary.groups["test-crate"]["even"];
        assert_eq!(summary.records, 2);
        assert_eq!(summary.malformed_records, 1);
        assert_eq!(aggregate.pass_rate, 0.5);
        assert!((aggregate.mean_confidence - 0.6).abs() < f64::EPSILON);
        assert_eq!(aggregate.scorer_versions["scorer-v1"], 1);
        assert_eq!(aggregate.scorer_versions["scorer-v2"], 1);
        assert_eq!(aggregate.lens_outcomes["aurelius"]["pass"], 1);
        assert_eq!(aggregate.lens_outcomes["aurelius"]["fail"], 1);
        let strict = BaconLiteReadWindow {
            malformed: MalformedLineBehavior::Fail,
            ..BaconLiteReadWindow::default()
        };
        assert!(read_bacon_lite_summary(&paths.machine, &strict).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
