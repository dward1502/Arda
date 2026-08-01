// sigil: REPAIR
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetAlert {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModelConfig {
    pub provider: String,
    pub input_rate: f64,
    pub output_rate: f64,
    pub batch_size: usize,
}

impl Default for CostModelConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_owned(),
            input_rate: 0.0015,
            output_rate: 0.002,
            batch_size: 1000,
        }
    }
}

pub trait CostModel: Send + Sync {
    fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64;
    fn provider(&self) -> &str;
}

pub struct LinearCostModel {
    provider: String,
    input_rate: f64,
    output_rate: f64,
}

impl LinearCostModel {
    pub fn new(provider: impl Into<String>, input_rate: f64, output_rate: f64) -> Self {
        Self {
            provider: provider.into(),
            input_rate,
            output_rate,
        }
    }

    pub fn from_config(config: &CostModelConfig) -> Self {
        Self::new(&config.provider, config.input_rate, config.output_rate)
    }
}

impl CostModel for LinearCostModel {
    fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        (input_tokens as f64 * self.input_rate) + (output_tokens as f64 * self.output_rate)
    }

    fn provider(&self) -> &str {
        &self.provider
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsEngine {
    models: HashMap<String, CostModelConfig>,
    daily_budget: f64,
    total_spend: f64,
}

impl EconomicsEngine {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            daily_budget: 100.0,
            total_spend: 0.0,
        }
    }

    pub fn with_budget(mut self, budget: f64) -> Self {
        self.daily_budget = budget;
        self
    }

    pub fn register_model(&mut self, config: CostModelConfig) {
        self.models.insert(config.provider.clone(), config);
    }

    pub fn calculate_cost(
        &self,
        provider: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> Option<f64> {
        self.models.get(provider).map(|config| {
            let model = LinearCostModel::from_config(config);
            model.calculate_cost(input_tokens, output_tokens)
        })
    }

    pub fn can_afford(&self, cost: f64) -> bool {
        self.total_spend + cost <= self.daily_budget
    }

    /// Record spend and return an alert only when a budget threshold is crossed.
    pub fn record_spend(&mut self, amount: f64) -> Option<BudgetAlert> {
        let previous = self.budget_alert();
        self.total_spend += amount;
        let current = self.budget_alert();
        (current != previous).then_some(current).flatten()
    }

    pub fn budget_remaining(&self) -> f64 {
        self.daily_budget - self.total_spend
    }

    pub fn budget_usage_percent(&self) -> f64 {
        if self.daily_budget > 0.0 {
            (self.total_spend / self.daily_budget) * 100.0
        } else if self.total_spend > 0.0 {
            100.0
        } else {
            0.0
        }
    }

    pub fn budget_alert(&self) -> Option<BudgetAlert> {
        let usage = self.budget_usage_percent();
        if usage >= 100.0 {
            Some(BudgetAlert::Critical)
        } else if usage >= 80.0 {
            Some(BudgetAlert::Warning)
        } else {
            None
        }
    }

    pub fn reset_daily(&mut self) {
        self.total_spend = 0.0;
    }

    pub fn providers(&self) -> Vec<&String> {
        self.models.keys().collect()
    }

    pub fn status_snapshot(&self) -> serde_json::Value {
        let mut providers = self.models.keys().cloned().collect::<Vec<_>>();
        providers.sort();
        json!({
            "daily_budget": self.daily_budget,
            "total_spend": self.total_spend,
            "budget_remaining": self.budget_remaining(),
            "budget_usage_percent": self.budget_usage_percent(),
            "budget_alert": self.budget_alert(),
            "providers": providers,
        })
    }

    pub fn restore_from_snapshot(&mut self, snapshot: &serde_json::Value) {
        self.daily_budget = snapshot
            .get("daily_budget")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.daily_budget);
        self.total_spend = snapshot
            .get("total_spend")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.total_spend);
    }
}

impl Default for EconomicsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl arda_core::governance_gates::AffordabilityPolicy for EconomicsEngine {
    fn policy_name(&self) -> &'static str {
        "plutus_daily_budget"
    }

    fn can_afford(&self, estimated_cost: f64) -> bool {
        EconomicsEngine::can_afford(self, estimated_cost)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROIMetrics {
    pub investment: f64,
    pub return_amount: f64,
    pub period_hours: f64,
    pub roi_percent: f64,
}

impl ROIMetrics {
    pub fn calculate(investment: f64, return_amount: f64, period_hours: f64) -> Self {
        let roi_percent = if investment > 0.0 {
            ((return_amount - investment) / investment) * 100.0
        } else {
            0.0
        };

        Self {
            investment,
            return_amount,
            period_hours,
            roi_percent,
        }
    }

    pub fn annualized(&self) -> f64 {
        if self.period_hours > 0.0 {
            self.roi_percent * (24.0 * 365.0 / self.period_hours)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetAlert, EconomicsEngine, ROIMetrics};
    use arda_core::governance_gates::GovernanceGates;

    #[test]
    fn record_spend_emits_only_threshold_crossings() {
        let mut engine = EconomicsEngine::new().with_budget(100.0);
        assert_eq!(engine.record_spend(79.0), None);
        assert_eq!(engine.record_spend(1.0), Some(BudgetAlert::Warning));
        assert_eq!(engine.record_spend(5.0), None);
        assert_eq!(engine.record_spend(15.0), Some(BudgetAlert::Critical));
    }

    #[test]
    fn zero_budget_usage_is_finite() {
        let mut engine = EconomicsEngine::new().with_budget(0.0);
        assert_eq!(engine.budget_usage_percent(), 0.0);
        engine.record_spend(1.0);
        assert_eq!(engine.budget_usage_percent(), 100.0);
    }

    #[test]
    fn roi_handles_profit_loss_and_zero_investment() {
        assert_eq!(ROIMetrics::calculate(100.0, 125.0, 24.0).roi_percent, 25.0);
        assert_eq!(ROIMetrics::calculate(100.0, 75.0, 24.0).roi_percent, -25.0);
        assert_eq!(ROIMetrics::calculate(0.0, 75.0, 24.0).roi_percent, 0.0);
        assert_eq!(ROIMetrics::calculate(100.0, 125.0, 0.0).annualized(), 0.0);
    }

    #[test]
    fn governance_affordability_uses_live_plutus_budget() {
        let mut engine = EconomicsEngine::new().with_budget(10.0);
        engine.record_spend(8.0);
        let gates = GovernanceGates::permissive();

        assert!(gates.evaluate_affordability(&engine, 2.0).allowed);
        let blocked = gates.evaluate_affordability(&engine, 2.01);
        assert!(!blocked.allowed);
        assert_eq!(blocked.policy, "plutus_daily_budget");
    }
}
