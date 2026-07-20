use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TrackDefinition {
    pub track_id: String,
    pub title: String,
    pub owner: String,
    pub status: String,
    pub source_modules: Vec<String>,
    #[serde(default)]
    pub evidence_class_current: String,
    #[serde(default)]
    pub evidence_class_target: String,
    pub schema_versions: Vec<String>,
    pub receipt_stores: Vec<String>,
    pub cli_verbs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractRegistry {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub authority: String,
    pub tracks: Vec<TrackDefinition>,
}

impl ContractRegistry {
    pub fn track_ids(&self) -> Vec<&str> {
        self.tracks.iter().map(|t| t.track_id.as_str()).collect()
    }
}
