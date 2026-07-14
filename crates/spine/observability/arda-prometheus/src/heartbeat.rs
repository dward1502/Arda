// sigil: REPAIR
use crate::core_link::CoreAutonomyProfile;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatMode {
    Interval,
    Threshold,
}

impl fmt::Display for HeartbeatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HeartbeatMode::Interval => "interval",
            HeartbeatMode::Threshold => "threshold",
        })
    }
}

#[derive(Debug, Clone)]
pub struct HeartbeatState {
    pub mode: HeartbeatMode,
    pub interval_ms: u64,
    pub reason: String,
}

pub fn select_heartbeat_mode(profile: Option<&CoreAutonomyProfile>) -> HeartbeatState {
    let default_interval_ms = 5 * 60 * 1000;

    let Some(profile) = profile else {
        return HeartbeatState {
            mode: HeartbeatMode::Threshold,
            interval_ms: default_interval_ms,
            reason: "no core profile loaded".to_string(),
        };
    };

    let world_status = profile
        .world_status
        .as_deref()
        .unwrap_or("INITIALIZING")
        .to_ascii_uppercase();
    let interval_ms = profile.heartbeat_ms.max(100);

    if world_status == "ONLINE" || world_status == "READY" {
        HeartbeatState {
            mode: HeartbeatMode::Interval,
            interval_ms,
            reason: format!("world status is {world_status}"),
        }
    } else {
        HeartbeatState {
            mode: HeartbeatMode::Threshold,
            interval_ms,
            reason: format!("world status is {world_status}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{select_heartbeat_mode, HeartbeatMode};
    use crate::core_link::CoreAutonomyProfile;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn defaults_to_threshold_without_profile() {
        let hb = select_heartbeat_mode(None);
        assert_eq!(hb.mode, HeartbeatMode::Threshold);
    }

    #[test]
    fn uses_interval_when_world_online() {
        let profile = CoreAutonomyProfile {
            heartbeat_ms: 500,
            triad_bypass: true,
            base_costs: HashMap::new(),
            world_status: Some("ONLINE".to_string()),
            world_resonance: Some(60.0),
            source_root: PathBuf::from("core"),
        };
        let hb = select_heartbeat_mode(Some(&profile));
        assert_eq!(hb.mode, HeartbeatMode::Interval);
        assert_eq!(hb.interval_ms, 500);
    }

    fn profile_with_status(status: &str, heartbeat_ms: u64) -> CoreAutonomyProfile {
        CoreAutonomyProfile {
            heartbeat_ms,
            triad_bypass: false,
            base_costs: HashMap::new(),
            world_status: Some(status.to_string()),
            world_resonance: Some(50.0),
            source_root: PathBuf::from("core"),
        }
    }

    proptest! {
        #[test]
        fn interval_mode_iff_online_or_ready(status in "[A-Z]{4,12}") {
            let profile = profile_with_status(&status, 500);
            let hb = select_heartbeat_mode(Some(&profile));
            let upper = status.to_ascii_uppercase();
            let expect_interval = upper == "ONLINE" || upper == "READY";
            if expect_interval {
                prop_assert_eq!(hb.mode, HeartbeatMode::Interval);
            } else {
                prop_assert_eq!(hb.mode, HeartbeatMode::Threshold);
            }
        }

        #[test]
        fn interval_ms_is_clamped_to_minimum_100(raw_ms in 0u64..50u64) {
            let profile = profile_with_status("ONLINE", raw_ms);
            let hb = select_heartbeat_mode(Some(&profile));
            prop_assert!(hb.interval_ms >= 100);
        }

        #[test]
        fn interval_ms_preserved_when_above_minimum(ms in 100u64..10_000u64) {
            let profile = profile_with_status("ONLINE", ms);
            let hb = select_heartbeat_mode(Some(&profile));
            prop_assert_eq!(hb.interval_ms, ms);
        }
    }
}
