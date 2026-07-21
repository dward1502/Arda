// sigil: REPAIR
// Governance/economics adapter traits and mock implementations.
//
// These traits let the adaptive routing tree call into governance and
// economics checks without depending on concrete runtime implementations.
// Tests use the mock adapters below; production wiring replaces them with
// real adapters at startup.

#![allow(dead_code)]

pub trait GovernanceAdapter: Send + Sync {
    fn validate_send(
        &self,
        route_task: &dyn std::any::Any,
    ) -> Result<GovernanceOutcome, GovernanceError>;
}

pub trait EconomicsAdapter: Send + Sync {
    fn check_budget(
        &self,
        route_task: &dyn std::any::Any,
    ) -> Result<EconomicsOutcome, EconomicsError>;
}

#[derive(Debug, Clone, Copy)]
pub struct GovernanceOutcome {
    pub passed: bool,
    pub method: &'static str,
    pub chain_id: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct EconomicsOutcome {
    pub allowed: bool,
    pub budget_risk: &'static str,
    pub cost_tier: &'static str,
}

#[derive(Debug, thiserror::Error, Debug)]
pub enum GovernanceError {
    #[error("governance unavailable: {0}")]
    Unavailable(String),
    #[error("governance rejected: {0}")]
    Rejected(String),
}

#[derive(Debug, thiserror::Error)]
pub enum EconomicsError {
    #[error("economics unavailable: {0}")]
    Unavailable(String),
    #[error("economics rejected: {0}")]
    Rejected(String),
}

pub struct MockGovernanceAdapter {
    pub default_outcome: GovernanceOutcome,
}

impl MockGovernanceAdapter {
    pub const fn new() -> Self {
        Self {
            default_outcome: GovernanceOutcome {
                passed: true,
                method: "mock_triad",
                chain_id: None,
            },
        }
    }
}

impl GovernanceAdapter for MockGovernanceAdapter {
    fn validate_send(
        &self,
        _route_task: &dyn std::any::Any,
    ) -> Result<GovernanceOutcome, GovernanceError> {
        Ok(self.default_outcome)
    }
}

pub struct MockEconomicsAdapter {
    pub default_outcome: EconomicsOutcome,
}

impl MockEconomicsAdapter {
    pub const fn new() -> Self {
        Self {
            default_outcome: EconomicsOutcome {
                allowed: true,
                budget_risk: "low",
                cost_tier: "free_or_low_cost",
            },
        }
    }
}

impl EconomicsAdapter for MockEconomicsAdapter {
    fn check_budget(
        &self,
        _route_task: &dyn std::any::Any,
    ) -> Result<EconomicsOutcome, EconomicsError> {
        Ok(self.default_outcome)
    }
}
