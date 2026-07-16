use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use sysinfo::System;

use crate::onboarding::helpers::{command_output, get_host_name, now_utc};
use crate::onboarding::types::*;

pub fn device_scan() -> DeviceScan {
    let mut system = System::new_all();
    system.refresh_all();
    let mut peers = serde_json::Map::new();

    if let Some(raw) = command_output("tailscale", &["status", "--json"]) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            if let Some(peer_array) = parsed.get("Peer").or_else(|| parsed.get("Peers")) {
                if let Some(peer_map) = peer_array.as_object() {
                    let mut host_counts = BTreeMap::<String, usize>::new();
                    for peer in peer_map.values() {
                        let host = peer
                            .get("HostName")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        *host_counts.entry(host).or_insert(0) += 1;
                    }

                    let summaries = peer_map
                        .values()
                        .map(|peer| {
                            let host = peer
                                .get("HostName")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            let online = peer
                                .get("Online")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            let active = peer
                                .get("Active")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            let duplicate_name = host_counts.get(host).copied().unwrap_or(0) > 1;
                            let posture = if online && active {
                                "active_online"
                            } else if online {
                                "online_idle"
                            } else if duplicate_name {
                                "offline_duplicate_name"
                            } else {
                                "offline"
                            };

                            json!({
                                "host": host,
                                "dns_name": peer.get("DNSName").and_then(Value::as_str).unwrap_or(""),
                                "online": online,
                                "active": active,
                                "posture": posture,
                                "duplicate_host_name": duplicate_name,
                                "tailscale_ips": peer.get("TailscaleIPs").cloned().unwrap_or_else(|| json!([])),
                                "last_seen": peer.get("LastSeen").and_then(Value::as_str).unwrap_or(""),
                                "last_handshake": peer.get("LastHandshake").and_then(Value::as_str).unwrap_or(""),
                                "os": peer.get("OS").and_then(Value::as_str).unwrap_or("unknown"),
                            })
                        })
                        .collect::<Vec<_>>();

                    let active_online = summaries
                        .iter()
                        .filter(|peer| {
                            peer.get("posture").and_then(Value::as_str) == Some("active_online")
                        })
                        .count();
                    let offline_duplicate_names = summaries
                        .iter()
                        .filter(|peer| {
                            peer.get("posture").and_then(Value::as_str)
                                == Some("offline_duplicate_name")
                        })
                        .count();
                    peers.insert(
                        "tailscale_peer_summary".to_string(),
                        json!({
                            "total": summaries.len(),
                            "active_online": active_online,
                            "offline_duplicate_names": offline_duplicate_names,
                            "peers": summaries,
                        }),
                    );
                }
            }
            if let Some(self_node) = parsed.get("Self") {
                peers.insert(
                    "tailscale_self".to_string(),
                    json!({
                        "host": self_node.get("HostName").and_then(Value::as_str).unwrap_or("unknown"),
                        "dns_name": self_node.get("DNSName").and_then(Value::as_str).unwrap_or(""),
                        "online": self_node.get("Online").and_then(Value::as_bool).unwrap_or(false),
                        "active": self_node.get("Active").and_then(Value::as_bool).unwrap_or(false),
                        "tailscale_ips": self_node.get("TailscaleIPs").cloned().unwrap_or_else(|| json!([])),
                        "os": self_node.get("OS").and_then(Value::as_str).unwrap_or("unknown"),
                    }),
                );
            }
        }
        peers.insert("tailscale_raw_available".to_string(), Value::Bool(true));
    }

    let host = get_host_name();
    let runtime = json!({
        "cpu_count": system.cpus().len(),
        "total_memory_bytes": system.total_memory() * 1024,
        "available_memory_bytes": system.available_memory() * 1024,
        "swap_free_bytes": system.free_swap() * 1024,
        "swap_total_bytes": system.total_swap() * 1024,
        "process_count": system.processes().len(),
        "uptime_seconds": System::uptime(),
    });
    let capabilities = json!({
        "has_systemctl": command_output("systemctl", &["--version"]).is_some(),
        "has_tailscale": command_output("tailscale", &["--version"]).is_some(),
        "has_node": command_output("node", &["--version"]).is_some(),
        "has_cargo": command_output("cargo", &["--version"]).is_some(),
        "has_python3": command_output("python3", &["--version"]).is_some(),
        "container_hint": Path::new("/run/.containerenv").exists() || Path::new(".dockerenv").exists(),
    });
    let container_hint =
        Path::new("/run/.containerenv").exists() || Path::new(".dockerenv").exists();

    DeviceScan {
        generated_at_utc: now_utc(),
        host,
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        container_hint,
        tailscale: Value::Object(peers),
        runtime,
        capabilities,
    }
}
