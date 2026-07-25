#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Service health monitor — consumes the `arda-systemd` typed client
//! and applies unit-type-aware scoring (timers in `active/waiting` healthy;
//! oneshot services with sibling timers healthy when inactive; etc.).

use arda_core::systemd::{SystemctlClient, SystemdClient, Unit, UnitKind};
use serde::Serialize;
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub score: f64,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ServiceHealthReport {
    pub services: Vec<ServiceHealth>,
    pub healthy: usize,
    pub degraded: usize,
    pub failed: usize,
    pub overall_score: f64,
}

/// Re-export for backwards compatibility — older code (and tests) used
/// `service_health::SystemdQuery`. Now delegates to `arda-systemd`.
pub trait SystemdQuery {
    fn list_units(&self, pattern: &str) -> std::io::Result<String>;
}

/// Default implementation — shells out via `arda-systemd::SystemctlClient`.
pub struct UserSystemd;
impl SystemdQuery for UserSystemd {
    fn list_units(&self, pattern: &str) -> std::io::Result<String> {
        SystemctlClient
            .list_units_raw(pattern)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

pub struct ServiceHealthMonitor<S: SystemdQuery> {
    pub systemd: S,
    pub pattern: String,
}

impl Default for ServiceHealthMonitor<UserSystemd> {
    fn default() -> Self {
        Self {
            systemd: UserSystemd,
            pattern: "arda-*".into(),
        }
    }
}

impl<S: SystemdQuery> ServiceHealthMonitor<S> {
    pub fn collect(&self) -> ServiceHealthReport {
        let raw = self.systemd.list_units(&self.pattern).unwrap_or_default();
        let mut report = Self::parse(&raw);
        if report.services.is_empty() {
            report = Self::parse(&fallback_list_units(Some(&self.pattern)).unwrap_or_default());
        }
        if report.services.is_empty() {
            report = score_units(
                &arda_core::systemd::parse_list_units(
                    &fallback_list_units(None).unwrap_or_default(),
                )
                .into_iter()
                .filter(|unit| unit_matches_pattern(&unit.name, &self.pattern))
                .collect::<Vec<_>>(),
            );
        }
        if report.services.is_empty() {
            report = Self::parse(&fallback_shell_list_units(&self.pattern).unwrap_or_default());
        }
        if report.services.is_empty() {
            return missing_systemd_query_report();
        }
        report
    }

    pub fn parse(raw: &str) -> ServiceHealthReport {
        let units = arda_core::systemd::parse_list_units(raw);
        score_units(&units)
    }
}

fn fallback_list_units(pattern: Option<&str>) -> std::io::Result<String> {
    let mut args = vec![
        "--user",
        "list-units",
        "--all",
        "--no-legend",
        "--no-pager",
        "--plain",
    ];
    if let Some(pattern) = pattern {
        args.push(pattern);
    }
    let output = Command::new("systemctl").args(args).output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn unit_matches_pattern(unit_name: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        return unit_name.starts_with(prefix);
    }
    unit_name == pattern
}

fn fallback_shell_list_units(pattern: &str) -> std::io::Result<String> {
    if !pattern
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '*'))
    {
        return Err(std::io::Error::other("unsafe systemd unit pattern"));
    }
    let command = format!(
        "systemctl --user list-units --all --no-legend --no-pager --plain '{}'",
        pattern
    );
    let output = Command::new("/usr/bin/bash")
        .args(["-lc", &command])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn missing_systemd_query_report() -> ServiceHealthReport {
    ServiceHealthReport {
        services: vec![ServiceHealth {
            unit: "arda-systemd-query".to_string(),
            load: "not-found".to_string(),
            active: "failed".to_string(),
            sub: "empty".to_string(),
            score: 0.2,
            note: "systemd query returned no arda units".to_string(),
        }],
        healthy: 0,
        degraded: 0,
        failed: 1,
        overall_score: 0.2,
    }
}

/// Score a parsed list of units. Public so that callers using the typed
/// `SystemdClient` path can skip the legacy raw-string round-trip.
pub fn score_units(units: &[Unit]) -> ServiceHealthReport {
    let timer_stems: HashSet<&str> = units
        .iter()
        .filter(|u| u.kind == UnitKind::Timer)
        .map(|u| u.name.trim_end_matches(".timer"))
        .collect();

    let services: Vec<ServiceHealth> = units
        .iter()
        .map(|u| {
            let has_sibling_timer = u.kind == UnitKind::Service
                && timer_stems.contains(u.name.trim_end_matches(".service"));
            let (score, note) = score_for(u.kind, &u.load, &u.active, &u.sub, has_sibling_timer);
            ServiceHealth {
                unit: u.name.clone(),
                load: u.load.clone(),
                active: u.active.clone(),
                sub: u.sub.clone(),
                score,
                note,
            }
        })
        .collect();

    let healthy = services.iter().filter(|s| s.score >= 0.85).count();
    let degraded = services
        .iter()
        .filter(|s| s.score >= 0.4 && s.score < 0.85)
        .count();
    let failed = services.iter().filter(|s| s.score < 0.4).count();
    let overall_score = if services.is_empty() {
        1.0
    } else {
        services.iter().map(|s| s.score).sum::<f64>() / services.len() as f64
    };
    ServiceHealthReport {
        healthy,
        degraded,
        failed,
        overall_score,
        services,
    }
}

fn score_for(
    kind: UnitKind,
    load: &str,
    active: &str,
    sub: &str,
    has_sibling_timer: bool,
) -> (f64, String) {
    if active == "failed" {
        return (0.0, "failed".into());
    }
    if load == "not-found" {
        return (0.2, "unit not found".into());
    }
    if load == "masked" {
        return (0.3, "masked".into());
    }
    if load != "loaded" {
        return (0.3, format!("load={load}"));
    }

    match kind {
        UnitKind::Timer => match active {
            "active" => (1.0, format!("timer armed ({sub})")),
            "inactive" => (0.3, "timer inactive".into()),
            _ => (0.5, format!("timer {active}/{sub}")),
        },
        UnitKind::Socket => match active {
            "active" => (1.0, format!("socket listening ({sub})")),
            _ => (0.4, format!("socket {active}/{sub}")),
        },
        UnitKind::Service => match (active, sub) {
            ("active", "running") => (1.0, "running".into()),
            ("active", "exited") => (0.9, "oneshot exited".into()),
            ("activating", _) if has_sibling_timer => {
                (0.85, format!("timer-backed oneshot running ({sub})"))
            }
            ("activating", _) => (0.6, format!("activating ({sub})")),
            ("inactive", "dead") if has_sibling_timer => (0.85, "oneshot awaiting timer".into()),
            ("inactive", "dead") => (0.4, "stopped".into()),
            _ => (0.5, format!("{active}/{sub}")),
        },
        UnitKind::Other => (0.5, format!("{active}/{sub}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timer_active_waiting_is_healthy() {
        let raw = "arda-foo.timer loaded active waiting Foo timer\n";
        let r = ServiceHealthMonitor::<UserSystemd>::parse(raw);
        assert_eq!(r.healthy, 1);
        assert_eq!(r.failed, 0);
    }
    #[test]
    fn oneshot_with_sibling_timer_is_healthy() {
        let raw = "arda-foo.service loaded inactive dead Foo\narda-foo.timer loaded active waiting Foo timer\n";
        let r = ServiceHealthMonitor::<UserSystemd>::parse(raw);
        assert_eq!(r.failed, 0);
        assert!(
            r.services
                .iter()
                .find(|s| s.unit.ends_with(".service"))
                .unwrap()
                .score
                >= 0.85
        );
    }
    #[test]
    fn activating_oneshot_with_sibling_timer_is_healthy() {
        let raw = "arda-foo.service loaded activating start Foo\narda-foo.timer loaded active waiting Foo timer\n";
        let r = ServiceHealthMonitor::<UserSystemd>::parse(raw);
        assert_eq!(r.failed, 0);
        assert_eq!(r.degraded, 0);
        assert!(
            r.services
                .iter()
                .find(|s| s.unit.ends_with(".service"))
                .unwrap()
                .score
                >= 0.85
        );
    }
    #[test]
    fn standalone_service_inactive_is_degraded() {
        let raw = "arda-warden.service loaded inactive dead Warden\n";
        let r = ServiceHealthMonitor::<UserSystemd>::parse(raw);
        assert_eq!(r.degraded, 1);
    }
    #[test]
    fn failed_active_state_is_failed() {
        let raw = "arda-x.service loaded failed failed X\n";
        let r = ServiceHealthMonitor::<UserSystemd>::parse(raw);
        assert_eq!(r.failed, 1);
    }

    #[test]
    fn empty_query_is_not_reported_as_healthy() {
        let r = missing_systemd_query_report();
        assert_eq!(r.healthy, 0);
        assert_eq!(r.failed, 1);
        assert!(r.overall_score < 0.85);
    }

    #[test]
    fn unit_prefix_pattern_matches_arda_units() {
        assert!(unit_matches_pattern("arda-manwe.service", "arda-*"));
        assert!(!unit_matches_pattern("llama-server.service", "arda-*"));
        assert!(unit_matches_pattern(
            "arda-manwe.service",
            "arda-manwe.service"
        ));
    }
}
