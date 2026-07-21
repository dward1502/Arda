// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceRole {
    Scout,
    Marketer,
    Analyst,
    Worker,
    Standby,
}

impl std::fmt::Display for DeviceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceRole::Scout => write!(f, "Scout"),
            DeviceRole::Marketer => write!(f, "Marketer"),
            DeviceRole::Analyst => write!(f, "Analyst"),
            DeviceRole::Worker => write!(f, "Worker"),
            DeviceRole::Standby => write!(f, "Standby"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDevice {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip_address: Option<String>,
    pub role: DeviceRole,
    pub previous_role: Option<DeviceRole>,
    pub status: DeviceStatus,
    pub capabilities: Vec<String>,
    pub gpu_available: bool,
    pub gpu_utilization: Option<f32>,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub last_seen: DateTime<Utc>,
    pub last_health_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Online,
    Busy,
    Idle,
    Unhealthy,
    Offline,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Online => write!(f, "Online"),
            DeviceStatus::Busy => write!(f, "Busy"),
            DeviceStatus::Idle => write!(f, "Idle"),
            DeviceStatus::Unhealthy => write!(f, "Unhealthy"),
            DeviceStatus::Offline => write!(f, "Offline"),
        }
    }
}

impl EdgeDevice {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        hostname: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            hostname: hostname.into(),
            ip_address: None,
            role: DeviceRole::Standby,
            previous_role: None,
            status: DeviceStatus::Offline,
            capabilities: Vec::new(),
            gpu_available: false,
            gpu_utilization: None,
            memory_total_mb: 0,
            memory_used_mb: 0,
            last_seen: Utc::now(),
            last_health_check: Utc::now(),
        }
    }

    pub fn with_role(mut self, role: DeviceRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_gpu(mut self, available: bool) -> Self {
        self.gpu_available = available;
        self
    }

    pub fn switch_role(&mut self, new_role: &DeviceRole) {
        self.previous_role = Some(self.role.clone());
        self.role = new_role.clone();
        tracing::info!(
            "Device {} switching from {:?} to {:?}",
            self.name,
            self.previous_role,
            new_role
        );
    }

    pub fn is_healthy(&self) -> bool {
        self.status != DeviceStatus::Offline && self.status != DeviceStatus::Unhealthy
    }

    pub fn memory_percent(&self) -> f32 {
        if self.memory_total_mb == 0 {
            return 0.0;
        }
        (self.memory_used_mb as f32 / self.memory_total_mb as f32) * 100.0
    }

    pub fn format_status(&self) -> String {
        let status_emoji = match self.status {
            DeviceStatus::Online => "🟢",
            DeviceStatus::Busy => "🔴",
            DeviceStatus::Idle => "🟡",
            DeviceStatus::Unhealthy => "⚠️",
            DeviceStatus::Offline => "⚫",
        };

        let gpu_info = if self.gpu_available {
            if let Some(util) = self.gpu_utilization {
                format!(" GPU: {:.0}%", util)
            } else {
                " GPU: ?".to_string()
            }
        } else {
            String::new()
        };

        format!(
            "{} {} | {:?} | RAM: {:.0}%{}",
            status_emoji,
            self.name,
            self.role,
            self.memory_percent(),
            gpu_info
        )
    }
}

pub struct EdgeRegistry {
    devices: std::collections::HashMap<String, EdgeDevice>,
}

impl EdgeRegistry {
    pub fn new() -> Self {
        Self {
            devices: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, device: EdgeDevice) {
        self.devices.insert(device.id.clone(), device);
    }

    pub fn unregister(&mut self, device_id: &str) {
        self.devices.remove(device_id);
    }

    pub fn get(&self, device_id: &str) -> Option<&EdgeDevice> {
        self.devices.get(device_id)
    }

    pub fn get_mut(&mut self, device_id: &str) -> Option<&mut EdgeDevice> {
        self.devices.get_mut(device_id)
    }

    pub fn list_all(&self) -> Vec<&EdgeDevice> {
        self.devices.values().collect()
    }

    pub fn list_by_role(&self, role: &DeviceRole) -> Vec<&EdgeDevice> {
        self.devices.values().filter(|d| &d.role == role).collect()
    }

    pub fn list_available(&self) -> Vec<&EdgeDevice> {
        self.devices
            .values()
            .filter(|d| d.status == DeviceStatus::Idle || d.status == DeviceStatus::Online)
            .collect()
    }

    pub fn find_device_for_role(&self, role: &DeviceRole) -> Option<&EdgeDevice> {
        self.devices
            .values()
            .find(|d| &d.role == role && d.is_healthy())
            .or_else(|| {
                self.devices
                    .values()
                    .find(|d| d.role == DeviceRole::Standby && d.is_healthy())
            })
    }

    pub fn switch_role(&mut self, device_id: &str, new_role: &DeviceRole) -> bool {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.switch_role(new_role);
            true
        } else {
            false
        }
    }

    pub fn format_all_status(&self) -> String {
        if self.devices.is_empty() {
            return "𓃭 Edge Devices: None registered".to_string();
        }

        let mut lines = vec!["𓃭 Edge Device Status".to_string()];
        lines.push("━━━━━━━━━━━━━━━━".to_string());

        for device in self.devices.values() {
            lines.push(device.format_status());
        }

        lines.join("\n")
    }
}

impl Default for EdgeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
