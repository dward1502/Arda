// Predictive maintenance and resource planning for Chronos agent
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System metrics for predictive analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_in: u64,
    pub network_out: u64,
}

/// Resource prediction based on historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePrediction {
    pub predicted_cpu: f64,
    pub predicted_memory: f64,
    pub anomaly_detected: bool,
    pub confidence: f64,
}

/// Moving average predictor for system metrics
pub struct MovingAveragePredictor {
    history: Vec<SystemMetrics>,
    window_size: usize,
}

impl MovingAveragePredictor {
    pub fn new(window_size: usize) -> Self {
        Self {
            history: Vec::new(),
            window_size,
        }
    }

    pub fn add_metric(&mut self, metric: SystemMetrics) {
        self.history.push(metric);
        if self.history.len() > self.window_size {
            self.history.remove(0);
        }
    }

    pub fn predict(&self) -> ResourcePrediction {
        if self.history.is_empty() {
            return ResourcePrediction {
                predicted_cpu: 0.5,
                predicted_memory: 0.5,
                anomaly_detected: false,
                confidence: 0.5,
            };
        }

        let mean: f64 =
            self.history.iter().map(|m| m.cpu_usage).sum::<f64>() / self.history.len() as f64;
        let variance: f64 = self
            .history
            .iter()
            .map(|m| {
                let diff = m.cpu_usage - mean;
                diff * diff
            })
            .sum::<f64>()
            / self.history.len() as f64;
        let latest_anomaly = self
            .history
            .last()
            .is_some_and(|metric| self.is_anomalous(metric));

        ResourcePrediction {
            predicted_cpu: mean,
            predicted_memory: mean,
            anomaly_detected: variance > 10.0 || latest_anomaly,
            confidence: if variance < 5.0 { 0.9 } else { 0.5 },
        }
    }

    fn is_anomalous(&self, metric: &SystemMetrics) -> bool {
        let cpu_values: Vec<f64> = self.history.iter().map(|m| m.cpu_usage).collect();
        let mean = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;

        if cpu_values.len() < 2 {
            return false;
        }

        let std_dev = (cpu_values
            .iter()
            .map(|v| {
                let diff = v - mean;
                diff * diff
            })
            .sum::<f64>()
            / cpu_values.len() as f64)
            .sqrt();

        (metric.cpu_usage - mean).abs() > 2.0 * std_dev
    }
}

/// Anomaly detector for system metrics
pub struct AnomalyDetector {
    thresholds: HashMap<String, f64>,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert("cpu_high".to_string(), 85.0);
        thresholds.insert("memory_high".to_string(), 80.0);
        thresholds.insert("disk_high".to_string(), 90.0);
        Self { thresholds }
    }

    pub fn check_anomaly(&self, metrics: &SystemMetrics) -> Vec<String> {
        let mut anomalies = Vec::new();

        if metrics.cpu_usage > *self.thresholds.get("cpu_high").unwrap_or(&90.0) {
            anomalies.push("High CPU usage detected".to_string());
        }

        if metrics.memory_usage > *self.thresholds.get("memory_high").unwrap_or(&85.0) {
            anomalies.push("High memory usage detected".to_string());
        }

        if metrics.disk_usage > *self.thresholds.get("disk_high").unwrap_or(&95.0) {
            anomalies.push("High disk usage detected".to_string());
        }

        anomalies
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_add_metric() {
        let mut predictor = MovingAveragePredictor::new(3);

        let metric1 = SystemMetrics {
            timestamp: Utc::now(),
            cpu_usage: 50.0,
            memory_usage: 40.0,
            disk_usage: 30.0,
            network_in: 1000,
            network_out: 2000,
        };

        predictor.add_metric(metric1);
        assert_eq!(predictor.predict().predicted_cpu, 50.0);
    }

    #[test]
    fn test_prediction() {
        let mut predictor = MovingAveragePredictor::new(3);

        for i in 0..5 {
            let metric = SystemMetrics {
                timestamp: Utc::now() - Duration::hours(i as i64),
                cpu_usage: (50 + i * 5) as f64,
                memory_usage: (40 + i * 4) as f64,
                disk_usage: (30 + i * 3) as f64,
                network_in: 1000 + i as u64 * 100,
                network_out: 2000 + i as u64 * 200,
            };
            predictor.add_metric(metric);
        }

        let prediction = predictor.predict();
        assert!(prediction.predicted_cpu > 0.0);
    }

    #[test]
    fn test_anomaly_detection() {
        let detector = AnomalyDetector::new();

        let high_cpu_metrics = SystemMetrics {
            timestamp: Utc::now(),
            cpu_usage: 95.0,
            memory_usage: 40.0,
            disk_usage: 30.0,
            network_in: 1000,
            network_out: 2000,
        };

        let anomalies = detector.check_anomaly(&high_cpu_metrics);
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn default_anomaly_detector_uses_standard_thresholds() {
        let detector = AnomalyDetector::default();

        let normal_metrics = SystemMetrics {
            timestamp: Utc::now(),
            cpu_usage: 85.0,
            memory_usage: 80.0,
            disk_usage: 90.0,
            network_in: 1000,
            network_out: 2000,
        };

        assert!(detector.check_anomaly(&normal_metrics).is_empty());
    }
}
