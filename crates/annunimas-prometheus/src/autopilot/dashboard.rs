// sigil: REPAIR
//! Dashboard — aggregates metrics and emits alerts.

use super::service_health::ServiceHealthReport;
use super::task_queue::TaskQueueMetrics;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DashboardSnapshot {
    pub task_completion_rate: f64,
    pub avg_aging_pending_secs: f64,
    pub recent_completions_1h: usize,
    pub service_overall_score: f64,
    pub services_failed: usize,
    pub alerts: Vec<Alert>,
}

pub fn build_snapshot(
    queue: &TaskQueueMetrics,
    services: &ServiceHealthReport,
) -> DashboardSnapshot {
    // Use the 24h window — lifetime cumulative is dominated by historical data.
    let rate = queue.completion_rate_24h;
    let mut snap = DashboardSnapshot {
        task_completion_rate: rate,
        avg_aging_pending_secs: queue.aging_oldest_pending_secs.unwrap_or(0) as f64,
        recent_completions_1h: queue.recent_completions_1h,
        service_overall_score: services.overall_score,
        services_failed: services.failed,
        alerts: Vec::new(),
    };
    if services.failed > 0 {
        snap.alerts.push(Alert {
            severity: AlertSeverity::Critical,
            source: "systemd".into(),
            message: format!("{} annunimas service(s) failed", services.failed),
        });
    }
    if queue.aging_oldest_pending_secs.unwrap_or(0) > 86_400 {
        snap.alerts.push(Alert {
            severity: AlertSeverity::Warning,
            source: "queue".into(),
            message: "oldest pending task is over 24h old".into(),
        });
    }
    let denom = queue.recent_completions_24h + queue.recent_failures_24h;
    if denom >= 3 && rate < 0.5 {
        snap.alerts.push(Alert {
            severity: AlertSeverity::Warning,
            source: "queue".into(),
            message: format!(
                "low 24h completion rate: {:.1}% ({}/{})",
                rate * 100.0,
                queue.recent_completions_24h,
                denom
            ),
        });
    }
    snap
}

#[cfg(test)]
mod tests {
    use super::super::service_health::ServiceHealth;
    use super::*;
    #[test]
    fn raises_failed_service_alert() {
        let q = TaskQueueMetrics::default();
        let s = ServiceHealthReport {
            services: vec![ServiceHealth {
                unit: "x".into(),
                load: "loaded".into(),
                active: "failed".into(),
                sub: "failed".into(),
                score: 0.0,
                note: "failed".into(),
            }],
            healthy: 0,
            degraded: 0,
            failed: 1,
            overall_score: 0.0,
        };
        let snap = build_snapshot(&q, &s);
        assert!(matches!(
            snap.alerts.first().map(|a| &a.severity),
            Some(AlertSeverity::Critical)
        ));
    }
}
