#![cfg(feature = "full-cli")]
use arda_varda::transport::expand_home;
use arda_varda::AthenaAgent;
use arda_core::config::Config;
use arda_core::llm::{LlmProvider, OpenAiCompatibleProvider};
use arda_core::router::Router;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) fn load_config(path: &str) -> Config {
    Config::load(path).unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({}), using defaults", e);
        Config::default()
    })
}

pub(crate) fn build_provider(config: &Config) -> Arc<dyn LlmProvider> {
    let llm = &config.llm;

    if let Some(provider_config) = llm.providers.get(&llm.default_provider) {
        let api_key = provider_config.resolve_api_key();
        Arc::new(OpenAiCompatibleProvider::new(
            &llm.default_provider,
            &provider_config.base_url,
            api_key,
            &provider_config.default_model,
        ))
    } else {
        let fallback_model = llm
            .providers
            .values()
            .next()
            .map(|p| p.default_model.as_str())
            .unwrap_or("qwen2.5-coder:3b");
        tracing::warn!(
            "Provider '{}' not found in config, falling back to local Ollama model '{}'",
            llm.default_provider,
            fallback_model
        );
        Arc::new(OpenAiCompatibleProvider::ollama(fallback_model))
    }
}

pub(crate) fn build_router(
    llm: Arc<dyn LlmProvider>,
    model_routes: std::collections::HashMap<String, String>,
) -> anyhow::Result<Router> {
    let mut router = Router::new();
    router.register(Box::new(
        AthenaAgent::with_model_routes(llm, model_routes)
            .context("failed to initialize Athena agent")?,
    ));
    router.register(Box::new(
        arda_mandos::HadesAgent::new().context("failed to initialize Hades agent")?,
    ));
    router.register(Box::new(
        arda_orome::HermesAgent::new().context("failed to initialize Hermes agent")?,
    ));
    Ok(router)
}

pub(crate) fn load_env_files() -> anyhow::Result<()> {
    for path in ["config/.env", ".env"] {
        let path_ref = Path::new(path);
        if !path_ref.exists() {
            continue;
        }
        let content = fs::read_to_string(path_ref)?;
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let without_export = line.strip_prefix("export ").unwrap_or(line).trim();
            let Some((key, value)) = without_export.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() || std::env::var_os(key).is_some() {
                continue;
            }
            let mut value = value.trim().to_string();
            if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                value = value[1..value.len() - 1].to_string();
            }
            std::env::set_var(key, value);
        }
    }
    Ok(())
}

pub(crate) fn set_runtime_defaults() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let homes = [
        ("ARDA_ATHENA_HOME", cwd.join("data/athena")),
        ("ARDA_PROMETHEUS_HOME", cwd.join("data/prometheus")),
        ("ARDA_MANWE_HOME", cwd.join("data/charon")),
        ("ARDA_HADES_HOME", cwd.join("data/hades")),
        ("ARDA_HERMES_HOME", cwd.join("data/hermes")),
        ("ARDA_MNEMOSYNE_HOME", cwd.join("data/mnemosyne")),
        ("ARDA_APOLLO_HOME", cwd.join("data/apollo")),
        ("ARDA_PLUTUS_HOME", cwd.join("data/plutus")),
        ("ARDA_ORACLE_HOME", cwd.join("data/oracle")),
    ];
    for (key, value) in homes {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value.to_string_lossy().to_string());
        }
    }

    for (socket_key, file) in [
        ("ARDA_ATHENA_SOCKET", "athena.sock"),
        ("ARDA_PROMETHEUS_SOCKET", "prometheus.sock"),
        ("ARDA_MANWE_SOCKET", "charon.sock"),
        ("ARDA_HADES_SOCKET", "hades.sock"),
        ("ARDA_HERMES_SOCKET", "hermes.sock"),
        ("ARDA_MNEMOSYNE_SOCKET", "mnemosyne.sock"),
        ("ARDA_APOLLO_SOCKET", "apollo.sock"),
        ("ARDA_PLUTUS_SOCKET", "plutus.sock"),
        ("ARDA_ORACLE_SOCKET", "oracle.sock"),
    ] {
        let runtime_default = default_runtime_socket(file);
        match std::env::var_os(socket_key) {
            Some(current) if !is_legacy_socket_path(&cwd, &current.to_string_lossy()) => continue,
            _ => std::env::set_var(socket_key, runtime_default),
        }
    }

    if std::env::var_os("ARDA_ILLUVATAR_DISCORD_USER").is_none() {
        std::env::set_var("ARDA_ILLUVATAR_DISCORD_USER", "illuvatar");
    }
    if std::env::var_os("ARDA_TASK_QUEUE_PATH").is_none() {
        std::env::set_var(
            "ARDA_TASK_QUEUE_PATH",
            cwd.join("core/projects/tasks/queue.jsonl")
                .to_string_lossy()
                .to_string(),
        );
    }
    if std::env::var_os("ARDA_DAILY_QUEUE_PATH").is_none() {
        std::env::set_var(
            "ARDA_DAILY_QUEUE_PATH",
            cwd.join("core/queue/queue.jsonl")
                .to_string_lossy()
                .to_string(),
        );
    }
    if std::env::var_os("ARDA_WARDEN_QUEUE_PATH").is_none() {
        std::env::set_var(
            "ARDA_WARDEN_QUEUE_PATH",
            cwd.join("data/warden/informant_queue.jsonl")
                .to_string_lossy()
                .to_string(),
        );
    }
    Ok(())
}

fn is_legacy_socket_path(cwd: &Path, value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    normalized.starts_with("data/") && normalized.ends_with(".sock")
        || normalized
            .strip_prefix(&format!("{}/", cwd.to_string_lossy().replace('\\', "/")))
            .is_some_and(|suffix| suffix.starts_with("data/") && suffix.ends_with(".sock"))
}

fn runtime_socket_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ARDA_RUNTIME_SOCKET_DIR") {
        let path = expand_home(&dir.to_string_lossy());
        if std::fs::create_dir_all(&path).is_ok() {
            return path;
        }
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(dir).join("arda");
        if std::fs::create_dir_all(&path).is_ok() {
            return path;
        }
    }
    if let Some(uid) = std::env::var_os("UID") {
        let path = PathBuf::from(format!("/run/user/{}/arda", uid.to_string_lossy()));
        if std::fs::create_dir_all(&path).is_ok() {
            return path;
        }
    }
    let fallback = PathBuf::from("data/run");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

pub(crate) fn default_runtime_socket(file: &str) -> String {
    runtime_socket_dir()
        .join(file)
        .to_string_lossy()
        .to_string()
}
