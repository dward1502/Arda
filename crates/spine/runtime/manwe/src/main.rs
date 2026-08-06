//! `manwe` — the single governed OpenAI-compatible inference gateway.
//!
//! The binary always starts the governed adaptive runtime on the configured
//! bind address. `--adaptive` remains accepted as a no-op during the operator
//! cutover so existing launchers do not create a second runtime mode.

#[cfg(not(feature = "adaptive"))]
compile_error!("the manwe binary requires its default `adaptive` feature");

#[allow(dead_code)]
mod config;

use std::path::PathBuf;

use clap::Parser;
#[cfg(feature = "telemetry")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "telemetry")]
use tracing_subscriber::util::SubscriberInitExt;
#[cfg(feature = "telemetry")]
use tracing_subscriber::Layer;

use config::ManweConfig;

#[derive(Parser, Debug)]
#[command(
    name = "manwe",
    version,
    about = "Governed OpenAI-compatible inference gateway"
)]
struct Cli {
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    bind: Option<String>,
    #[arg(long, default_value = "manwe.toml")]
    config: PathBuf,
    /// Compatibility no-op: the governed runtime is now unconditional.
    #[arg(long, hide = true)]
    adaptive: bool,
    /// Retired compatibility flag. The canonical HTTP runtime rejects it.
    #[arg(long, hide = true)]
    grpc: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    #[cfg(feature = "telemetry")]
    let _telemetry_shutdown = {
        tracing_subscriber::registry()
            .with(arda_aule::telemetry::tracing_layer())
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .init();
        arda_aule::telemetry::shutdown_guard()
    };

    #[cfg(not(feature = "telemetry"))]
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();
    if cli.grpc {
        anyhow::bail!(
            "the separate Manwe gRPC runtime was retired; use the canonical HTTP/OpenAI surface"
        );
    }

    let (mut cfg, _) = ManweConfig::load_with_source(&cli.config);
    if let Some(port) = cli.port {
        cfg.port = port;
    }
    if let Some(bind) = cli.bind {
        cfg.bind = bind;
    }
    cfg.validate()
        .map_err(|error| anyhow::anyhow!("manwe config validation failed: {error}"))?;

    let arda_home = std::env::var_os("ARDA_HOME").map(PathBuf::from);
    let root = config::adaptive_state_dir(arda_home.as_deref());
    let service = manwe::adaptive::service::ManweService::new(root)?;
    if let Err(error) = service.reload_provider_config().await {
        tracing::warn!(
            error = %error,
            "provider config reload failed; using governed bootstrap catalog"
        );
    }

    let addr = format!("{}:{}", cfg.bind, cfg.port);
    manwe::adaptive::transport::http::run_http_server(service, &addr)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn adaptive_flag_is_a_compatible_no_op_not_a_runtime_selector() {
        let cli = Cli::try_parse_from(["manwe", "--adaptive"]).expect("legacy flag parses");
        assert!(cli.adaptive);
        assert!(!cli.grpc);
    }

    #[test]
    fn canonical_bind_overrides_are_typed() {
        let cli = Cli::try_parse_from([
            "manwe",
            "--bind",
            "0.0.0.0",
            "--port",
            "7171",
            "--config",
            "manwe.toml",
        ])
        .expect("canonical launch parses");
        assert_eq!(cli.bind.as_deref(), Some("0.0.0.0"));
        assert_eq!(cli.port, Some(7171));
    }
}
