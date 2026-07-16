use chrono::Utc;
use std::env;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::onboarding::constants::ONBOARDING_APPLY_RESULT_CONTRACT;
use crate::onboarding::types::{
    ApplyResult, ApprovalReceipt, LocalModelDefaultValue, PathValue, PrerequisiteCheck,
    PrivateConfigEntry, ProviderSignupHint, UrlValue, ValueSource,
};

pub(crate) fn action_receipt_path(root: &Path, action_id: &str) -> String {
    root.join("core/state")
        .join(format!("{action_id}.json"))
        .to_string_lossy()
        .to_string()
}

pub(crate) fn action_is_approved(receipt: Option<&ApprovalReceipt>, action_id: &str) -> bool {
    match receipt {
        Some(receipt) => {
            if !receipt.approved {
                return false;
            }
            if receipt.approved_scope.iter().any(|entry| entry == "all") {
                return true;
            }
            receipt
                .approved_scope
                .iter()
                .any(|entry| entry == action_id)
        }
        None => false,
    }
}

pub(crate) fn make_apply_result(action_id: &str, execute: bool, result: &str) -> ApplyResult {
    ApplyResult {
        contract: ONBOARDING_APPLY_RESULT_CONTRACT.to_string(),
        action: action_id.to_string(),
        generated_at_utc: now_utc(),
        execute,
        result: result.to_string(),
    }
}

pub fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub(crate) fn now_run_id() -> String {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

pub(crate) fn today_stamp() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub(crate) fn canonical_home(home: &Path) -> PathBuf {
    if home.to_string_lossy().is_empty() {
        return PathBuf::from("/");
    }
    home.to_path_buf()
}

pub(crate) fn parse_url_host(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .and_then(|host_port| host_port.split(':').next())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

pub(crate) fn infer_provider_profile(
    provider_id: &str,
    access_tier: Option<&str>,
    base_url: Option<&str>,
) -> (String, String) {
    let id = provider_id.to_lowercase();
    let tier = access_tier.unwrap_or("unknown").to_lowercase();
    let locality = if id.starts_with("edge_")
        || id == "edge_worker_light"
        || id == "edge_core"
        || id == "edge_backbone"
        || id == "edge_laptop"
        || id == "edge_guardhouse"
        || tier == "local"
    {
        "local".to_string()
    } else if tier.contains("mixed") || id == "openrouter" || id == "litellm_gateway" {
        "aggregator".to_string()
    } else if let Some(host) = base_url.and_then(parse_url_host) {
        if host == "localhost"
            || host == "127.0.0.1"
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || host.starts_with("172.")
            || host.ends_with(".local")
        {
            "local".to_string()
        } else {
            "cloud".to_string()
        }
    } else {
        "unknown".to_string()
    };

    let route_class = if locality == "local" {
        "local_route".to_string()
    } else if locality == "aggregator" {
        "aggregated_cloud".to_string()
    } else if locality == "cloud" {
        "cloud_direct".to_string()
    } else {
        "unknown_route".to_string()
    };

    (locality, route_class)
}

pub(crate) fn infer_provider_class(locality: &str, access_tier: Option<&str>) -> String {
    match locality {
        "local" => "local".to_string(),
        "aggregator" => "aggregator".to_string(),
        "cloud" => "cloud".to_string(),
        _ => {
            let tier = access_tier.unwrap_or("").to_lowercase();
            if tier.contains("free") {
                "free".to_string()
            } else {
                "unknown".to_string()
            }
        }
    }
}

pub(crate) fn infer_payment_status(
    access_tier: Option<&str>,
    requires_key: bool,
    model_ids: &[String],
) -> String {
    if access_tier.unwrap_or("").contains("free")
        || model_ids
            .iter()
            .any(|m| m.contains(":free") || m.contains("-free") || m.ends_with("/free"))
    {
        "free".to_string()
    } else if requires_key {
        "paid".to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn make_signup_hint(provider_id: &str, env_key: Option<&str>) -> ProviderSignupHint {
    let mut steps = vec![format!(
        "Collect setup docs for {provider_id} before enabling dependent services."
    )];
    let normalized = provider_id.to_lowercase();
    match normalized.as_str() {
        "opencode" => {
            steps.push("Create an OpenCode account and API key.".to_string());
            steps.push("Set OPENCODE_API_KEY in ~/.config/arda/arda.env".to_string());
        }
        "openrouter" => {
            steps.push("Open OpenRouter and generate an API key.".to_string());
            steps.push("Set OPENROUTER_API_KEY in ~/.config/arda/arda.env".to_string());
        }
        "openai" | "openai_sub" => {
            steps.push(
                "OpenAI API keys: create key and billing/usage caps where needed.".to_string(),
            );
            steps.push("Set OPENAI_API_KEY in local env file.".to_string());
        }
        "google" => {
            steps.push("Enable Gemini API and generate key from Google AI Studio.".to_string());
            steps.push("Set GEMINI_API_KEY in local env file.".to_string());
        }
        "mistral" => {
            steps.push("Create Mistral API key.".to_string());
            steps.push("Set MISTRAL_API_KEY in local arda.env.".to_string());
        }
        "groq" => {
            steps.push("Create Groq API key from Groq console.".to_string());
            steps.push("Set GROQ_API_KEY in local arda.env.".to_string());
        }
        "zai" => {
            steps.push("Create Z.AI key from portal.".to_string());
            steps.push("Set ZAI_API_KEY in local arda.env.".to_string());
        }
        "anthropic" => {
            steps.push("Create Anthropic credentials for Claude family models.".to_string());
            steps.push("Set ANTHROPIC_API_KEY in local arda.env.".to_string());
        }
        "cerebras" => {
            steps.push("Enable model route and key in Cerebras workspace.".to_string());
            steps.push("Set CEREBRAS_API_KEY in local arda.env.".to_string());
        }
        _ => {
            if let Some(key) = env_key {
                steps.push(format!("Set {key} in local arda.env."));
            }
            if !matches!(normalized.as_str(), "litellm_gateway") {
                steps
                    .push("Review provider docs for endpoint and billing assumptions.".to_string());
            }
        }
    }

    let signup_url = match normalized.as_str() {
        "opencode" => Some("https://opencode.ai".to_string()),
        "openrouter" => Some("https://openrouter.ai/keys".to_string()),
        "openai" | "openai_sub" => Some("https://platform.openai.com/api-keys".to_string()),
        "google" => Some("https://aistudio.google.com/app/apikey".to_string()),
        "mistral" => Some("https://console.mistral.ai/api-keys".to_string()),
        "groq" => Some("https://console.groq.com/keys".to_string()),
        "zai" => Some("https://platform.z.ai/dashboard".to_string()),
        "anthropic" => Some("https://console.anthropic.com/settings/keys".to_string()),
        "cerebras" => Some("https://inference.cerebras.ai/".to_string()),
        _ => None,
    };

    ProviderSignupHint {
        title: format!("{provider_id} onboarding"),
        signup_url,
        steps,
    }
}

pub(crate) fn make_local_model_default(
    value: String,
    source: ValueSource,
) -> LocalModelDefaultValue {
    LocalModelDefaultValue { value, source }
}

pub(crate) fn make_path_value(path: PathBuf, source: ValueSource, home: &Path) -> PathValue {
    let exists = path.exists().then_some(path.exists());
    let portable_expression = path.to_str().and_then(|raw| {
        raw.strip_prefix(home.to_string_lossy().as_ref())
            .map(|rest| {
                if rest.is_empty() {
                    "${HOME}".to_string()
                } else {
                    format!("${{HOME}}{rest}")
                }
            })
    });
    PathValue {
        value: path.to_string_lossy().to_string(),
        source,
        exists,
        portable_expression,
    }
}

pub(crate) fn make_url_value(value: String, source: ValueSource) -> UrlValue {
    let health = check_url_health(&value);
    UrlValue {
        value,
        source,
        health: Some(health),
    }
}

pub(crate) fn check_url_health(url: &str) -> String {
    let (host, default_port) = if let Some(rest) = url.strip_prefix("https://") {
        (rest, 443u16)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (rest, 80u16)
    } else {
        (url, 80u16)
    };
    let address = host;
    let host_port = address.split('/').next().unwrap_or(address);
    let address = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:{default_port}")
    };
    let mut addrs = match address.to_socket_addrs() {
        Ok(v) => v,
        Err(_) => return "unreachable".to_string(),
    };
    let candidate = match addrs.next() {
        Some(c) => c,
        None => return "unreachable".to_string(),
    };
    match TcpStream::connect_timeout(&candidate, Duration::from_millis(250)) {
        Ok(_) => "healthy".to_string(),
        Err(_) => "unreachable".to_string(),
    }
}

pub(crate) fn command_output(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|out| out.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub(crate) fn command_version(cmd: &str, args: &[&str]) -> Option<String> {
    command_output(cmd, args)
        .and_then(|value| value.lines().next().map(str::to_string))
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn make_prerequisite_check(
    check_id: &str,
    title: &str,
    status: &str,
    severity: &str,
    detected: String,
    recommendation: &str,
    command_hint: Option<&str>,
) -> PrerequisiteCheck {
    PrerequisiteCheck {
        check_id: check_id.to_string(),
        title: title.to_string(),
        status: status.to_string(),
        severity: severity.to_string(),
        detected,
        recommendation: recommendation.to_string(),
        command_hint: command_hint.map(str::to_string),
    }
}

pub(crate) fn secret_safe_preview(
    key: &str,
    value: Option<String>,
    secret: bool,
    fallback: &str,
) -> (String, bool, ValueSource) {
    match value.filter(|v| !v.trim().is_empty()) {
        Some(value) if secret => (
            format!("<secret-present:{} chars>", value.chars().count()),
            true,
            ValueSource::Environment,
        ),
        Some(value) => (value, true, ValueSource::Environment),
        None if secret => ("<missing-secret>".to_string(), false, ValueSource::Default),
        None => (
            fallback.to_string(),
            !fallback.trim().is_empty(),
            ValueSource::Default,
        ),
    }
    .pipe(|(preview, present, source): (String, bool, crate::onboarding::types::ValueSource)| {
        if secret && preview.contains(key) {
            ("<secret-present>".to_string(), present, source)
        } else {
            (preview, present, source)
        }
    })
}

pub(crate) trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

pub(crate) fn config_entry(
    key: &str,
    fallback: String,
    required: bool,
    secret: bool,
    recommendation: &str,
) -> PrivateConfigEntry {
    let (value_preview, present, source) =
        secret_safe_preview(key, env::var(key).ok(), secret, &fallback);
    PrivateConfigEntry {
        key: key.to_string(),
        value_preview,
        source,
        required,
        secret,
        present,
        recommendation: recommendation.to_string(),
    }
}

pub(crate) fn get_host_name() -> String {
    env::var("HOSTNAME")
        .or_else(|_| env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}
