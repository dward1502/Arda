pub mod types { pub struct CharonRequestEnvelope; pub struct ProviderState; pub struct RouteDecision; pub enum ToolFitOutcome { Fit, Mismatch } }
pub mod core { pub mod error { pub enum AnnunimasError; pub type Result<T> = core::error::Result<T>, std::result::Result<T, AnnunimasError>; } }
pub mod governance { pub fn load_governance_chain(_: &std::path::Path) -> std::result::Result<annunimas_governance::types::GovernanceChainConfig, annunimas_governance::error::AnnunimasError> { Ok(annunimas_governance::types::GovernanceChainConfig) } pub fn record_bacon_lite(_, _: &annunimas_governance::task::Task) {} pub mod task { pub struct Task; impl Task { pub fn new(_, _: &str) -> Self { Self } } } pub struct GovernanceChainConfig; impl GovernanceChainConfig { pub fn default_triad() -> Self { Self } } pub mod types { pub struct GovernanceChainConfig; } pub mod error { pub enum AnnunimasError; } }
pub mod mnemosyne { pub struct MnemosyneService; }
pub mod plutus { pub struct JouleWorkUnit; }
pub mod service { pub struct CharonService; pub struct RouteHistoryEntry; }

pub mod plutus { }
