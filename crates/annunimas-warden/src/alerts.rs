// sigil: REPAIR
use sysinfo::System;

type CommandStatusFn = fn(&str, &[&str]) -> std::io::Result<bool>;

pub async fn post_heartbeat(
    client: &reqwest::Client,
    webhook_url: &str,
    node: &str,
    role: &str,
) -> Result<(), reqwest::Error> {
    let payload = serde_json::json!({
        "username": "WARDEN",
        "embeds": [{
            "title": format!("{} // HEARTBEAT", node),
            "color": 0x00d4ff,
            "fields": [
                {"name": "role", "value": role, "inline": true},
                {"name": "uptime", "value": get_uptime(), "inline": true},
                {"name": "load", "value": get_load(), "inline": true},
                {"name": "tailscale", "value": tailscale_status(), "inline": false}
            ]
        }]
    });

    client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn get_uptime() -> String {
    let seconds = System::uptime();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("{}h {}m", hours, minutes)
}

fn get_load() -> String {
    let load = System::load_average();
    format!("{:.2} / {:.2} / {:.2}", load.one, load.five, load.fifteen)
}

fn tailscale_status() -> String {
    command_status_label(run_command_status, "tailscale", &["status"])
}

fn command_status_label(run_status: CommandStatusFn, command: &str, args: &[&str]) -> String {
    match run_status(command, args) {
        Ok(true) => String::from("online"),
        Ok(false) => String::from("degraded"),
        Err(_) => String::from("unavailable"),
    }
}

fn run_command_status(command: &str, args: &[&str]) -> std::io::Result<bool> {
    std::process::Command::new(command)
        .args(args)
        .status()
        .map(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::{command_status_label, get_load, get_uptime};
    use std::io;

    #[test]
    fn uptime_and_load_are_human_readable() {
        let uptime = get_uptime();
        let load = get_load();

        assert!(uptime.contains('h'));
        assert!(uptime.contains('m'));
        assert_eq!(load.matches('/').count(), 2);
    }

    #[test]
    fn command_status_labels_are_stable() {
        fn ok(_: &str, _: &[&str]) -> io::Result<bool> {
            Ok(true)
        }
        fn degraded(_: &str, _: &[&str]) -> io::Result<bool> {
            Ok(false)
        }
        fn unavailable(_: &str, _: &[&str]) -> io::Result<bool> {
            Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
        }

        assert_eq!(command_status_label(ok, "tailscale", &["status"]), "online");
        assert_eq!(
            command_status_label(degraded, "tailscale", &["status"]),
            "degraded"
        );
        assert_eq!(
            command_status_label(unavailable, "tailscale", &["status"]),
            "unavailable"
        );
    }
}
