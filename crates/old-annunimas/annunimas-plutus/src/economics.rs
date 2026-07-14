// sigil: REPAIR
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

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

    pub fn record_spend(&mut self, amount: f64) {
        self.total_spend += amount;
    }

    pub fn budget_remaining(&self) -> f64 {
        self.daily_budget - self.total_spend
    }

    pub fn budget_usage_percent(&self) -> f64 {
        (self.total_spend / self.daily_budget) * 100.0
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
