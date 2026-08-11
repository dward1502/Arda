mod console;
mod constants;
mod device;
mod environment;
mod first_run;
mod guided;
mod helpers;
mod io;
mod prerequisites;
mod private_config;
mod provider;
mod readiness;
pub mod registry;
pub mod service_plan;
pub mod types;

pub use console::launch_console;
pub use device::device_scan;
pub use environment::{build_environment_profile, workspace_root};
pub use first_run::build_first_run_projection;
pub use guided::build_guided_session;
pub use helpers::now_utc;
pub use io::{
    build_proposed_config, onboarding_run_dir, read_json_optional, write_json,
    write_onboarding_receipt, write_profile, write_readiness,
};
pub use prerequisites::build_prerequisite_report;
pub use private_config::{
    apply_private_config_baseline, build_operator_answers_template, build_private_config_stage,
    parse_operator_answers, write_private_config_stage,
};
pub use provider::provider_checklist;
pub use readiness::{build_readiness_projection, l3_readiness_onboarding_checklist};
pub use registry::{check_registry, load_registry, registry_track_ids};
pub use service_plan::{
    apply_service_plan, build_approval_template, build_service_plan, parse_approval_receipt,
};
pub use types::*;

#[cfg(test)]
mod tests;
