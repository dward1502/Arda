use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayDescriptor {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplaySelection {
    pub schema_version: String,
    pub display_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DisplayState {
    pub displays: Vec<DisplayDescriptor>,
    pub selected_display_id: Option<String>,
    pub projected: bool,
    pub veil_reason: Option<String>,
}

fn monitor_id(monitor: &Monitor) -> String {
    let position = monitor.position();
    let size = monitor.size();
    format!(
        "{}:{}:{}:{}x{}",
        monitor.name().map_or("unnamed", String::as_str),
        position.x,
        position.y,
        size.width,
        size.height
    )
}

fn descriptor(monitor: &Monitor, primary_id: Option<&str>) -> DisplayDescriptor {
    let position = monitor.position();
    let size = monitor.size();
    let id = monitor_id(monitor);
    DisplayDescriptor {
        name: monitor
            .name()
            .map_or("Unnamed display", String::as_str)
            .to_string(),
        is_primary: primary_id.is_some_and(|primary| primary == id),
        id,
        position: (position.x, position.y),
        size: (size.width, size.height),
        scale_factor: monitor.scale_factor(),
    }
}

pub fn resolve_selected_display<'a>(
    displays: &'a [DisplayDescriptor],
    display_id: &str,
) -> Result<&'a DisplayDescriptor, String> {
    let display = displays
        .iter()
        .find(|display| display.id == display_id)
        .ok_or_else(|| "selected display is unavailable; projection remains veiled".to_string())?;
    if display.is_primary {
        return Err(
            "primary display selection is forbidden; choose an explicit non-primary display"
                .to_string(),
        );
    }
    Ok(display)
}

fn selection_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "HOME and XDG_CONFIG_HOME are unavailable".to_string())?;
    Ok(base.join("arda").join("mirromere-display.json"))
}

pub fn load_selection() -> Result<Option<DisplaySelection>, String> {
    let path = selection_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let selection: DisplaySelection = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    if selection.schema_version != "arda.mirromere-display.v1" {
        return Err("unsupported Mirromere display selection schema".to_string());
    }
    Ok(Some(selection))
}

pub fn save_selection(display_id: &str) -> Result<(), String> {
    let path = selection_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "display selection path has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&DisplaySelection {
        schema_version: "arda.mirromere-display.v1".to_string(),
        display_id: display_id.to_string(),
    })
    .map_err(|error| error.to_string())?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("failed to publish {}: {error}", path.display()))
}

pub fn inventory(app: &AppHandle) -> Result<(Vec<Monitor>, Vec<DisplayDescriptor>), String> {
    let monitors = app
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let primary_id = app
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(monitor_id);
    let displays = monitors
        .iter()
        .map(|monitor| descriptor(monitor, primary_id.as_deref()))
        .collect();
    Ok((monitors, displays))
}

fn project_window(
    window: &WebviewWindow,
    monitor: &Monitor,
    expected_id: &str,
) -> Result<(), String> {
    window
        .set_fullscreen(false)
        .map_err(|error| error.to_string())?;
    let position = monitor.position();
    let size = monitor.size();
    window
        .set_position(PhysicalPosition::new(position.x, position.y))
        .map_err(|error| error.to_string())?;
    window
        .set_size(PhysicalSize::new(size.width, size.height))
        .map_err(|error| error.to_string())?;
    let observed = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "window monitor could not be verified".to_string())?;
    if monitor_id(&observed) != expected_id {
        return Err("window move did not resolve to the selected display".to_string());
    }
    window
        .set_fullscreen(true)
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn apply_selection(app: &AppHandle, selected: Option<&str>) -> Result<DisplayState, String> {
    let (monitors, displays) = inventory(app)?;
    let Some(display_id) = selected else {
        return Ok(DisplayState {
            displays,
            selected_display_id: None,
            projected: false,
            veil_reason: Some(
                "Projection is veiled until an operator selects a non-primary display.".to_string(),
            ),
        });
    };
    let resolved = match resolve_selected_display(&displays, display_id) {
        Ok(display) => display,
        Err(reason) => {
            return Ok(DisplayState {
                displays,
                selected_display_id: Some(display_id.to_string()),
                projected: false,
                veil_reason: Some(reason),
            })
        }
    };
    let monitor = monitors
        .iter()
        .find(|monitor| monitor_id(monitor) == resolved.id)
        .ok_or_else(|| "selected display vanished during projection".to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Mirromere main window is unavailable".to_string())?;
    project_window(&window, monitor, display_id)?;
    Ok(DisplayState {
        displays,
        selected_display_id: Some(display_id.to_string()),
        projected: true,
        veil_reason: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display(id: &str, primary: bool) -> DisplayDescriptor {
        DisplayDescriptor {
            id: id.into(),
            name: id.into(),
            is_primary: primary,
            position: (0, 0),
            size: (1920, 1080),
            scale_factor: 1.0,
        }
    }
    #[test]
    fn rejects_primary_display() {
        assert!(
            resolve_selected_display(&[display("primary", true)], "primary")
                .unwrap_err()
                .contains("forbidden")
        );
    }
    #[test]
    fn rejects_unavailable_display() {
        assert!(
            resolve_selected_display(&[display("secondary", false)], "missing")
                .unwrap_err()
                .contains("unavailable")
        );
    }
    #[test]
    fn accepts_explicit_non_primary_display() {
        assert_eq!(
            resolve_selected_display(&[display("secondary", false)], "secondary")
                .unwrap()
                .id,
            "secondary"
        );
    }
}
