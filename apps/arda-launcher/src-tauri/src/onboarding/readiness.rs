use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::onboarding::helpers::{get_host_name, now_utc};
use crate::onboarding::io::read_json_optional;
use crate::onboarding::types::*;

pub fn l3_readiness_onboarding_checklist() -> Value {
    json!({
        "contract": "arda.onboarding.l3_readiness.v1",
        "source_doc": "docs/operations/l3-readiness-onboarding.md",
        "levels": [
            {
                "id": "read_only",
                "meaning": "Operator and agent surfaces may inspect projections and receipts without task mutation."
            },
            {
                "id": "safe_local",
                "meaning": "Agents may select local, reversible work packets and prepare evidence, but mutation remains gated by class."
            },
            {
                "id": "bounded_mutation",
                "meaning": "Only approved local classes with path limits, verification, rollback, and receipt evidence may mutate."
            },
            {
                "id": "human_required",
                "meaning": "External side effects, credentials, destructive cleanup, service changes, provider reroutes, and commitments require explicit human or triad approval."
            }
        ],
        "verification": [
            "cargo run -p arda-cli --release -- export l3-readiness-projection",
            "jq '{status, flywheel, queue, hades_lifecycle}' core/state/l3_readiness_projection.json",
            "cargo run -p arda-cli --release -- pipeline flywheel-packet-readiness",
            "scripts/check_task_queue_append_only.sh"
        ],
        "operator_surfaces": [
            "core/state/l3_readiness_projection.json",
            "core/state/flywheel_packet_runtime.json",
            "core/state/task_lifecycle_runtime.json",
            "external:/var/home/mythos/Eregion/arda-hud",
            "crates/arda-hermes"
        ],
        "human_gates": [
            "destructive delete or archive/retention mutation",
            "credential or private config writes",
            "external message send",
            "service restart or disablement",
            "provider reroute/reload policy change",
            "fleet mutation",
            "funds, legal, customer, or public commitment"
        ],
        "low_power_route_posture": {
            "default": "capability_and_context_headroom_before_local_preference",
            "local_models": "acceptable for low-context helper work when healthy",
            "degraded_route": "surface as blocker; do not claim L3 live readiness from weak local fallback alone"
        }
    })
}

pub fn build_readiness_projection(
    profile: &EnvironmentProfile,
    root: &Path,
) -> ReadinessProjection {
    let portability_source = root.join("audit/PORTABILITY_AUDIT_2026-05-24/summary.json");
    let mut checks = Vec::new();

    checks.push(ReadinessCheck {
        check_id: "AGENTS.md".to_string(),
        title: "Agent/project operating instructions available".to_string(),
        status: if root.join("AGENTS.md").exists() {
            "pass"
        } else {
            "warn"
        }
        .to_string(),
        severity: "high".to_string(),
        evidence: if root.join("AGENTS.md").exists() {
            vec!["present: AGENTS.md".to_string()]
        } else {
            vec!["missing: AGENTS.md".to_string()]
        },
        recommendation: "Keep AGENTS.md current as setup instruction context.".to_string(),
    });

    checks.push(ReadinessCheck {
        check_id: "ARDA_ROOT_PROTOCOL.md".to_string(),
        title: "Root protocol available".to_string(),
        status: if root.join("arda_ROOT_PROTOCOL.md").exists() {
            "pass"
        } else {
            "warn"
        }
        .to_string(),
        severity: "high".to_string(),
        evidence: if root.join("arda_ROOT_PROTOCOL.md").exists() {
            vec!["present: arda_ROOT_PROTOCOL.md".to_string()]
        } else {
            vec!["missing: arda_ROOT_PROTOCOL.md".to_string()]
        },
        recommendation: "Preserve root protocol pointer for new-machine onboarding.".to_string(),
    });

    checks.push(ReadinessCheck {
        check_id: "environment.surface".to_string(),
        title: "Environment profile/template surface discoverable".to_string(),
        status: if root
            .join("core/state/environment_profile.schema.json")
            .exists()
        {
            "pass"
        } else {
            "warn"
        }
        .to_string(),
        severity: "medium".to_string(),
        evidence: vec![
            format!(
                "present: {}",
                if root
                    .join("core/state/environment_profile.schema.json")
                    .exists()
                {
                    "core/state/environment_profile.schema.json"
                } else {
                    "missing core/state/environment_profile.schema.json"
                }
            ),
            format!(
                "present: {}",
                if root.join("config/arda.template.toml").exists() {
                    "config/arda.template.toml"
                } else {
                    "missing config/arda.template.toml"
                }
            ),
        ],
        recommendation:
            "Expose a setup-console path from the profile contract to local override templates."
                .to_string(),
    });

    checks.push(ReadinessCheck {
        check_id: "portability.receipt".to_string(),
        title: "Portability/config hygiene findings classified".to_string(),
        status: if portability_source.exists() { "pass" } else { "warn" }.to_string(),
        severity: "medium".to_string(),
        evidence: if portability_source.exists() {
            vec![format!("receipt: {}", portability_source.display())]
        } else {
            vec!["missing: audit/PORTABILITY_AUDIT_2026-05-24/summary.json".to_string()]
        },
        recommendation: "Keep setup console read-only until portability findings are parameterized.".to_string(),
    });

    if !profile.missing_gates.is_empty() {
        checks.push(ReadinessCheck {
            check_id: "endpoint.missing_gates".to_string(),
            title: "Service assumptions are explicit".to_string(),
            status: "warn".to_string(),
            severity: "medium".to_string(),
            evidence: profile
                .missing_gates
                .iter()
                .cloned()
                .map(|x| format!("missing: {x}"))
                .collect(),
            recommendation: "Populate missing endpoints before enabling service start helpers."
                .to_string(),
        });
    }

    let mut summary = BTreeMap::new();
    for check in &checks {
        *summary.entry(check.status.clone()).or_insert(0) += 1;
    }
    let gate_status = if summary.get("warn").copied().unwrap_or(0) == 0
        && summary.get("fail").copied().unwrap_or(0) == 0
    {
        "pass"
    } else {
        "warn"
    };

    let portability_status = if portability_source.exists() {
        let raw = read_json_optional(&portability_source)
            .and_then(|value| value.get("summary").and_then(Value::as_object).cloned())
            .unwrap_or_default();
        let active_blockers = raw
            .get("active_blocker_findings")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let findings_total = raw
            .get("findings_total")
            .or_else(|| raw.get("total_findings"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        json!({
            "status": if active_blockers == 0 { "pass" } else { "warn" },
            "active_blocker_findings": active_blockers,
            "findings_total": findings_total,
            "label": if active_blockers == 0 { "zero active portability blockers" } else { "active portability blockers present" },
            "source": portability_source.to_string_lossy(),
        })
    } else {
        json!({
            "status": "missing",
            "active_blocker_findings": null,
            "findings_total": null,
            "label": "portability receipt missing",
            "source": portability_source.to_string_lossy(),
        })
    };

    let runtime = json!({
        "cwd": std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "/".to_string()),
        "host": get_host_name(),
        "platform": std::env::consts::OS,
        "repo_root": root.to_string_lossy().to_string(),
    });

    let mut pass = Vec::new();
    let mut warn = Vec::new();
    for check in &checks {
        match check.status.as_str() {
            "pass" => pass.push(check.check_id.clone()),
            _ => warn.push(check.check_id.clone()),
        }
    }

    ReadinessProjection {
        checks,
        gate_status: gate_status.to_string(),
        generated_at_utc: now_utc(),
        mode: "read_only".to_string(),
        mutation_policy: "receipts_only_no_source_config_or_service_rewrites".to_string(),
        portability_status,
        runtime,
        schema_version: 1,
        summary,
        pass,
        warn,
        runner: Some("arda-onboarding".to_string()),
    }
}
