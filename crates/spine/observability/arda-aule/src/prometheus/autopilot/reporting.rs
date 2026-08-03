#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! CEO autopilot reporting from cycle snapshots, heartbeats, and learning.

use super::learning::LearningStore;
use super::runner::CycleReport;
use chrono::{DateTime, Datelike, Utc};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn write_daily_report(
    report_dir: impl AsRef<Path>,
    heartbeat_path: impl AsRef<Path>,
    learning_path: impl AsRef<Path>,
    cycle: &CycleReport,
) -> std::io::Result<PathBuf> {
    let report_dir = report_dir.as_ref();
    std::fs::create_dir_all(report_dir)?;
    let path = report_dir.join(format!("daily_{}.md", Utc::now().format("%Y-%m-%d")));
    let heartbeat_count = recent_heartbeat_count(heartbeat_path.as_ref());
    let learning = LearningStore::new(learning_path.as_ref()).load();
    let governance = governance_summary(cycle);
    let mut learning_rows = learning
        .stats
        .iter()
        .map(|(key, stats)| {
            let (agent, task_type) = key.split_once("::").unwrap_or((key, "unknown"));
            format!(
                "- {} / {}: attempts={}, success_rate={:.2}, avg_joules={:.1}",
                agent,
                task_type,
                stats.attempts,
                stats.success_rate(),
                stats.avg_joules
            )
        })
        .collect::<Vec<_>>();
    learning_rows.sort();
    if learning_rows.is_empty() {
        learning_rows.push("- no learned outcomes yet".to_string());
    }

    let content = format!(
        "# CEO Autopilot Daily Summary\n\n\
Generated: {}\n\n\
## Cycle\n\
- objectives processed: {}\n\
- outcomes ingested: {}\n\
- plans queued: {}\n\
- Apollo dispatches: {}\n\
- A2H responses processed: {}\n\
- H2A objectives resumed: {}\n\n\
## Queue\n\
- total: {}\n\
- pending: {}\n\
- in progress: {}\n\
- completed: {}\n\
- failed: {}\n\
- completion rate 24h: {:.2}\n\n\
## Services\n\
- healthy: {}\n\
- degraded: {}\n\
- failed: {}\n\
- score: {:.2}\n\n\
## Alerts\n{}\n\n\
## Governance\n\
- held objectives: {}\n\
- escalated objectives: {}\n\
- human required: {}\n\
- triad quorum required: {}\n\
- triad quorum approved: {}\n\
- HADES review required: {}\n\
- read-only benchmark required: {}\n\n\
### Governance classes\n{}\n\n\
### Triad Philosopher evidence\n{}\n\n\
## Sovereign Adapters\n\
- configured: {}\n\
- active runtime: {}\n\
- evidence only: {}\n\
- missing required: {}\n\
\n\
### Adapter receipts\n{}\n\n\
## Council Runtime\n\
- ledger records: {}\n\
- appended this cycle: {}\n\
- evidence only: {}\n\
- task promotion allowed: {}\n\n\
## Autonomy Readiness Gate\n\
- decision: {}\n\
- task promotion allowed: {}\n\
- reasons: {}\n\n\
## Learning\n{}\n\n\
## Heartbeats\n\
- recent heartbeat rows scanned: {}\n",
        cycle.timestamp,
        cycle.objectives_processed,
        cycle.outcomes_ingested,
        cycle
            .plans
            .iter()
            .map(|plan| plan.queued_task_ids.len())
            .sum::<usize>(),
        cycle
            .plans
            .iter()
            .map(|plan| plan.apollo_dispatches.len())
            .sum::<usize>(),
        cycle.h2a.responses_processed,
        cycle.h2a.objectives_resumed,
        cycle.queue.total,
        cycle.queue.pending,
        cycle.queue.in_progress,
        cycle.queue.completed,
        cycle.queue.failed,
        cycle.queue.completion_rate_24h,
        cycle.services.healthy,
        cycle.services.degraded,
        cycle.services.failed,
        cycle.services.overall_score,
        alerts(cycle),
        governance.held,
        governance.escalated,
        governance.human_required,
        governance.triad_required,
        governance.triad_approved,
        governance.hades_review_required,
        governance.read_only_benchmark_required,
        governance.class_rows(),
        governance.triad_philosopher_rows(),
        cycle.sovereign_adapters.adapter_count,
        cycle.sovereign_adapters.active_runtime_adapter_count,
        cycle.sovereign_adapters.evidence_only_adapter_count,
        cycle.sovereign_adapters.missing_required_adapter_count,
        sovereign_adapter_rows(cycle),
        cycle.council_runtime.existing_record_count,
        cycle.council_runtime.appended_record_count,
        cycle.council_runtime.evidence_only,
        cycle.council_runtime.task_promotion_allowed,
        cycle.autonomy_readiness.decision,
        cycle.autonomy_readiness.task_promotion_allowed,
        cycle.autonomy_readiness.reasons.join("; "),
        learning_rows.join("\n"),
        heartbeat_count,
    );
    std::fs::write(&path, content)?;
    Ok(path)
}

pub fn write_weekly_report(
    report_dir: impl AsRef<Path>,
    heartbeat_path: impl AsRef<Path>,
    learning_path: impl AsRef<Path>,
    cycle: &CycleReport,
) -> std::io::Result<PathBuf> {
    let report_dir = report_dir.as_ref();
    std::fs::create_dir_all(report_dir)?;
    let now = Utc::now();
    let iso = now.iso_week();
    let path = report_dir.join(format!("weekly_{}-W{:02}.md", iso.year(), iso.week()));
    let summary = weekly_summary(heartbeat_path.as_ref());
    let learning = LearningStore::new(learning_path.as_ref()).load();
    let mut best_routes = learning
        .stats
        .iter()
        .filter(|(_, stats)| stats.attempts >= 3)
        .map(|(key, stats)| {
            let (agent, task_type) = key.split_once("::").unwrap_or((key, "unknown"));
            format!(
                "- {} / {}: attempts={}, success_rate={:.2}, avg_joules={:.1}",
                agent,
                task_type,
                stats.attempts,
                stats.success_rate(),
                stats.avg_joules
            )
        })
        .collect::<Vec<_>>();
    best_routes.sort();
    if best_routes.is_empty() {
        best_routes.push("- no route has enough attempts yet".to_string());
    }

    let content = format!(
        "# CEO Autopilot Weekly Summary\n\n\
Generated: {}\n\
Window: last 7 days\n\n\
## Throughput\n\
- cycles: {}\n\
- objectives processed: {}\n\
- outcomes ingested: {}\n\
- plans queued: {}\n\
- Apollo dispatches: {}\n\
- Pipeline submissions: {}\n\
- delegated Joules: {:.1}\n\n\
## H2A / A2H\n\
- responses processed: {}\n\
- objectives resumed: {}\n\
- denials recorded: {}\n\
- escalations emitted: {}\n\n\
## Governance\n\
- held objectives: {}\n\
- escalated objectives: {}\n\
- human required: {}\n\
- triad quorum required: {}\n\
- triad quorum approved: {}\n\
- HADES review required: {}\n\
- read-only benchmark required: {}\n\n\
## Sovereign Adapters\n\
- configured: {}\n\
- active runtime: {}\n\
- evidence only: {}\n\
- missing required: {}\n\n\
## Council Runtime\n\
- latest ledger records: {}\n\
- records appended: {}\n\n\
## Health\n\
- average service score: {:.2}\n\
- minimum service score: {:.2}\n\
- latest queue pending: {}\n\
- latest completion rate 24h: {:.2}\n\
- latest alerts: {}\n\n\
## Learning\n{}\n\n\
## Current Cycle\n\
- objectives processed: {}\n\
- outcomes ingested: {}\n\
- services failed: {}\n",
        cycle.timestamp,
        summary.cycles,
        summary.objectives,
        summary.outcomes,
        summary.plans_queued,
        summary.apollo_dispatches,
        summary.pipeline_submissions,
        summary.delegated_joules,
        summary.h2a_responses,
        summary.h2a_resumed,
        summary.h2a_denials,
        summary.a2h_escalations,
        summary.governance_held,
        summary.governance_escalated,
        summary.governance_human_required,
        summary.governance_triad_required,
        summary.governance_triad_approved,
        summary.governance_hades_review_required,
        summary.governance_read_only_benchmark_required,
        summary.sovereign_adapter_count,
        summary.sovereign_active_runtime_adapter_count,
        summary.sovereign_evidence_only_adapter_count,
        summary.sovereign_missing_required_adapter_count,
        summary.council_existing_record_count,
        summary.council_appended_record_count,
        summary.avg_service_score(),
        summary.min_service_score.unwrap_or(0.0),
        summary.latest_queue_pending.unwrap_or(0),
        summary.latest_completion_rate.unwrap_or(0.0),
        summary.latest_alerts.unwrap_or(0),
        best_routes.join("\n"),
        cycle.objectives_processed,
        cycle.outcomes_ingested,
        cycle.services.failed,
    );
    std::fs::write(&path, content)?;
    Ok(path)
}

fn alerts(cycle: &CycleReport) -> String {
    if cycle.dashboard.alerts.is_empty() {
        return "- none".to_string();
    }
    cycle
        .dashboard
        .alerts
        .iter()
        .map(|alert| format!("- {:?}: {}", alert.severity, alert.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sovereign_adapter_rows(cycle: &CycleReport) -> String {
    if cycle.sovereign_adapters.adapters.is_empty() {
        return "- none".to_string();
    }
    cycle
        .sovereign_adapters
        .adapters
        .iter()
        .map(|adapter| {
            format!(
                "- {} / {}: effect={}, receipts={}, source_records={}",
                adapter.id,
                adapter.crate_name,
                adapter.gate_effect,
                adapter.cycle_receipts,
                adapter.source_records
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Default)]
struct GovernanceSummary {
    class_counts: BTreeMap<String, usize>,
    triad_philosopher_evidence: BTreeMap<String, usize>,
    held: usize,
    escalated: usize,
    human_required: usize,
    triad_required: usize,
    triad_approved: usize,
    hades_review_required: usize,
    read_only_benchmark_required: usize,
}

impl GovernanceSummary {
    fn class_rows(&self) -> String {
        if self.class_counts.is_empty() {
            return "- none".to_string();
        }
        let mut rows = self
            .class_counts
            .iter()
            .take(20)
            .map(|(class, count)| format!("- {class}: {count}"))
            .collect::<Vec<_>>();
        let omitted = self.class_counts.len().saturating_sub(20);
        if omitted > 0 {
            rows.push(format!(
                "- ... {omitted} additional governance classes omitted"
            ));
        }
        rows.join("\n")
    }

    fn triad_philosopher_rows(&self) -> String {
        if self.triad_philosopher_evidence.is_empty() {
            return "- none".to_string();
        }
        let mut rows = self
            .triad_philosopher_evidence
            .iter()
            .take(20)
            .map(|(evidence, count)| format!("- {evidence}: {count}"))
            .collect::<Vec<_>>();
        let omitted = self.triad_philosopher_evidence.len().saturating_sub(20);
        if omitted > 0 {
            rows.push(format!(
                "- ... {omitted} additional Triad Philosopher evidence rows omitted"
            ));
        }
        rows.join("\n")
    }
}

fn governance_summary(cycle: &CycleReport) -> GovernanceSummary {
    let mut summary = GovernanceSummary::default();
    for plan in &cycle.plans {
        *summary
            .class_counts
            .entry(plan.governance.action_class.clone())
            .or_insert(0) += 1;
        for evidence in plan
            .governance
            .evidence
            .iter()
            .filter(|evidence| evidence.starts_with("triad_philosopher:"))
        {
            *summary
                .triad_philosopher_evidence
                .entry(evidence.clone())
                .or_insert(0) += 1;
        }
        if plan.governance.blocks_delegation()
            || !plan.gate.allows_delegation()
            || plan.joule_limited
        {
            summary.held += 1;
        }
        if plan.a2h_emitted
            || plan.governance.requires_escalation()
            || plan.gate.requires_escalation()
        {
            summary.escalated += 1;
        }
        if plan.governance.requires_human {
            summary.human_required += 1;
        }
        match plan.governance.gate {
            super::governance_policy::GovernanceGate::TriadQuorumRequired => {
                summary.triad_required += 1;
            }
            super::governance_policy::GovernanceGate::TriadQuorumApproved => {
                summary.triad_approved += 1;
            }
            super::governance_policy::GovernanceGate::HadesReviewRequired => {
                summary.hades_review_required += 1;
            }
            super::governance_policy::GovernanceGate::ReadOnlyBenchmarkRequired => {
                summary.read_only_benchmark_required += 1;
            }
            _ => {}
        }
    }
    summary
}

fn recent_heartbeat_count(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

#[derive(Debug, Default)]
struct WeeklySummary {
    cycles: u64,
    objectives: u64,
    outcomes: u64,
    plans_queued: u64,
    apollo_dispatches: u64,
    pipeline_submissions: u64,
    delegated_joules: f64,
    h2a_responses: u64,
    h2a_resumed: u64,
    h2a_denials: u64,
    a2h_escalations: u64,
    service_score_total: f64,
    min_service_score: Option<f64>,
    latest_queue_pending: Option<u64>,
    latest_completion_rate: Option<f64>,
    latest_alerts: Option<u64>,
    governance_held: u64,
    governance_escalated: u64,
    governance_human_required: u64,
    governance_triad_required: u64,
    governance_triad_approved: u64,
    governance_hades_review_required: u64,
    governance_read_only_benchmark_required: u64,
    sovereign_adapter_count: u64,
    sovereign_active_runtime_adapter_count: u64,
    sovereign_evidence_only_adapter_count: u64,
    sovereign_missing_required_adapter_count: u64,
    council_existing_record_count: u64,
    council_appended_record_count: u64,
}

impl WeeklySummary {
    fn avg_service_score(&self) -> f64 {
        if self.cycles == 0 {
            0.0
        } else {
            self.service_score_total / self.cycles as f64
        }
    }
}

fn weekly_summary(path: &Path) -> WeeklySummary {
    let Ok(content) = std::fs::read_to_string(path) else {
        return WeeklySummary::default();
    };
    let cutoff = Utc::now() - chrono::Duration::days(7);
    let mut summary = WeeklySummary::default();
    for row in content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| {
            row.get("ts")
                .and_then(|value| value.as_str())
                .and_then(parse_ts)
                .map(|ts| ts >= cutoff)
                .unwrap_or(false)
        })
    {
        summary.cycles += 1;
        summary.objectives += u64_field(&row, "objectives");
        summary.outcomes += u64_field(&row, "outcomes_ingested");
        summary.plans_queued += u64_field(&row, "plans_queued");
        summary.apollo_dispatches += u64_field(&row, "apollo_dispatches");
        summary.pipeline_submissions += u64_field(&row, "pipeline_submissions");
        summary.delegated_joules += f64_field(&row, "delegated_joules");
        summary.h2a_responses += u64_field(&row, "h2a_responses_processed");
        summary.h2a_resumed += u64_field(&row, "h2a_objectives_resumed");
        summary.h2a_denials += u64_field(&row, "h2a_denials_recorded");
        summary.a2h_escalations += u64_field(&row, "a2h_escalations");
        summary.governance_held += u64_field(&row, "governance_held");
        summary.governance_escalated += u64_field(&row, "governance_escalated");
        summary.governance_human_required += u64_field(&row, "governance_human_required");
        summary.governance_triad_required += u64_field(&row, "governance_triad_required");
        summary.governance_triad_approved += u64_field(&row, "governance_triad_approved");
        summary.governance_hades_review_required +=
            u64_field(&row, "governance_hades_review_required");
        summary.governance_read_only_benchmark_required +=
            u64_field(&row, "governance_read_only_benchmark_required");
        summary.sovereign_adapter_count = u64_field(&row, "sovereign_adapter_count");
        summary.sovereign_active_runtime_adapter_count =
            u64_field(&row, "sovereign_active_runtime_adapter_count");
        summary.sovereign_evidence_only_adapter_count =
            u64_field(&row, "sovereign_evidence_only_adapter_count");
        summary.sovereign_missing_required_adapter_count =
            u64_field(&row, "sovereign_missing_required_adapter_count");
        summary.council_existing_record_count = u64_field(&row, "council_existing_record_count");
        summary.council_appended_record_count += u64_field(&row, "council_appended_record_count");
        let service_score = f64_field(&row, "service_score");
        summary.service_score_total += service_score;
        summary.min_service_score = Some(match summary.min_service_score {
            Some(current) => current.min(service_score),
            None => service_score,
        });
        summary.latest_queue_pending = Some(u64_field(&row, "queue_pending"));
        summary.latest_completion_rate = Some(f64_field(&row, "completion_rate_24h"));
        summary.latest_alerts = Some(u64_field(&row, "alerts"));
    }
    summary
}

fn parse_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|ts| ts.with_timezone(&Utc))
}

fn u64_field(row: &serde_json::Value, key: &str) -> u64 {
    row.get(key).and_then(|value| value.as_u64()).unwrap_or(0)
}

fn f64_field(row: &serde_json::Value, key: &str) -> f64 {
    let value = row.get(key).and_then(|value| value.as_f64()).unwrap_or(0.0);
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::super::dashboard::build_snapshot;
    use super::super::decomposer::{PlannedTask, Priority};
    use super::super::governance_policy::{GovernanceDecision, GovernanceGate};
    use super::super::oracle_gate::GateDecision;
    use super::super::runner::{CycleReport, PlanCycle};
    use super::super::service_health::ServiceHealthReport;
    use super::super::task_queue::TaskQueueMetrics;
    use super::super::validator::ValidationResult;
    use super::*;

    #[test]
    fn writes_daily_report() {
        let dir = tempfile::tempdir().unwrap();
        let queue = TaskQueueMetrics::default();
        let services = ServiceHealthReport::default();
        let dashboard = build_snapshot(&queue, &services);
        let cycle = CycleReport {
            timestamp: Utc::now().to_rfc3339(),
            queue,
            services,
            dashboard,
            objective_selection: Default::default(),
            objectives_processed: 1,
            plans: Vec::new(),
            outcomes_ingested: 0,
            h2a: Default::default(),
            hades_introspection: Default::default(),
            sovereign_adapters: Default::default(),
            council_runtime: Default::default(),
            autonomy_readiness: Default::default(),
            report_path: None,
            weekly_report_path: None,
        };
        let path = write_daily_report(
            dir.path().join("reports"),
            dir.path().join("heartbeats.jsonl"),
            dir.path().join("learning.json"),
            &cycle,
        )
        .unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("CEO Autopilot Daily Summary"));
        assert!(content.contains("objectives processed: 1"));
    }

    #[test]
    fn daily_report_includes_bounded_governance_summary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cycle = cycle_with_plans(vec![
            plan_cycle(
                "reroute-1",
                governance(
                    "provider_reroute",
                    GovernanceGate::TriadQuorumRequired,
                    false,
                    false,
                ),
                GateDecision::Skipped,
                false,
                false,
            ),
            plan_cycle(
                "human-1",
                governance(
                    "credential_rotation",
                    GovernanceGate::HumanRequired,
                    true,
                    false,
                ),
                GateDecision::Rejected {
                    resonance: 0.2,
                    concerns: vec!["human approval required".to_string()],
                },
                true,
                false,
            ),
            plan_cycle(
                "approved-1",
                governance(
                    "provider_reroute",
                    GovernanceGate::TriadQuorumApproved,
                    false,
                    true,
                ),
                GateDecision::Approved { resonance: 0.91 },
                false,
                false,
            ),
        ]);

        let path = write_daily_report(
            dir.path().join("reports"),
            dir.path().join("heartbeats.jsonl"),
            dir.path().join("learning.json"),
            &cycle,
        )
        .expect("daily report");
        let content = std::fs::read_to_string(path).expect("daily report content");

        assert!(content.contains("## Governance"));
        assert!(content.contains("held objectives: 2"));
        assert!(content.contains("escalated objectives: 1"));
        assert!(content.contains("human required: 1"));
        assert!(content.contains("triad quorum required: 1"));
        assert!(content.contains("triad quorum approved: 1"));
        assert!(content.contains("- provider_reroute: 2"));
        assert!(content.contains("- credential_rotation: 1"));
    }

    #[test]
    fn daily_report_surfaces_compact_triad_philosopher_evidence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut governance = governance(
            "provider_reroute",
            GovernanceGate::TriadQuorumRequired,
            false,
            false,
        );
        governance
            .evidence
            .push("triad_philosopher:hold:0.42".to_string());
        let cycle = cycle_with_plans(vec![plan_cycle(
            "reroute-1",
            governance,
            GateDecision::Skipped,
            false,
            false,
        )]);

        let path = write_daily_report(
            dir.path().join("reports"),
            dir.path().join("heartbeats.jsonl"),
            dir.path().join("learning.json"),
            &cycle,
        )
        .expect("daily report");
        let content = std::fs::read_to_string(path).expect("daily report content");

        assert!(content.contains("### Triad Philosopher evidence"));
        assert!(content.contains("- triad_philosopher:hold:0.42: 1"));
    }

    #[test]
    fn weekly_report_rolls_up_governance_heartbeat_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let heartbeat = dir.path().join("heartbeats.jsonl");
        std::fs::write(
            &heartbeat,
            format!(
                "{{\"ts\":\"{}\",\"objectives\":2,\"outcomes_ingested\":0,\"plans_queued\":1,\"apollo_dispatches\":0,\"pipeline_submissions\":0,\"delegated_joules\":0.0,\"service_score\":1.0,\"queue_pending\":0,\"completion_rate_24h\":1.0,\"alerts\":0,\"governance_held\":2,\"governance_escalated\":1,\"governance_human_required\":1,\"governance_triad_required\":1,\"governance_triad_approved\":1,\"governance_hades_review_required\":0,\"governance_read_only_benchmark_required\":0}}\n",
                Utc::now().to_rfc3339()
            ),
        )
        .expect("heartbeat write");
        let cycle = cycle_with_plans(Vec::new());

        let path = write_weekly_report(
            dir.path().join("reports"),
            &heartbeat,
            dir.path().join("learning.json"),
            &cycle,
        )
        .expect("weekly report");
        let content = std::fs::read_to_string(path).expect("weekly report content");

        assert!(content.contains("## Governance"));
        assert!(content.contains("held objectives: 2"));
        assert!(content.contains("escalated objectives: 1"));
        assert!(content.contains("human required: 1"));
        assert!(content.contains("triad quorum required: 1"));
        assert!(content.contains("triad quorum approved: 1"));
    }

    #[test]
    fn writes_weekly_report_from_heartbeats() {
        let dir = tempfile::tempdir().unwrap();
        let heartbeat = dir.path().join("heartbeats.jsonl");
        std::fs::write(
            &heartbeat,
            format!(
                "{{\"ts\":\"{}\",\"objectives\":2,\"outcomes_ingested\":3,\"plans_queued\":4,\"apollo_dispatches\":5,\"pipeline_submissions\":1,\"delegated_joules\":12.5,\"service_score\":0.9,\"queue_pending\":7,\"completion_rate_24h\":1.0,\"alerts\":0}}\n",
                Utc::now().to_rfc3339()
            ),
        )
        .unwrap();
        let queue = TaskQueueMetrics::default();
        let services = ServiceHealthReport::default();
        let dashboard = build_snapshot(&queue, &services);
        let cycle = CycleReport {
            timestamp: Utc::now().to_rfc3339(),
            queue,
            services,
            dashboard,
            objective_selection: Default::default(),
            objectives_processed: 0,
            plans: Vec::new(),
            outcomes_ingested: 0,
            h2a: Default::default(),
            hades_introspection: Default::default(),
            sovereign_adapters: Default::default(),
            council_runtime: Default::default(),
            autonomy_readiness: Default::default(),
            report_path: None,
            weekly_report_path: None,
        };
        let path = write_weekly_report(
            dir.path().join("reports"),
            &heartbeat,
            dir.path().join("learning.json"),
            &cycle,
        )
        .unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("CEO Autopilot Weekly Summary"));
        assert!(content.contains("objectives processed: 2"));
        assert!(content.contains("delegated Joules: 12.5"));
    }

    fn cycle_with_plans(plans: Vec<PlanCycle>) -> CycleReport {
        let queue = TaskQueueMetrics::default();
        let services = ServiceHealthReport::default();
        let dashboard = build_snapshot(&queue, &services);
        CycleReport {
            timestamp: Utc::now().to_rfc3339(),
            queue,
            services,
            dashboard,
            objective_selection: Default::default(),
            objectives_processed: plans.len(),
            plans,
            outcomes_ingested: 0,
            h2a: Default::default(),
            hades_introspection: Default::default(),
            sovereign_adapters: Default::default(),
            council_runtime: Default::default(),
            autonomy_readiness: Default::default(),
            report_path: None,
            weekly_report_path: None,
        }
    }

    fn plan_cycle(
        objective_id: &str,
        governance: GovernanceDecision,
        gate: GateDecision,
        a2h_emitted: bool,
        joule_limited: bool,
    ) -> PlanCycle {
        PlanCycle {
            objective_id: objective_id.to_string(),
            plan: vec![PlannedTask {
                key: format!("{objective_id}-task"),
                title: "test task".to_string(),
                task_type: "test".to_string(),
                depends_on: Vec::new(),
                priority: Priority::Medium,
                joule_cost: 1.0,
                eta_seconds: 1,
                assigned_agent: Some("prometheus".to_string()),
            }],
            validation: ValidationResult {
                ok: true,
                ..Default::default()
            },
            governance,
            gate,
            autonomy_readiness_decision: "allow".into(),
            autonomy_readiness_reasons: Vec::new(),
            delegation: None,
            queued_task_ids: Vec::new(),
            queue_operation: None,
            apollo_dispatches: Vec::new(),
            a2h_emitted,
            joule_limited,
            pipeline_submitted: false,
        }
    }

    fn governance(
        action_class: &str,
        gate: GovernanceGate,
        requires_human: bool,
        allowed_to_delegate: bool,
    ) -> GovernanceDecision {
        GovernanceDecision {
            contract: "autopilot_governance_v1".to_string(),
            objective_id: "objective".to_string(),
            action_class: action_class.to_string(),
            gate: gate.clone(),
            requires_human,
            requires_triad: matches!(
                gate,
                GovernanceGate::TriadQuorumRequired | GovernanceGate::TriadQuorumApproved
            ),
            requires_hades_review: matches!(gate, GovernanceGate::HadesReviewRequired),
            confidence: "test".to_string(),
            allowed_to_delegate,
            reasons: Vec::new(),
            evidence: Vec::new(),
            triad_quorum: None,
        }
    }
}
