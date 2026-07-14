use super::*;

// ============================================================
// 🔥 CLI DISPATCH ENTRY
// ============================================================
pub(crate) async fn execute(cli: Cli, config: &Config) -> anyhow::Result<()> {
    match cli.command {
        Commands::Run {
            task_type,
            description,
        } => handle_run(config, &task_type, &description).await?,
        Commands::Tools => handle_tools(config)?,
        Commands::Status => handle_status(config, &cli.config),
        Commands::Export { command } => {
            let value = export_surface::run(command)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::Control { command } => control::handle(command)?,
        Commands::Council { command } => operating::handle_council(command)?,
        Commands::Venture { command } => operating::handle_venture(command)?,
        Commands::Utility { command } => utility::handle(command)?,
        Commands::Pipeline { command } => pipeline::handle(command).await?,
        Commands::Aipkg { command } => aipkg::handle(command)?,
        Commands::Athena { command } => athena::handle(command, config).await?,
        Commands::Prometheus { command } => prometheus::handle(command).await?,
        Commands::Charon { command } => charon::handle(command).await?,
        Commands::Mnemosyne { command } => mnemosyne::handle(command).await?,
        Commands::Hades { command } => hades::handle(command).await?, // 🔥 FIXED
        Commands::Hermes { command } => hermes::handle(command).await?,
        Commands::Chronos { command } => chronos::handle(command)?,
        Commands::Apollo { command } => apollo::handle(command).await?,
        Commands::Plutus { command } => plutus::handle(command).await?,
        Commands::Oracle { command } => oracle::handle(command).await?,
        Commands::Metrics { command } => metrics::handle(command).await?,
        Commands::State { command } => state::handle(command)?,
        Commands::Onboarding { command } => onboarding::handle(command)?,
        Commands::Halt { command } => loop_cmd::handle_halt(command)?,
        Commands::Warden { command } => loop_cmd::handle_warden(command)?,
        Commands::Learning { command } => learning::handle(command)?,
        Commands::Loop { command } => loop_cmd::handle(command)?,
        Commands::Forge { command } => forge::handle(command).await?,
        Commands::Iterate {
            target_image,
            asset_id,
            domain,
            prompt,
            negative,
            assets_root,
            scene_binding,
            material_family,
            budget_iters,
            accept_threshold,
            comfyui_addr,
            timeout_secs,
            vision_addr,
            vision_model,
            vision_timeout_secs,
            bpy_python,
            angles,
            render_size,
        } => {
            use annunimas_forge_mind::forge::{iterate_asset, IterateSpec};
            use annunimas_forge_mind::tools::comfyui::ComfyUiClient;
            use annunimas_forge_mind::tools::vision::VisionClient;

            let assets_root = assets_root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(annunimas_forge_mind::forge::DEFAULT_ASSETS_ROOT));

            let render_config = annunimas_forge_mind::forge::RenderConfig {
                python: bpy_python
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(annunimas_forge_mind::forge::DEFAULT_PYTHON)),
                angles: angles
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_else(|| {
                        annunimas_forge_mind::forge::DEFAULT_ANGLES
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    }),
                width: render_size,
                height: render_size,
            };

            let spec = IterateSpec {
                target_image: PathBuf::from(&target_image),
                asset_id: asset_id.clone(),
                domain,
                initial_prompt: prompt,
                negative_prompt: negative.unwrap_or_else(|| {
                    annunimas_forge_mind::forge::DEFAULT_NEGATIVE_PROMPT.to_string()
                }),
                assets_root,
                scene_binding: scene_binding.unwrap_or_else(|| asset_id.clone()),
                material_family,
                overrides: annunimas_forge_mind::forge::GenerateOverrides::default(),
                budget_iters,
                accept_threshold,
                render_config,
            };

            let mut comfy = match comfyui_addr {
                Some(addr) => ComfyUiClient::new(addr),
                None => ComfyUiClient::from_env(),
            };
            if let Some(t) = timeout_secs {
                comfy = comfy.with_timeout(std::time::Duration::from_secs(t));
            }

            let mut vision = VisionClient::new(
                vision_addr.unwrap_or_else(|| "http://annunimas-server:8081".to_string()),
                vision_model.unwrap_or_else(|| "qwen2.5-vl-7b-instruct".to_string()),
            );
            if let Some(t) = vision_timeout_secs {
                vision.timeout = std::time::Duration::from_secs(t);
            }

            let output = iterate_asset(comfy, vision, spec).await?;
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

// ============================================================
// 🔥 HANDLE RUN (ECHO ROOT CONTROL LAYER)
// ============================================================
async fn handle_run(config: &Config, task_type: &str, description: &str) -> anyhow::Result<()> {
    let mut config = config.clone();

    let mut decision = "PAUSE".to_string();
    let mut trace_id: String;
    let mut route_hint = "safe_only".to_string();

    let mut rho = None;
    let mut gamma = None;
    let mut delta = None;

    {
        use serde_json::json;
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let fallback_trace_id = format!(
            "trace-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let bridge_path = std::env::var("ECHO_GATE_PATH")
            .unwrap_or("/mnt/e/Echo_Root_VE/api/echo_gate_bridge.py".to_string());

        let gate_payload = json!({
            "task_type": task_type,
            "description": description,
            "context": {
                "stage": "pre-provider-build",
                "candidate_provider": config.llm.default_provider,
                "model_routes_present": !config.llm.model_routes.is_empty()
            }
        });

        let output = Command::new("python3")
            .arg(&bridge_path)
            .arg(gate_payload.to_string())
            .output();

        trace_id = fallback_trace_id.clone();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();

                if let Ok(gate) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    decision = gate["decision"].as_str().unwrap_or("PAUSE").to_string();

                    trace_id = gate["trace_id"]
                        .as_str()
                        .unwrap_or(&fallback_trace_id)
                        .to_string();

                    route_hint = gate["route_hint"]
                        .as_str()
                        .unwrap_or("safe_only")
                        .to_string();

                    rho = gate.get("rho").and_then(|v| v.as_f64());
                    gamma = gate.get("gamma").and_then(|v| v.as_f64());
                    delta = gate.get("delta").and_then(|v| v.as_f64());
                } else {
                    eprintln!("⚠️ Gate parse failed — forcing safe mode");
                }
            }
            Err(_) => {
                eprintln!("⚠️ Gate execution failed — forcing safe mode");
            }
        }

        eprintln!(
            "🧠 Echo FINAL Gate | decision={} | route_hint={} | trace_id={} | rho={:?} | gamma={:?} | delta={:?}",
            decision, route_hint, trace_id, rho, gamma, delta
        );

        match decision.as_str() {
            "ABORT" => {
                eprintln!("🛑 Echo Root blocked execution | trace_id={}", trace_id);
                std::process::exit(1);
            }
            "PAUSE" => {
                config.llm.default_provider = "opencode".to_string();
            }
            "PROCEED" => {}
            _ => {
                config.llm.default_provider = "opencode".to_string();
            }
        }

        // 🔥 Route-aware enforcement
        match route_hint.as_str() {
            "blocked" => {
                eprintln!("🛑 Route hint blocked execution | trace_id={}", trace_id);
                std::process::exit(1);
            }
            "safe_only" => {
                eprintln!(
                    "⚠️ Route hint enforcing safe provider | trace_id={}",
                    trace_id
                );
                config.llm.default_provider = "opencode".to_string();
            }
            "normal" => {}
            _ => {
                config.llm.default_provider = "opencode".to_string();
            }
        }
    }

    let provider = build_provider(&config);

    eprintln!(
        "⚡ Annunimas — provider: {} | model: {}",
        provider.provider_name(),
        provider.default_model()
    );

    let router = build_router(provider, config.llm.model_routes.clone())?;
    let ledger = Ledger::new(&config.paths.ledger_dir)?;
    let pipeline = Pipeline::with_core_link(router, ledger, config.joulework.threshold, "core");

    let result = pipeline.submit(Task::new(description, task_type)).await?;

    match &result.status {
        annunimas_core::TaskStatus::Complete => {
            if let Some(value) = &result.result {
                if let Some(response) = value.get("response").and_then(|v| v.as_str()) {
                    println!("{}", response);
                } else {
                    println!("{}", serde_json::to_string_pretty(value)?);
                }
            }
        }
        annunimas_core::TaskStatus::Pending => {
            println!("⏳ Task pending — execution pipeline active");
        }
        annunimas_core::TaskStatus::Failed { reason } => {
            eprintln!("Task failed: {}", reason);
            std::process::exit(1);
        }
        other => {
            eprintln!("Unexpected task status: {:?}", other);
        }
    }

    Ok(())
}

// ============================================================
// 🔧 TOOL HANDLERS
// ============================================================
fn handle_tools(config: &Config) -> anyhow::Result<()> {
    let provider = build_provider(config);
    let router = build_router(provider, config.llm.model_routes.clone())?;

    let output = format_tools_output(config, &router.list_agents());
    print!("{output}");
    Ok(())
}

fn handle_status(config: &Config, config_path: &str) {
    let provider = build_provider(config);
    let output = format_status_output(
        config,
        config_path,
        provider.provider_name(),
        provider.default_model(),
    );
    print!("{output}");
}
