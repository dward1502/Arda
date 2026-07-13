// sigil: REPAIR
//! Plutus - JouleWork economics engine
//!
//! Financial flows, JW accounting, ROI proof
//! Implements Love Equation and CRUSTIES economy layer

pub mod economics;
pub mod error;
pub mod joule_work;
pub mod ledger;
pub mod love_equation;
pub mod meter;
pub mod service;
pub mod transport;

pub use economics::{CostModel, CostModelConfig, EconomicsEngine, LinearCostModel, ROIMetrics};
pub use error::PlutusError;
pub use joule_work::{JouleWork, JouleWorkSummary, JouleWorkTracker, JouleWorkUnit};
pub use ledger::PlutusLedger;
pub use love_equation::{LoveConfig, LoveEquation, LoveScore};
pub use meter::{
    EnergyMeter, EstimatorMeter, JouleSample, MeterRegistry, SampleSource, TariffTable, WorkProfile,
};
pub use service::{PlutusRuntimePaths, PlutusService, PLUTUS_RUNTIME_SCHEMA_VERSION};
pub use transport::{expand_home, PlutusDaemon, PlutusDaemonConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn economics_engine_tracks_budget_and_snapshot() {
        let mut engine = EconomicsEngine::new().with_budget(10.0);
        engine.register_model(CostModelConfig {
            provider: "openai".to_string(),
            input_rate: 0.001,
            output_rate: 0.002,
            batch_size: 1000,
        });

        let cost = engine
            .calculate_cost("openai", 100, 50)
            .expect("cost calculated");
        assert!(cost > 0.0);
        assert!(engine.can_afford(cost));
        engine.record_spend(cost);

        let snapshot = engine.status_snapshot();
        assert_eq!(snapshot["providers"][0], "openai");
        assert!(snapshot["budget_remaining"].as_f64().unwrap_or_default() < 10.0);
    }

    #[tokio::test]
    async fn joulework_tracker_summarizes_work() {
        let tracker = JouleWorkTracker::new();
        tracker
            .track_work(
                "athena",
                2.0,
                JouleWorkUnit::Reasoning,
                Some("task_1".to_string()),
            )
            .await;
        tracker
            .track_work("hermes", 1.0, JouleWorkUnit::Network, None)
            .await;

        let summary = tracker.summary().await;
        assert!(summary.total > 0.0);
        assert_eq!(tracker.agent_total("athena").await, 4.0);

        let snapshot = tracker.status_snapshot().await;
        assert_eq!(snapshot["by_agent"]["athena"], 4.0);
    }

    #[test]
    fn love_equation_records_and_ranks_relationships() {
        let mut love = LoveEquation::new();
        let score = love.calculate("athena", "hermes", 0.9, 0.7, 0.8);
        love.record_relationship("athena".to_string(), "hermes".to_string(), score);
        love.record_relationship("athena".to_string(), "hades".to_string(), 0.4);

        let top = love.top_relationships(1);
        assert_eq!(top.len(), 1);
        assert!(top[0].value >= 0.7);

        let snapshot = love.snapshot(2);
        assert_eq!(snapshot["relationships_total"], 2);
    }

    #[test]
    fn plutus_ledger_tracks_balances() {
        let mut ledger = PlutusLedger::new();
        ledger.credit("athena", 5.0);
        ledger.credit("athena", 2.5);
        ledger.credit("hermes", 1.0);

        assert_eq!(ledger.balance("athena"), 7.5);
        let snapshot = ledger.snapshot();
        assert_eq!(snapshot["accounts_total"], 2);
    }
}
