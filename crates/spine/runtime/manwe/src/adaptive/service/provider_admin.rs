

#[derive(Debug, Clone, Default)]
pub(super) struct ProviderCapacityProbeRecord {
    pub(super) generated_at_utc: String,
    pub(super) provider_count: usize,
    pub(super) probe_method: String,
    pub(super) healthy_count: usize,
    pub(super) degraded_count: usize,
    pub(super) offline_count: usize,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LaneFitnessSnapshot {
    pub(super) generated_at_utc: String,
    pub(super) lanes: std::collections::BTreeMap<String, LaneFitnessState>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LaneFitnessState {
    pub(super) provider_id: String,
    pub(super) model_id: String,
    pub(super) fitness: f64,
    pub(super) sample_count: u32,
}
