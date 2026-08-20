use crate::lifecycle::types::{
    AggregateState, Availability, Freshness, HermesGatewayObservation, HudNativeObservation,
    LifecycleSchemaVersion, ObservationMetadata, ObservationSourceKind, Observed, RunningState,
    SystemLifecycleSnapshot,
};
use crate::lifecycle::{observe_required_components, reduce_aggregate_state};
use chrono::Utc;
use serde::Serialize;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

pub const STOP_SESSION_CONFIRMATION: &str = "stop-arda-session";
const SESSION_TARGET: &str = "arda-session.target";
const HUD_UNIT: &str = "arda-hud.service";
const MIRROMERE_UNIT: &str = "arda-mirromere.service";
const HERMES_UNIT: &str = "hermes-gateway.service";
const HUD_BINARY: &str = ".local/lib/arda/hud/arda_hud";
const MIRROMERE_BINARY: &str = ".local/lib/arda/mirromere/arda_mirromere";
const CONTROL_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutcome {
    Started,
    Stopped,
    Recovered,
    AlreadyRunning,
    AlreadyStopped,
}

pub trait LifecycleControl {
    fn execute(&self, action: ControlAction, unit: &str) -> Result<(), String>;
    fn is_active(&self, unit: &str) -> Result<bool, String>;
    fn native_hud_available(&self) -> bool;
    fn native_mirromere_available(&self) -> bool;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemdLifecycleControl;

impl LifecycleControl for SystemdLifecycleControl {
    fn execute(&self, action: ControlAction, unit: &str) -> Result<(), String> {
        if !matches!(
            unit,
            SESSION_TARGET | HUD_UNIT | MIRROMERE_UNIT | HERMES_UNIT
        ) {
            return Err("unit is not allowlisted".to_string());
        }
        let verb = match action {
            ControlAction::Start => "start",
            ControlAction::Stop => "stop",
            ControlAction::Restart => "restart",
        };
        run_systemctl(&[verb, unit])?;
        let expected_active = action != ControlAction::Stop;
        let deadline = Instant::now() + POLL_TIMEOUT;
        while Instant::now() < deadline {
            if self.is_active(unit)? == expected_active {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("bounded lifecycle polling timed out".to_string())
    }

    fn is_active(&self, unit: &str) -> Result<bool, String> {
        if !matches!(
            unit,
            SESSION_TARGET | HUD_UNIT | MIRROMERE_UNIT | HERMES_UNIT
        ) {
            return Err("unit is not allowlisted".to_string());
        }
        run_systemctl_status(&["is-active", "--quiet", unit])
    }

    fn native_hud_available(&self) -> bool {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join(HUD_BINARY).is_file())
            .unwrap_or(false)
    }

    fn native_mirromere_available(&self) -> bool {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join(MIRROMERE_BINARY).is_file())
            .unwrap_or(false)
    }
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    if run_systemctl_status(args)? {
        Ok(())
    } else {
        Err("systemctl action failed".to_string())
    }
}

fn run_systemctl_status(args: &[&str]) -> Result<bool, String> {
    let mut local_args = vec!["--user"];
    local_args.extend_from_slice(args);
    match command_status(&local_args) {
        Ok(true) => Ok(true),
        Ok(false) | Err(_) => {
            let user = std::env::var("USER").map_err(|_| "USER is unavailable".to_string())?;
            let machine = format!("--machine={user}@.host");
            let mut fallback = vec!["--user", machine.as_str()];
            fallback.extend_from_slice(args);
            command_status(&fallback)
        }
    }
}

fn command_status(args: &[&str]) -> Result<bool, String> {
    let mut child = Command::new("systemctl")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "systemctl is unavailable".to_string())?;
    match child
        .wait_timeout(CONTROL_TIMEOUT)
        .map_err(|_| "systemctl wait failed".to_string())?
    {
        Some(status) => Ok(status.success()),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err("systemctl timed out".to_string())
        }
    }
}

pub fn start_arda_session_with<C: LifecycleControl>(control: &C) -> Result<CommandOutcome, String> {
    control.execute(ControlAction::Start, SESSION_TARGET)?;
    Ok(CommandOutcome::Started)
}

pub fn stop_arda_session_with<C: LifecycleControl>(
    control: &C,
    confirmation: &str,
) -> Result<CommandOutcome, String> {
    if confirmation != STOP_SESSION_CONFIRMATION {
        return Err("invalid stop confirmation".to_string());
    }
    control.execute(ControlAction::Stop, SESSION_TARGET)?;
    Ok(CommandOutcome::Stopped)
}

pub fn recover_component_with<C: LifecycleControl>(
    control: &C,
    action_id: &str,
) -> Result<CommandOutcome, String> {
    match action_id {
        "start-arda-session" => control.execute(ControlAction::Start, SESSION_TARGET)?,
        "restart-hermes-gateway" => control.execute(ControlAction::Restart, HERMES_UNIT)?,
        "restart-native-hud" => control.execute(ControlAction::Restart, HUD_UNIT)?,
        "retry-health-check" | "inspect-component" => {}
        _ => return Err("recovery action is not allowlisted".to_string()),
    }
    Ok(CommandOutcome::Recovered)
}

pub fn launch_native_hud_with<C: LifecycleControl>(
    control: &C,
    aggregate_state: AggregateState,
) -> Result<CommandOutcome, String> {
    if aggregate_state != AggregateState::Healthy {
        return Err("required lifecycle health is not healthy".to_string());
    }
    if !control.native_hud_available() {
        return Err("native HUD binary is unavailable".to_string());
    }
    if control.is_active(HUD_UNIT)? {
        return Ok(CommandOutcome::AlreadyRunning);
    }
    control.execute(ControlAction::Start, HUD_UNIT)?;
    Ok(CommandOutcome::Started)
}

pub fn launch_native_mirromere_with<C: LifecycleControl>(
    control: &C,
    aggregate_state: AggregateState,
) -> Result<CommandOutcome, String> {
    if aggregate_state != AggregateState::Healthy {
        return Err("required lifecycle health is not healthy".to_string());
    }
    if !control.native_mirromere_available() {
        return Err("Mirromere binary is unavailable".to_string());
    }
    if control.is_active(MIRROMERE_UNIT)? {
        return Ok(CommandOutcome::AlreadyRunning);
    }
    control.execute(ControlAction::Start, MIRROMERE_UNIT)?;
    Ok(CommandOutcome::Started)
}

pub fn stop_mirromere_with<C: LifecycleControl>(control: &C) -> Result<CommandOutcome, String> {
    if !control.is_active(MIRROMERE_UNIT)? {
        return Ok(CommandOutcome::AlreadyStopped);
    }
    control.execute(ControlAction::Stop, MIRROMERE_UNIT)?;
    Ok(CommandOutcome::Stopped)
}

fn observed<T>(value: T, source_id: &str) -> Observed<T> {
    Observed {
        value,
        observation: ObservationMetadata {
            source: ObservationSourceKind::Systemd,
            source_id: source_id.to_string(),
            observed_at: Utc::now(),
            freshness: Freshness::Fresh,
        },
    }
}

#[tauri::command]
pub fn lifecycle_status() -> SystemLifecycleSnapshot {
    let observed_at = Utc::now();
    let components = observe_required_components(observed_at);
    let aggregate_state = reduce_aggregate_state(&components);
    let gateway = components
        .iter()
        .find(|item| item.component_id == "hermes-gateway");
    let gateway_available = gateway.is_some_and(|item| {
        item.unit.active_state.value == crate::lifecycle::types::ActiveState::Active
    });
    let gateway_health = gateway
        .map(|item| item.protocol_health.clone())
        .unwrap_or_else(|| observed(crate::lifecycle::types::HealthState::Unknown, HERMES_UNIT));
    SystemLifecycleSnapshot {
        schema_version: LifecycleSchemaVersion::V1,
        observed_at,
        aggregate_state,
        components,
        hud_native: hud_status(),
        hermes_gateway: HermesGatewayObservation {
            availability: observed(
                if gateway_available {
                    Availability::Available
                } else {
                    Availability::Unavailable
                },
                HERMES_UNIT,
            ),
            protocol_health: gateway_health,
        },
    }
}

#[tauri::command]
pub fn start_arda_session() -> Result<CommandOutcome, String> {
    start_arda_session_with(&SystemdLifecycleControl)
}

#[tauri::command]
pub fn stop_arda_session(confirmation: String) -> Result<CommandOutcome, String> {
    stop_arda_session_with(&SystemdLifecycleControl, &confirmation)
}

#[tauri::command]
pub fn recover_component(action_id: String) -> Result<CommandOutcome, String> {
    recover_component_with(&SystemdLifecycleControl, &action_id)
}

#[tauri::command]
pub fn launch_native_hud() -> Result<CommandOutcome, String> {
    let status = lifecycle_status();
    launch_native_hud_with(&SystemdLifecycleControl, status.aggregate_state)
}

#[tauri::command]
pub fn launch_native_mirromere() -> Result<CommandOutcome, String> {
    let status = lifecycle_status();
    launch_native_mirromere_with(&SystemdLifecycleControl, status.aggregate_state)
}

#[tauri::command]
pub fn stop_mirromere() -> Result<CommandOutcome, String> {
    stop_mirromere_with(&SystemdLifecycleControl)
}

#[tauri::command]
pub fn mirromere_status() -> HudNativeObservation {
    let control = SystemdLifecycleControl;
    let available = control.native_mirromere_available();
    let running = control.is_active(MIRROMERE_UNIT).ok();
    HudNativeObservation {
        availability: observed(
            if available {
                Availability::Available
            } else {
                Availability::Unavailable
            },
            MIRROMERE_BINARY,
        ),
        running: observed(
            match running {
                Some(true) => RunningState::Running,
                Some(false) => RunningState::Stopped,
                None => RunningState::Unknown,
            },
            MIRROMERE_UNIT,
        ),
    }
}

#[tauri::command]
pub fn hud_status() -> HudNativeObservation {
    let control = SystemdLifecycleControl;
    let available = control.native_hud_available();
    let running = control.is_active(HUD_UNIT).ok();
    HudNativeObservation {
        availability: observed(
            if available {
                Availability::Available
            } else {
                Availability::Unavailable
            },
            HUD_BINARY,
        ),
        running: observed(
            match running {
                Some(true) => RunningState::Running,
                Some(false) => RunningState::Stopped,
                None => RunningState::Unknown,
            },
            HUD_UNIT,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        launch_native_hud_with, launch_native_mirromere_with, recover_component_with,
        start_arda_session_with, stop_arda_session_with, stop_mirromere_with, CommandOutcome,
        ControlAction, LifecycleControl, STOP_SESSION_CONFIRMATION,
    };
    use crate::lifecycle::types::AggregateState;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FixtureControl {
        calls: RefCell<Vec<(ControlAction, String)>>,
        hud_active: bool,
        hud_available: bool,
        mirromere_active: bool,
        mirromere_available: bool,
    }

    impl LifecycleControl for FixtureControl {
        fn execute(&self, action: ControlAction, unit: &str) -> Result<(), String> {
            self.calls.borrow_mut().push((action, unit.to_string()));
            Ok(())
        }

        fn is_active(&self, unit: &str) -> Result<bool, String> {
            Ok(match unit {
                "arda-hud.service" => self.hud_active,
                "arda-mirromere.service" => self.mirromere_active,
                _ => false,
            })
        }

        fn native_hud_available(&self) -> bool {
            self.hud_available
        }

        fn native_mirromere_available(&self) -> bool {
            self.mirromere_available
        }
    }

    #[test]
    fn start_targets_only_arda_session() {
        let control = FixtureControl::default();
        assert_eq!(
            start_arda_session_with(&control).unwrap(),
            CommandOutcome::Started
        );
        assert_eq!(
            control.calls.into_inner(),
            vec![(ControlAction::Start, "arda-session.target".to_string())]
        );
    }

    #[test]
    fn stop_requires_exact_confirmation_and_targets_only_session() {
        let control = FixtureControl::default();
        assert!(stop_arda_session_with(&control, "wrong").is_err());
        assert!(control.calls.borrow().is_empty());
        assert_eq!(
            stop_arda_session_with(&control, STOP_SESSION_CONFIRMATION).unwrap(),
            CommandOutcome::Stopped
        );
        assert_eq!(
            control.calls.into_inner(),
            vec![(ControlAction::Stop, "arda-session.target".to_string())]
        );
    }

    #[test]
    fn recovery_rejects_arbitrary_action_ids() {
        let control = FixtureControl::default();
        assert!(recover_component_with(&control, "run-shell").is_err());
        assert!(control.calls.borrow().is_empty());
        assert_eq!(
            recover_component_with(&control, "restart-hermes-gateway").unwrap(),
            CommandOutcome::Recovered
        );
        assert_eq!(
            control.calls.into_inner(),
            vec![(ControlAction::Restart, "hermes-gateway.service".to_string())]
        );
    }

    #[test]
    fn hud_launch_requires_healthy_runtime_and_native_binary() {
        let unavailable = FixtureControl::default();
        assert!(launch_native_hud_with(&unavailable, AggregateState::Healthy).is_err());
        let available = FixtureControl {
            hud_available: true,
            ..Default::default()
        };
        assert!(launch_native_hud_with(&available, AggregateState::Failed).is_err());
        assert_eq!(
            launch_native_hud_with(&available, AggregateState::Healthy).unwrap(),
            CommandOutcome::Started
        );
    }

    #[test]
    fn repeated_hud_launch_reports_already_running() {
        let control = FixtureControl {
            hud_active: true,
            hud_available: true,
            ..Default::default()
        };
        assert_eq!(
            launch_native_hud_with(&control, AggregateState::Healthy).unwrap(),
            CommandOutcome::AlreadyRunning
        );
        assert!(control.calls.borrow().is_empty());
    }

    #[test]
    fn mirromere_lifecycle_is_explicit_and_closed_means_closed() {
        let unavailable = FixtureControl::default();
        assert!(launch_native_mirromere_with(&unavailable, AggregateState::Healthy).is_err());

        let available = FixtureControl {
            mirromere_available: true,
            ..Default::default()
        };
        assert_eq!(
            launch_native_mirromere_with(&available, AggregateState::Healthy).unwrap(),
            CommandOutcome::Started
        );
        assert_eq!(
            available.calls.borrow().as_slice(),
            &[(ControlAction::Start, "arda-mirromere.service".to_string())]
        );

        let stopped = FixtureControl::default();
        assert_eq!(
            stop_mirromere_with(&stopped).unwrap(),
            CommandOutcome::AlreadyStopped
        );
        assert!(stopped.calls.borrow().is_empty());

        let running = FixtureControl {
            mirromere_active: true,
            ..Default::default()
        };
        assert_eq!(
            stop_mirromere_with(&running).unwrap(),
            CommandOutcome::Stopped
        );
        assert_eq!(
            running.calls.borrow().as_slice(),
            &[(ControlAction::Stop, "arda-mirromere.service".to_string())]
        );
    }
}
