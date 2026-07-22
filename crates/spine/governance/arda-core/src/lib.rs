// sigil: REPAIR
pub mod agent;
pub mod aipkg;
pub mod background;
pub mod config;
pub mod contract;
pub mod daemon;
pub mod error;
pub mod governance;
pub mod governance_gates;
pub mod learning;
pub mod ledger;
pub mod llm;
pub mod loop_alerts;
pub mod loop_economy;
pub mod loop_engine;
pub mod message;
pub mod pipeline;
pub mod router;
pub mod orome_runtime;
pub mod service_registry;
pub mod soterion;
pub mod soterion_watcher;
pub mod state;
pub mod systemd;
pub mod task;
pub mod tool;
pub mod tool_contract;

pub use agent::Agent;
pub use aipkg::{AipkgGovernance, AipkgManifest, AipkgPreflight, AipkgReceiptPolicy};
pub use background::{spawn_bounded_background, try_run_bounded, try_run_bounded_async};
pub use config::Config;
pub use contract::CONTRACT_VERSION;
pub use daemon::{CommandEnvelope, ResponseEnvelope};
pub use error::{ArdaError, Result};
pub use ledger::Ledger;
pub use llm::{
    ChatMessage, ChatRequest, ChatResponse, LlmConfig, LlmProvider, OpenAiCompatibleProvider,
};
pub use message::Message;
pub use orome_runtime::{
    AgentRegistryState, OromeCoreRuntimeState, SharedRegistryStateStorage, SharedRouterStateStorage,
    OromeRuntimeStateError,
};
pub use service_registry::{
    ArdaServiceRegistryStatus, ContinuityConfig, ContractConfig, GovernanceConfig, RegistryError,
    ServiceContract, ServiceHandle, ServiceKind, ServiceRecord, ServiceRegistry,
    ServiceRegistryState, ServiceRegistryStateValidator, ServiceStatus,
};
pub use soterion::{
    default_soterion_registry_path, file_sigil_name_from_registry, load_default_soterion_registry,
    load_soterion_registry, machine_sigil_from_registry, machine_sigil_or_default,
    SoterionConfidenceEntry, SoterionFileSigilEntry, SoterionGlyphEntry, SoterionIndex,
    SoterionMachineSigil, SoterionMeta, SoterionRegistry, SoterionRegistryEntry, SIGIL_DICTIONARY,
};
pub use soterion_watcher::SoterionWatcher;
pub use systemd::{parse_list_units, SystemctlClient, SystemdClient, SystemdError, Unit, UnitKind};
pub use task::{JouleWorkMeasurementSource, Task, TaskId, TaskStatus};
pub use tool::ToolRegistry;
