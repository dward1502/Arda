#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! CEO autopilot binary — runs the autonomous loop or a single cycle.

use arda_prometheus::autopilot::{ceo_loop, AutopilotConfig, CeoAutopilot};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

fn parse_args() -> (PathBuf, bool, bool, Option<u64>) {
    let mut root = std::env::var("ARDA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mut once = false;
    let mut read_only = std::env::var("ARDA_CEO_READ_ONLY").ok().as_deref() == Some("1");
    let mut interval: Option<u64> = std::env::var("ARDA_CEO_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok());
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--once" => once = true,
            "--read-only" => read_only = true,
            "--root" => {
                if let Some(v) = args.next() {
                    root = PathBuf::from(v);
                }
            }
            "--interval" => {
                if let Some(v) = args.next() {
                    interval = v.parse().ok();
                }
            }
            "--help" | "-h" => {
                println!("ceo-autopilot [--once] [--read-only] [--root PATH] [--interval SECS]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    (root, once, read_only, interval)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (root, once, read_only, interval) = parse_args();
    let mut cfg = AutopilotConfig::from_root(&root);
    cfg.read_only = read_only;
    if let Some(s) = interval {
        cfg.interval = Duration::from_secs(s);
    }
    if let Ok(raw) = std::env::var("ARDA_CEO_JOULE_CYCLE_LIMIT") {
        if let Ok(limit) = raw.parse::<f64>() {
            cfg.joule_cycle_limit = limit;
        }
    }
    if let Ok(raw) = std::env::var("ARDA_CEO_JOULE_HOURLY_LIMIT") {
        if let Ok(limit) = raw.parse::<f64>() {
            cfg.joule_hourly_limit = limit;
        }
    }

    let mut auto = CeoAutopilot::from_world(cfg);

    if once {
        let report = auto.run_cycle().await;
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        return Ok(());
    }

    let stop = Arc::new(AtomicBool::new(false));
    let s2 = stop.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        s2.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    eprintln!(
        "ceo-autopilot starting (root={}, interval={:?}, read_only={}, joule_budget={:.0})",
        auto.config().root.display(),
        auto.config().interval,
        auto.config().read_only,
        auto.config().joule_budget
    );
    ceo_loop(auto, stop).await;
    Ok(())
}

#[cfg(not(feature = "full-cli"))]
fn main() {}
