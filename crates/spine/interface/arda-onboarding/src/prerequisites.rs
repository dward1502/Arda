use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::ONBOARDING_PREREQUISITE_CONTRACT;
use crate::helpers::{command_available, command_version, make_prerequisite_check, now_utc};
use crate::types::*;

pub fn build_prerequisite_report(profile: &EnvironmentProfile, root: &Path) -> PrerequisiteReport {
    let mut checks = Vec::new();
    let tool_checks = [
        (
            "tool.git",
            "Git",
            "git",
            &["--version"][..],
            "Install git before clone/update workflows.",
            "git --version",
        ),
        (
            "tool.cargo",
            "Cargo",
            "cargo",
            &["--version"][..],
            "Install Rust/Cargo before building Arda crates.",
            "cargo --version",
        ),
        (
            "tool.rustc",
            "Rust compiler",
            "rustc",
            &["--version"][..],
            "Install rustc through rustup or distro packages.",
            "rustc --version",
        ),
        (
            "tool.node",
            "Node.js",
            "node",
            &["--version"][..],
            "Install Node.js before frontend/Tauri app work.",
            "node --version",
        ),
        (
            "tool.npm",
            "npm",
            "npm",
            &["--version"][..],
            "Install npm or use the repo-supported package manager.",
            "npm --version",
        ),
        (
            "tool.python3",
            "Python 3",
            "python3",
            &["--version"][..],
            "Install Python 3 for local utility servers and scripts.",
            "python3 --version",
        ),
        (
            "tool.systemctl",
            "systemd user tools",
            "systemctl",
            &["--version"][..],
            "Install systemd tools or mark this host as non-systemd/container.",
            "systemctl --version",
        ),
        (
            "tool.tailscale",
            "Tailscale",
            "tailscale",
            &["--version"][..],
            "Install and authenticate Tailscale before fleet peer discovery.",
            "tailscale --version",
        ),
    ];

    for (check_id, title, cmd, args, recommendation, command_hint) in tool_checks {
        let detected = command_version(cmd, args).unwrap_or_else(|| "missing".to_string());
        let mut status = if detected == "missing" {
            "warn"
        } else {
            "pass"
        };
        if check_id == "tool.systemctl"
            && profile.machine_role == "container"
            && detected == "missing"
        {
            status = "warn";
        }
        checks.push(make_prerequisite_check(
            check_id,
            title,
            status,
            if status == "pass" { "low" } else { "medium" },
            detected,
            recommendation,
            Some(command_hint),
        ));
    }

    let pkg_config_available = command_available("pkg-config");
    for (check_id, title, package) in [
        ("tauri.webkit", "Tauri WebKit runtime", "webkit2gtk-4.1"),
        ("tauri.libsoup", "Tauri libsoup runtime", "libsoup-3.0"),
        ("build.openssl", "OpenSSL development package", "openssl"),
    ] {
        let found = pkg_config_available
            && Command::new("pkg-config")
                .args(["--exists", package])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
        checks.push(make_prerequisite_check(
            check_id,
            title,
            if found { "pass" } else { "warn" },
            "medium",
            if found {
                format!("pkg-config found {package}")
            } else {
                format!("pkg-config missing {package}")
            },
            "Install host GUI/build dependencies before native ARDA HUD validation.",
            Some(&format!("pkg-config --exists {package}")),
        ));
    }

    for (check_id, title, path, recommendation) in [
        (
            "repo.agents",
            "Agent instructions",
            root.join("AGENTS.md"),
            "Keep AGENTS.md with install-specific operating instructions.",
        ),
        (
            "repo.workspace",
            "Rust workspace manifest",
            root.join("Cargo.toml"),
            "Run from a complete Arda checkout.",
        ),
        (
            "repo.template",
            "Config template",
            root.join("config/arda.template.toml"),
            "Restore config template before setup proposal generation.",
        ),
        (
            "repo.providers",
            "Charon providers config",
            root.join("config/charon.providers.toml"),
            "Restore provider matrix before Charon onboarding.",
        ),
        (
            "repo.console",
            "Onboarding console",
            root.join("apps/onboarding-console/index.html"),
            "Keep setup console assets available for non-technical onboarding.",
        ),
        (
            "path.config_dir",
            "Private config directory",
            PathBuf::from(&profile.paths.config_dir.value),
            "Create this directory only after human approval.",
        ),
        (
            "path.data_dir",
            "Runtime data directory",
            PathBuf::from(&profile.paths.data_dir.value),
            "Create this directory only after human approval.",
        ),
        (
            "path.cache_dir",
            "Runtime cache directory",
            PathBuf::from(&profile.paths.cache_dir.value),
            "Create this directory only after human approval.",
        ),
    ] {
        let exists = path.exists();
        checks.push(make_prerequisite_check(
            check_id,
            title,
            if exists { "pass" } else { "warn" },
            if check_id.starts_with("repo.") {
                "high"
            } else {
                "medium"
            },
            if exists {
                format!("present: {}", path.display())
            } else {
                format!("missing: {}", path.display())
            },
            recommendation,
            None,
        ));
    }

    let mut summary = BTreeMap::new();
    for check in &checks {
        *summary.entry(check.status.clone()).or_insert(0) += 1;
    }

    PrerequisiteReport {
        contract: ONBOARDING_PREREQUISITE_CONTRACT.to_string(),
        generated_at_utc: now_utc(),
        profile: profile.profile.clone(),
        machine_role: profile.machine_role.clone(),
        checks,
        summary,
    }
}
