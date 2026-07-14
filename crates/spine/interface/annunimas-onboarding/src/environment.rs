use anyhow::Result;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::helpers::{
    canonical_home, check_url_health, command_output, make_local_model_default, make_path_value,
    make_url_value, now_utc,
};
use crate::{
    EndpointSection, EnvironmentProfile, OperatorProfile, PathsSection, SafetySection,
    SystemdSection, UrlValue, ValueSource,
};

const CONTRACT_ID: &str = "annunimas.environment_profile.v1";
const ENV_FILE_PATTERN: &str = "%h/.config/annunimas/annunimas.env";
const PROFILE_MACHINE_ROLES: [&str; 7] = [
    "workstation",
    "server",
    "pi-citadel",
    "laptop",
    "cloud-node",
    "container",
    "unknown",
];

pub fn workspace_root() -> PathBuf {
    env::var("ANNUNIMAS_ROOT")
        .map(PathBuf::from)
        .or_else(|_| {
            if let Ok(out) = Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .current_dir(".")
                .output()
            {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(PathBuf::from(path));
                    }
                }
            }
            std::env::current_dir()
        })
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn sanitize_machine_role(role: &str) -> String {
    let normalized = role.trim().to_lowercase();
    if PROFILE_MACHINE_ROLES
        .iter()
        .any(|allowed| normalized == *allowed)
    {
        normalized
    } else {
        "unknown".to_string()
    }
}

fn detect_machine_role(root: &Path, machine_role_override: Option<&str>) -> String {
    if let Some(role) = machine_role_override {
        return sanitize_machine_role(role);
    }
    if let Ok(role) = env::var("ANNUNIMAS_MACHINE_ROLE") {
        return sanitize_machine_role(&role);
    }
    let host = env::var("HOSTNAME").unwrap_or_else(|_| String::new());
    if root.join(".dockerenv").exists() || Path::new("/run/.containerenv").exists() {
        return "container".to_string();
    }
    let hostname = host.to_lowercase();
    if hostname.contains("pi") {
        return "pi-citadel".to_string();
    }
    if env::var("CLOUD_REGION").is_ok() || env::var("AWS_REGION").is_ok() {
        return "cloud-node".to_string();
    }
    if env::var("LAPTOP_VENDOR").is_ok() {
        return "laptop".to_string();
    }
    "unknown".to_string()
}

pub fn build_environment_profile(
    root_override: Option<&Path>,
    profile_override: Option<&str>,
    machine_role_override: Option<&str>,
) -> Result<EnvironmentProfile> {
    let ann_root = root_override
        .map(Path::to_path_buf)
        .unwrap_or_else(workspace_root);
    let home = canonical_home(&env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| {
        ann_root
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .to_path_buf()
    }));
    let mut profile = match profile_override {
        Some(p) => p.to_string(),
        None => env::var("ANNUNIMAS_PROFILE").unwrap_or_else(|_| "local".to_string()),
    };
    if !PROFILE_MACHINE_ROLES.iter().any(|role| role == &profile) {
        profile = "local".to_string();
    }
    let mut missing_gates = Vec::new();

    let config_dir = env::var("ANNUNIMAS_CONFIG_DIR")
        .ok()
        .map(|v| PathBuf::from(v))
        .unwrap_or_else(|| home.join(".config").join("annunimas"));
    let data_dir = env::var("ANNUNIMAS_DATA_DIR")
        .ok()
        .map(|v| PathBuf::from(v))
        .unwrap_or_else(|| home.join(".local/share/annunimas"));
    let cache_dir = env::var("ANNUNIMAS_CACHE_DIR")
        .ok()
        .map(|v| PathBuf::from(v))
        .unwrap_or_else(|| home.join(".cache/annunimas"));
    let runtime_dir = env::var("ANNUNIMAS_RUNTIME_DIR")
        .ok()
        .map(|v| PathBuf::from(v))
        .unwrap_or_else(|| {
            env::var("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    PathBuf::from(format!(
                        "/run/user/{}",
                        env::var("UID").unwrap_or_else(|_| "0".into())
                    ))
                })
                .join("annunimas")
        });
    let build_cache_root = env::var("ANNUNIMAS_BUILD_CACHE_ROOT")
        .ok()
        .map(|v| PathBuf::from(v))
        .or_else(|| Some(home.join(".cache/annunimas-build")));

    let charon_base = env::var("CHARON_BASE_URL")
        .ok()
        .or_else(|| env::var("ANNUNIMAS_CHARON_BASE_URL").ok());
    let hermes_base = env::var("HERMES_BASE_URL")
        .ok()
        .or_else(|| env::var("ANNUNIMAS_HERMES_BASE_URL").ok());
    let arda_hud = env::var("ARDA_HUD_URL")
        .ok()
        .or_else(|| env::var("ANNUNIMAS_ARDA_HUD_URL").ok());
    let local_model_base = env::var("LOCAL_MODEL_BASE_URL")
        .ok()
        .or_else(|| env::var("ANNUNIMAS_LOCAL_MODEL_BASE_URL").ok());
    let local_model_default = env::var("LOCAL_MODEL_DEFAULT").ok();
    let litellm_proxy = env::var("LITELLM_PROXY_URL")
        .ok()
        .or_else(|| env::var("ANNUNIMAS_LITELLM_PROXY_URL").ok());
    let crawl4ai = env::var("ANNUNIMAS_CRAWL4AI_URL").ok();
    let search_runtime = env::var("ANNUNIMAS_SEARCH_RUNTIME_URL").ok();

    if charon_base.is_none() {
        missing_gates.push("CHARON_BASE_URL".to_string());
    }
    if hermes_base.is_none() {
        missing_gates.push("HERMES_BASE_URL".to_string());
    }

    let mut agent_homes = BTreeMap::new();
    let root_agent_env = [
        ("athena", "ANNUNIMAS_ATHENA_HOME"),
        ("prometheus", "ANNUNIMAS_PROMETHEUS_HOME"),
        ("charon", "ANNUNIMAS_CHARON_HOME"),
        ("hades", "ANNUNIMAS_HADES_HOME"),
        ("hermes", "ANNUNIMAS_HERMES_HOME"),
        ("mnemosyne", "ANNUNIMAS_MNEMOSYNE_HOME"),
        ("apollo", "ANNUNIMAS_APOLLO_HOME"),
        ("oracle", "ANNUNIMAS_ORACLE_HOME"),
    ];
    for (name, env_key) in root_agent_env {
        let val = env::var(env_key)
            .unwrap_or_else(|_| format!("{}/{}", data_dir.to_string_lossy(), name));
        agent_homes.insert(
            name.to_string(),
            make_path_value(PathBuf::from(val), ValueSource::Environment, &home),
        );
    }

    let mut sockets = BTreeMap::new();
    let mut socket_keys = Vec::new();
    socket_keys.extend_from_slice(&[
        ("athena", "ANNUNIMAS_ATHENA_SOCKET"),
        ("prometheus", "ANNUNIMAS_PROMETHEUS_SOCKET"),
        ("charon", "ANNUNIMAS_CHARON_SOCKET"),
        ("hades", "ANNUNIMAS_HADES_SOCKET"),
        ("hermes", "ANNUNIMAS_HERMES_SOCKET"),
        ("mnemosyne", "ANNUNIMAS_MNEMOSYNE_SOCKET"),
        ("apollo", "ANNUNIMAS_APOLLO_SOCKET"),
        ("oracle", "ANNUNIMAS_ORACLE_SOCKET"),
        ("plutus", "ANNUNIMAS_PLUTUS_SOCKET"),
    ]);
    for (name, env_key) in socket_keys {
        let fallback = runtime_dir.join(format!("{name}.sock"));
        let path = if let Ok(val) = env::var(env_key) {
            PathBuf::from(val)
        } else {
            fallback
        };
        let source = if env::var(env_key).is_ok() {
            ValueSource::Environment
        } else {
            ValueSource::Detected
        };
        sockets.insert(name.to_string(), make_path_value(path, source, &home));
    }

    let mut endpoints = EndpointSection {
        charon_base_url: None,
        hermes_base_url: None,
        arda_hud_url: None,
        local_model_base_url: None,
        local_model_default: None,
        litellm_proxy_url: None,
        crawl4ai_url: None,
        search_runtime_url: None,
    };
    if let Some(url) = charon_base {
        endpoints.charon_base_url = Some(UrlValue {
            value: url.clone(),
            source: ValueSource::Environment,
            health: Some(check_url_health(&url.clone())),
        });
    }
    if let Some(url) = hermes_base {
        endpoints.hermes_base_url = Some(UrlValue {
            value: url.clone(),
            source: ValueSource::Environment,
            health: Some(check_url_health(&url.clone())),
        });
    }
    if let Some(url) = arda_hud {
        endpoints.arda_hud_url = Some(make_url_value(url, ValueSource::Environment));
    }
    if let Some(url) = local_model_base {
        endpoints.local_model_base_url = Some(make_url_value(url, ValueSource::Environment));
    }
    if let Some(value) = local_model_default {
        endpoints.local_model_default =
            Some(make_local_model_default(value, ValueSource::Environment));
    }
    if let Some(url) = litellm_proxy {
        endpoints.litellm_proxy_url = Some(make_url_value(url, ValueSource::Environment));
    }
    if let Some(url) = crawl4ai {
        endpoints.crawl4ai_url = Some(make_url_value(url, ValueSource::Environment));
    }
    if let Some(url) = search_runtime {
        endpoints.search_runtime_url = Some(make_url_value(url, ValueSource::Environment));
    }

    let operator_user = env::var("ANNUNIMAS_USER").ok();
    let autonomy_posture =
        env::var("ANNUNIMAS_AUTONOMY_POSTURE").unwrap_or_else(|_| "read_only".to_string());
    let mutation_gate = env::var("ANNUNIMAS_MUTATION_REQUIRES_HUMAN_GATE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let profile = EnvironmentProfile {
        contract: CONTRACT_ID.to_string(),
        generated_at: now_utc(),
        profile: profile.to_string(),
        operator: Some(OperatorProfile {
            annunimas_user: operator_user,
            source: Some(ValueSource::Environment),
        }),
        machine_role: detect_machine_role(&ann_root, machine_role_override),
        paths: PathsSection {
            annunimas_root: make_path_value(ann_root.clone(), ValueSource::Detected, &home),
            home: make_path_value(home.clone(), ValueSource::Environment, &home),
            config_dir: make_path_value(config_dir, ValueSource::Environment, &home),
            data_dir: make_path_value(data_dir, ValueSource::Environment, &home),
            cache_dir: make_path_value(cache_dir, ValueSource::Environment, &home),
            runtime_dir: make_path_value(runtime_dir, ValueSource::Environment, &home),
            build_cache_root: build_cache_root
                .map(|path| make_path_value(path, ValueSource::Environment, &home)),
            agent_homes,
            sockets,
        },
        endpoints,
        systemd: SystemdSection {
            environment_file_pattern: ENV_FILE_PATTERN.to_string(),
            user_units_available: command_output("systemctl", &["--version"]).is_some(),
            notes: vec![
                "Mutations remain receipt-only in slice 1".to_string(),
                "Paths prefer XDG/env precedence and avoid hard-coded /var/home names.".to_string(),
            ],
        },
        safety: SafetySection {
            autonomy_posture,
            mutation_requires_human_gate: mutation_gate,
            destructive_allowed_by_default: false,
        },
        missing_gates,
        receipts: vec![],
    };
    Ok(profile)
}
