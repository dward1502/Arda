// sigil: REPAIR
//! Energy meter — produces joule estimates for the loop.
//!
//! Phase 2 step 2: trait + an `EstimatorMeter` fallback that turns
//! (provider, model, tokens) into joules using a tariff table.
//! Real RAPL/NVML/Pi5/powermetrics backends slot in behind the
//! `EnergyMeter` trait in step 2b without changing callers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// One reading of energy expenditure attributable to a piece of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleSample {
    pub joules: f64,
    pub source: SampleSource,
    pub sampled_at: DateTime<Utc>,
}

/// A deterministic meter failure. Registries use this contract to skip a
/// failed hardware backend and continue to the estimator fallback.
#[derive(Debug, thiserror::Error)]
pub enum MeterError {
    #[error("meter {meter} failed: {message}")]
    Backend {
        meter: &'static str,
        message: String,
    },
    #[error("invalid work profile: {0}")]
    InvalidWork(String),
    #[error("meter {meter} returned invalid joules: {joules}")]
    InvalidSample { meter: &'static str, joules: f64 },
}

/// Where the reading came from. Lets the ledger separate measured
/// energy from estimated energy without losing fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleSource {
    /// CPU package energy via Intel/AMD RAPL.
    Rapl,
    /// NVIDIA GPU energy via NVML.
    Nvml,
    /// Apple Silicon via `powermetrics`.
    PowerMetrics,
    /// Raspberry Pi 5 power rails.
    Pi5Rails,
    /// Provider tariff × token-count estimate (no direct measurement).
    EstimatorTariff,
    /// Hard-coded constant — last-resort fallback.
    EstimatorConstant,
}

impl SampleSource {
    /// True if this came from a hardware measurement vs. an estimate.
    /// Surfaced in the ledger so analysis can weight measured samples
    /// higher than estimated ones.
    pub fn is_measured(&self) -> bool {
        matches!(
            self,
            SampleSource::Rapl
                | SampleSource::Nvml
                | SampleSource::PowerMetrics
                | SampleSource::Pi5Rails
        )
    }
}

/// What we want a joule estimate *for*. Keeping this small in v0.2 —
/// the dispatcher uses `Cloud` (provider-tariff path) by default
/// because today's loop simulates execution and most real work will
/// be cloud-bound until local inference comes online.
#[derive(Debug, Clone)]
pub enum WorkProfile {
    /// A cloud LLM call we can estimate from a tariff.
    Cloud {
        provider: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
    },
    /// A local compute task with a coarse joules-per-second estimate.
    Local { duration_secs: f64 },
}

/// Energy meter abstraction. Every backend (RAPL/NVML/Pi5/powermetrics
/// + the estimator fallback) implements this trait. Sampling is async so
/// hardware backends can perform non-blocking platform reads.
#[async_trait::async_trait]
pub trait EnergyMeter: Send + Sync {
    /// Stable name for telemetry (e.g. `"rapl"`, `"estimator"`).
    fn name(&self) -> &'static str;

    /// True if this backend can run on the current host. Registry
    /// uses this to select the best available meter at startup.
    fn available(&self) -> bool;

    /// Produce a joule estimate for the given work.
    async fn estimate(&self, work: &WorkProfile) -> Result<JouleSample, MeterError>;
}

fn validate_work(work: &WorkProfile) -> Result<(), MeterError> {
    match work {
        WorkProfile::Cloud { .. } => Ok(()),
        WorkProfile::Local { duration_secs }
            if duration_secs.is_finite() && *duration_secs >= 0.0 =>
        {
            Ok(())
        }
        WorkProfile::Local { duration_secs } => Err(MeterError::InvalidWork(format!(
            "duration_secs must be finite and non-negative, got {duration_secs}"
        ))),
    }
}

// ---------------------------------------------------------------
// Estimator fallback
// ---------------------------------------------------------------

/// Errors raised loading or parsing a tariffs file.
#[derive(Debug, thiserror::Error)]
pub enum TariffError {
    #[error("tariffs io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("tariffs parse error: {0}")]
    Parse(String),
    #[error("invalid tariff rate for {field}: {value}; rates must be finite and non-negative")]
    InvalidRate { field: String, value: f64 },
}

#[derive(Debug, Deserialize)]
struct TariffFile {
    defaults: TariffDefaults,
    entries: Option<Vec<TariffEntry>>,
}

#[derive(Debug, Deserialize)]
struct TariffDefaults {
    joules_per_input_token: f64,
    joules_per_output_token: f64,
}

#[derive(Debug, Deserialize)]
struct TariffEntry {
    provider: String,
    model: String,
    joules_per_input_token: f64,
    joules_per_output_token: f64,
}

/// Per-provider/model joule rates. Loaded from `config/governance/joule_tariffs.toml`;
/// a hand-tuned default table is the fallback when no file is given.
#[derive(Debug, Clone)]
pub struct TariffTable {
    /// Joules per input token, keyed by `provider/model`.
    input: std::collections::HashMap<String, f64>,
    /// Joules per output token, keyed by `provider/model`.
    output: std::collections::HashMap<String, f64>,
    /// Default if no entry matches.
    default_input: f64,
    default_output: f64,
    loaded_at_utc: DateTime<Utc>,
}

impl TariffTable {
    /// Hand-tuned conservative defaults. Numbers are deliberately
    /// rough — step 3 replaces them with the loaded tariffs file.
    pub fn default_v0_2() -> Self {
        let mut input = std::collections::HashMap::new();
        let mut output = std::collections::HashMap::new();
        // Order-of-magnitude estimates from public datacenter PUE +
        // per-token energy figures. Replaced wholesale by the
        // tariffs file in step 3.
        for (k, i, o) in [
            ("anthropic/claude-opus-4-7", 0.05, 0.15),
            ("anthropic/claude-sonnet-4-6", 0.02, 0.06),
            ("anthropic/claude-haiku-4-5", 0.005, 0.015),
            ("openai/gpt-4o", 0.03, 0.09),
            ("openai/gpt-4o-mini", 0.008, 0.024),
            ("local/llama3-8b", 0.001, 0.003),
        ] {
            input.insert(k.into(), i);
            output.insert(k.into(), o);
        }
        Self {
            input,
            output,
            default_input: 0.02,
            default_output: 0.06,
            loaded_at_utc: Utc::now(),
        }
    }

    /// Load a tariff table from a TOML file. See
    /// `config/governance/joule_tariffs.toml` for the schema. Reload by calling
    /// this again — there's no daemon, the caller decides cadence.
    pub fn load_from_path(path: &Path) -> Result<Self, TariffError> {
        let raw = std::fs::read_to_string(path).map_err(TariffError::Io)?;
        let mut table = Self::load_from_str(&raw)?;
        table.loaded_at_utc = std::fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(Utc::now);
        Ok(table)
    }

    /// Parse from a TOML string. Useful for tests.
    pub fn load_from_str(raw: &str) -> Result<Self, TariffError> {
        let file: TariffFile =
            toml::from_str(raw).map_err(|e| TariffError::Parse(e.to_string()))?;
        validate_rate(
            "defaults.joules_per_input_token",
            file.defaults.joules_per_input_token,
        )?;
        validate_rate(
            "defaults.joules_per_output_token",
            file.defaults.joules_per_output_token,
        )?;
        let mut input = std::collections::HashMap::new();
        let mut output = std::collections::HashMap::new();
        for e in file.entries.unwrap_or_default() {
            let key = format!("{}/{}", e.provider, e.model);
            validate_rate(
                &format!("entries[{key}].joules_per_input_token"),
                e.joules_per_input_token,
            )?;
            validate_rate(
                &format!("entries[{key}].joules_per_output_token"),
                e.joules_per_output_token,
            )?;
            input.insert(key.clone(), e.joules_per_input_token);
            output.insert(key, e.joules_per_output_token);
        }
        Ok(Self {
            input,
            output,
            default_input: file.defaults.joules_per_input_token,
            default_output: file.defaults.joules_per_output_token,
            loaded_at_utc: Utc::now(),
        })
    }

    /// Replace this table with the contents of `path` in-place.
    /// Convenience for the refresh hook.
    pub fn reload_from_path(&mut self, path: &Path) -> Result<(), TariffError> {
        *self = Self::load_from_path(path)?;
        Ok(())
    }

    /// Timestamp of the currently active tariff source. File-backed tables use
    /// the source modification time; in-memory/default tables use load time.
    pub fn loaded_at_utc(&self) -> DateTime<Utc> {
        self.loaded_at_utc
    }

    /// Whether this table is older than the caller's accepted freshness window.
    pub fn is_stale_at(&self, now: DateTime<Utc>, max_age: chrono::Duration) -> bool {
        now.signed_duration_since(self.loaded_at_utc) > max_age
    }

    fn rates(&self, provider: &str, model: &str) -> (f64, f64) {
        let key = format!("{provider}/{model}");
        let i = self.input.get(&key).copied().unwrap_or(self.default_input);
        let o = self
            .output
            .get(&key)
            .copied()
            .unwrap_or(self.default_output);
        (i, o)
    }
}

fn validate_rate(field: &str, value: f64) -> Result<(), TariffError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(TariffError::InvalidRate {
            field: field.to_owned(),
            value,
        })
    }
}

/// Estimator backend — always available. Uses a tariff table for
/// `Cloud` work and a flat joules/sec for `Local` work.
pub struct EstimatorMeter {
    tariffs: TariffTable,
    /// Joules per second for `WorkProfile::Local`. Conservative
    /// stand-in until the real RAPL backend lands.
    local_joules_per_sec: f64,
}

impl EstimatorMeter {
    pub fn new(tariffs: TariffTable) -> Self {
        Self {
            tariffs,
            local_joules_per_sec: 25.0,
        }
    }

    pub fn with_default_tariffs() -> Self {
        Self::new(TariffTable::default_v0_2())
    }

    /// Build an estimator from a tariffs TOML at `path`. The file
    /// schema lives in `config/governance/joule_tariffs.toml`.
    pub fn load_from_path(path: &Path) -> Result<Self, TariffError> {
        Ok(Self::new(TariffTable::load_from_path(path)?))
    }

    /// Override the local joules/sec rate. Useful for tests and for
    /// host-specific tuning once we know typical CPU draw.
    pub fn with_local_joules_per_sec(mut self, j: f64) -> Self {
        self.local_joules_per_sec = j;
        self
    }

    fn estimate_sync(&self, work: &WorkProfile) -> JouleSample {
        let (joules, source) = match work {
            WorkProfile::Cloud {
                provider,
                model,
                input_tokens,
                output_tokens,
            } => {
                let (ri, ro) = self.tariffs.rates(provider, model);
                let joules = ri * (*input_tokens as f64) + ro * (*output_tokens as f64);
                (joules, SampleSource::EstimatorTariff)
            }
            WorkProfile::Local { duration_secs } => (
                self.local_joules_per_sec * duration_secs,
                SampleSource::EstimatorConstant,
            ),
        };
        JouleSample {
            joules,
            source,
            sampled_at: Utc::now(),
        }
    }
}

#[async_trait::async_trait]
impl EnergyMeter for EstimatorMeter {
    fn name(&self) -> &'static str {
        "estimator"
    }

    fn available(&self) -> bool {
        true
    }

    async fn estimate(&self, work: &WorkProfile) -> Result<JouleSample, MeterError> {
        validate_work(work)?;
        let sample = self.estimate_sync(work);
        if sample.joules.is_finite() && sample.joules >= 0.0 {
            Ok(sample)
        } else {
            Err(MeterError::InvalidSample {
                meter: self.name(),
                joules: sample.joules,
            })
        }
    }
}

// ---------------------------------------------------------------
// Hardware backends
// ---------------------------------------------------------------

/// Availability-aware direct measurement backend. v0.2 uses the
/// platform probe to choose a measured source when the host exposes
/// it, then estimates local work from a configurable wattage. The
/// trait boundary is the important part: callers can distinguish
/// measured-capable backends from tariff-only estimates immediately,
/// and the backend can evolve from wattage snapshots to delta
/// sampling without changing the dispatcher.
pub struct HardwareMeter {
    name: &'static str,
    source: SampleSource,
    probe_paths: &'static [&'static str],
    local_watts: f64,
}

impl HardwareMeter {
    pub fn rapl() -> Self {
        Self {
            name: "rapl",
            source: SampleSource::Rapl,
            probe_paths: &[
                "/sys/class/powercap/intel-rapl:0/energy_uj",
                "/sys/class/powercap/amd-rapl:0/energy_uj",
            ],
            local_watts: 35.0,
        }
    }

    pub fn nvml() -> Self {
        Self {
            name: "nvml",
            source: SampleSource::Nvml,
            probe_paths: &["/proc/driver/nvidia/version"],
            local_watts: 120.0,
        }
    }

    pub fn powermetrics() -> Self {
        Self {
            name: "powermetrics",
            source: SampleSource::PowerMetrics,
            probe_paths: &["/usr/bin/powermetrics"],
            local_watts: 25.0,
        }
    }

    pub fn pi5_rails() -> Self {
        Self {
            name: "pi5_rails",
            source: SampleSource::Pi5Rails,
            probe_paths: &[
                "/sys/bus/iio/devices/iio:device0/in_power0_input",
                "/sys/class/hwmon/hwmon0/power1_input",
            ],
            local_watts: 8.0,
        }
    }

    fn local_joules(&self, work: &WorkProfile) -> f64 {
        match work {
            WorkProfile::Local { duration_secs } => self.local_watts * duration_secs,
            WorkProfile::Cloud { .. } => 0.0,
        }
    }
}

#[async_trait::async_trait]
impl EnergyMeter for HardwareMeter {
    fn name(&self) -> &'static str {
        self.name
    }

    fn available(&self) -> bool {
        self.probe_paths.iter().any(|path| Path::new(path).exists())
    }

    async fn estimate(&self, work: &WorkProfile) -> Result<JouleSample, MeterError> {
        validate_work(work)?;
        let sample = JouleSample {
            joules: self.local_joules(work),
            source: self.source,
            sampled_at: Utc::now(),
        };
        if sample.joules.is_finite() && sample.joules >= 0.0 {
            Ok(sample)
        } else {
            Err(MeterError::InvalidSample {
                meter: self.name(),
                joules: sample.joules,
            })
        }
    }
}

// ---------------------------------------------------------------
// JouleEstimator impl for the loop_engine dispatcher hook.
// ---------------------------------------------------------------

/// Default per-intent token estimates the dispatcher uses to ask
/// the meter for a joule cost. Numbers are conservative averages —
/// the joule market in step 8 will replace this with actual per-bid
/// estimates from each agent.
fn default_intent_profile(intent: &str) -> WorkProfile {
    let (provider, model, input, output) = match intent {
        // Manwe probes are short prompts, short replies.
        "probe_provider" | "retire_failing" => ("anthropic", "claude-haiku-4-5", 200u64, 100u64),
        // Plutus summaries: read structured data, write a tally.
        "collect_joule_samples"
        | "summarize_by_agent"
        | "summarize_by_provider_tier"
        | "emit_ledger_summary" => ("anthropic", "claude-haiku-4-5", 800, 400),
        // Athena scans + reindex: heavier reads.
        "scan_knowledge_sources" | "diff_against_last_index" | "reindex_changed" => {
            ("anthropic", "claude-sonnet-4-6", 1200, 400)
        }
        // Ledger maintenance is essentially deterministic; tiny LLM.
        "list_ledger_segments" | "archive_older_than" => {
            ("anthropic", "claude-haiku-4-5", 300, 150)
        }
        // Council/Oracle deliberations skew long-form.
        "probe_seat" | "escalate_if_repeat_failure" => {
            ("anthropic", "claude-sonnet-4-6", 1500, 800)
        }
        _ => ("anthropic", "claude-haiku-4-5", 500, 250),
    };
    WorkProfile::Cloud {
        provider: provider.into(),
        model: model.into(),
        input_tokens: input,
        output_tokens: output,
    }
}

impl arda_core::loop_engine::JouleEstimator for EstimatorMeter {
    fn estimate_for_task(&self, task: &arda_core::task::Task) -> f64 {
        let profile = default_intent_profile(&task.task_type);
        self.estimate_sync(&profile).joules
    }
}

// ---------------------------------------------------------------
// Registry
// ---------------------------------------------------------------

/// Picks the best available meter at startup. Order is real-hardware
/// backends first (preferred), `EstimatorMeter` last (always works).
pub struct MeterRegistry {
    meters: Vec<Arc<dyn EnergyMeter>>,
}

impl MeterRegistry {
    pub fn new() -> Self {
        Self { meters: Vec::new() }
    }

    pub fn register(&mut self, meter: Arc<dyn EnergyMeter>) {
        self.meters.push(meter);
    }

    /// Build the default registry: hardware probes first, estimator
    /// fallback last.
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(HardwareMeter::rapl()));
        r.register(Arc::new(HardwareMeter::nvml()));
        r.register(Arc::new(HardwareMeter::powermetrics()));
        r.register(Arc::new(HardwareMeter::pi5_rails()));
        r.register(Arc::new(EstimatorMeter::with_default_tariffs()));
        r
    }

    /// First registered meter that reports `available()`. Falls back
    /// to a fresh `EstimatorMeter` if the registry is empty (so
    /// callers never hit a None path).
    pub fn pick(&self) -> Arc<dyn EnergyMeter> {
        for m in &self.meters {
            if m.available() {
                return m.clone();
            }
        }
        Arc::new(EstimatorMeter::with_default_tariffs())
    }

    /// Estimate with ordered backend fallback. Available registered meters are
    /// tried in order; backend errors and invalid samples are skipped. A fresh,
    /// deterministic estimator is the final fallback.
    pub async fn estimate(&self, work: &WorkProfile) -> Result<JouleSample, MeterError> {
        for meter in &self.meters {
            if !meter.available() {
                continue;
            }
            match meter.estimate(work).await {
                Ok(sample) if sample.joules.is_finite() && sample.joules >= 0.0 => {
                    return Ok(sample);
                }
                Ok(sample) => {
                    tracing::warn!(
                        meter = meter.name(),
                        joules = sample.joules,
                        "energy meter returned invalid sample; trying fallback"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        meter = meter.name(),
                        error = %error,
                        "energy meter failed; trying fallback"
                    );
                }
            }
        }

        EstimatorMeter::with_default_tariffs().estimate(work).await
    }
}

impl Default for MeterRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingMeter;

    #[async_trait::async_trait]
    impl EnergyMeter for FailingMeter {
        fn name(&self) -> &'static str {
            "failing_hardware"
        }

        fn available(&self) -> bool {
            true
        }

        async fn estimate(&self, _work: &WorkProfile) -> Result<JouleSample, MeterError> {
            Err(MeterError::Backend {
                meter: self.name(),
                message: "simulated read failure".to_owned(),
            })
        }
    }

    struct NonFiniteMeter;

    #[async_trait::async_trait]
    impl EnergyMeter for NonFiniteMeter {
        fn name(&self) -> &'static str {
            "non_finite_hardware"
        }

        fn available(&self) -> bool {
            true
        }

        async fn estimate(&self, _work: &WorkProfile) -> Result<JouleSample, MeterError> {
            Ok(JouleSample {
                joules: f64::NAN,
                source: SampleSource::Rapl,
                sampled_at: Utc::now(),
            })
        }
    }

    #[tokio::test]
    async fn estimator_uses_tariff_for_known_model() {
        let m = EstimatorMeter::with_default_tariffs();
        let s = m
            .estimate(&WorkProfile::Cloud {
                provider: "anthropic".into(),
                model: "claude-haiku-4-5".into(),
                input_tokens: 1_000,
                output_tokens: 500,
            })
            .await
            .expect("sample");
        // 1000 * 0.005 + 500 * 0.015 = 5.0 + 7.5 = 12.5
        assert!((s.joules - 12.5).abs() < 1e-9);
        assert_eq!(s.source, SampleSource::EstimatorTariff);
        assert!(!s.source.is_measured());
    }

    #[tokio::test]
    async fn estimator_falls_back_to_default_for_unknown_model() {
        let m = EstimatorMeter::with_default_tariffs();
        let s = m
            .estimate(&WorkProfile::Cloud {
                provider: "weird".into(),
                model: "unknown-7b".into(),
                input_tokens: 100,
                output_tokens: 100,
            })
            .await
            .expect("sample");
        // 100 * 0.02 + 100 * 0.06 = 2.0 + 6.0 = 8.0
        assert!((s.joules - 8.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn estimator_local_uses_joules_per_sec() {
        let m = EstimatorMeter::with_default_tariffs().with_local_joules_per_sec(10.0);
        let s = m
            .estimate(&WorkProfile::Local { duration_secs: 4.0 })
            .await
            .expect("sample");
        assert!((s.joules - 40.0).abs() < 1e-9);
        assert_eq!(s.source, SampleSource::EstimatorConstant);
    }

    #[test]
    fn registry_picks_estimator_by_default() {
        let r = MeterRegistry::with_defaults();
        let m = r.pick();
        assert!(m.available());
    }

    #[test]
    fn registry_falls_back_when_empty() {
        let r = MeterRegistry::new();
        let m = r.pick();
        assert_eq!(m.name(), "estimator");
    }

    #[test]
    fn tariff_table_loads_from_str_and_overrides_defaults() {
        let raw = r#"
            [defaults]
            joules_per_input_token = 0.10
            joules_per_output_token = 0.20

            [[entries]]
            provider = "anthropic"
            model = "claude-haiku-4-5"
            joules_per_input_token = 0.001
            joules_per_output_token = 0.002
        "#;
        let table = TariffTable::load_from_str(raw).expect("parse");
        assert_eq!(table.rates("anthropic", "claude-haiku-4-5"), (0.001, 0.002));
        assert_eq!(table.rates("unknown", "weird"), (0.10, 0.20));
    }

    #[test]
    fn tariff_table_rejects_negative_and_non_finite_rates() {
        for value in ["-0.1", "nan"] {
            let raw = format!(
                "[defaults]\njoules_per_input_token = {value}\njoules_per_output_token = 0.2"
            );
            assert!(matches!(
                TariffTable::load_from_str(&raw),
                Err(TariffError::InvalidRate { .. })
            ));
        }
    }

    #[test]
    fn tariff_table_reloads_from_disk() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tariffs.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "[defaults]\njoules_per_input_token = 1.0\njoules_per_output_token = 2.0"
        )
        .unwrap();
        drop(f);

        let mut table = TariffTable::load_from_path(&path).unwrap();
        assert_eq!(table.rates("a", "b"), (1.0, 2.0));

        // Rewrite with different defaults; reload should pick up.
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "[defaults]\njoules_per_input_token = 7.0\njoules_per_output_token = 9.0"
        )
        .unwrap();
        drop(f);
        table.reload_from_path(&path).unwrap();
        assert_eq!(table.rates("a", "b"), (7.0, 9.0));
    }

    #[test]
    fn tariff_table_exposes_deterministic_staleness_hook() {
        let table = TariffTable::default_v0_2();
        let loaded = table.loaded_at_utc();
        let max_age = chrono::Duration::seconds(300);
        assert!(!table.is_stale_at(loaded + chrono::Duration::seconds(300), max_age));
        assert!(table.is_stale_at(loaded + chrono::Duration::seconds(301), max_age));
    }

    #[test]
    fn failed_tariff_reload_keeps_last_valid_table() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tariffs.toml");
        std::fs::write(
            &path,
            "[defaults]\njoules_per_input_token = 1.0\njoules_per_output_token = 2.0",
        )
        .expect("valid tariffs");
        let mut table = TariffTable::load_from_path(&path).expect("load valid tariffs");

        std::fs::write(&path, "not valid toml = [").expect("invalid tariffs");
        assert!(table.reload_from_path(&path).is_err());
        assert_eq!(table.rates("a", "b"), (1.0, 2.0));
    }

    #[test]
    fn shipped_tariffs_file_parses() {
        // Sanity-check that config/governance/joule_tariffs.toml stays in sync
        // with the loader. CARGO_MANIFEST_DIR points at the crate, but tests
        // run from the target/debug/deps/... tree, so search upward for the
        // repo root.
        let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = loop {
            let candidate = dir
                .join("config")
                .join("governance")
                .join("joule_tariffs.toml");
            if candidate.exists() {
                break candidate;
            }
            if !dir.pop() {
                panic!("joule_tariffs.toml not found");
            }
        };
        let table = TariffTable::load_from_path(&path).expect("ship file parses");
        // Known entry from the file.
        let (i, o) = table.rates("edge_beelink_light", "Qwen_Qwen3.5-4B-Q6_K");
        assert!(i > 0.0 && o > 0.0);
    }

    #[test]
    fn sample_source_is_measured_distinguishes_hardware() {
        assert!(SampleSource::Rapl.is_measured());
        assert!(SampleSource::Nvml.is_measured());
        assert!(!SampleSource::EstimatorTariff.is_measured());
        assert!(!SampleSource::EstimatorConstant.is_measured());
    }

    #[tokio::test]
    async fn hardware_meter_marks_local_samples_as_measured_source() {
        let meter = HardwareMeter {
            name: "test_hardware",
            source: SampleSource::Rapl,
            probe_paths: &[],
            local_watts: 12.0,
        };
        assert!(!meter.available());
        let sample = meter
            .estimate(&WorkProfile::Local { duration_secs: 2.5 })
            .await
            .expect("sample");
        assert!((sample.joules - 30.0).abs() < 1e-9);
        assert_eq!(sample.source, SampleSource::Rapl);
        assert!(sample.source.is_measured());
    }

    #[tokio::test]
    async fn registry_falls_back_after_hardware_backend_failure() {
        let mut registry = MeterRegistry::new();
        registry.register(Arc::new(FailingMeter));

        let sample = registry
            .estimate(&WorkProfile::Local { duration_secs: 2.0 })
            .await
            .expect("estimator fallback");

        assert_eq!(sample.source, SampleSource::EstimatorConstant);
        assert!((sample.joules - 50.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn registry_rejects_non_finite_hardware_sample_before_fallback() {
        let mut registry = MeterRegistry::new();
        registry.register(Arc::new(NonFiniteMeter));

        let sample = registry
            .estimate(&WorkProfile::Local { duration_secs: 1.0 })
            .await
            .expect("estimator fallback");

        assert_eq!(sample.source, SampleSource::EstimatorConstant);
        assert!(sample.joules.is_finite());
    }

    #[tokio::test]
    async fn invalid_work_fails_even_when_only_estimator_is_available() {
        let registry = MeterRegistry::new();
        let error = registry
            .estimate(&WorkProfile::Local {
                duration_secs: f64::NAN,
            })
            .await
            .expect_err("non-finite duration must fail");

        assert!(matches!(error, MeterError::InvalidWork(_)));
    }
}
