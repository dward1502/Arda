//! Explicit Workbench execution, with Engine-owned runtime and workspace admission.
//! The default build exposes only contracts; `runtime` enables HTTP and receipt I/O.
//! This crate has no dependency on Aule, metrics, or legacy queue scheduling.

pub mod execution_contract;
pub use execution_contract::*;

#[cfg(feature = "runtime")]
mod execution;
#[cfg(feature = "runtime")]
pub use execution::WorkbenchExecutionAdapter;

/// Compatibility operations shared with Aule's retired queue adapter.
#[doc(hidden)]
#[cfg(feature = "runtime")]
pub mod compatibility;
