//! Thin typed client for `systemctl --user`.
//!
//! Wraps the most common operational query — `list-units --all` — and exposes
//! a `SystemdClient` trait so consumers can mock for tests.

use serde::Serialize;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemdError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("systemctl exited {code}: {stderr}")]
    Exit { code: i32, stderr: String },
}

/// Outcome of `systemctl list-units` for a single unit.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Unit {
    pub name: String,
    pub load: String,   // loaded | not-found | masked | error
    pub active: String, // active | inactive | activating | deactivating | failed
    pub sub: String,    // running | exited | dead | waiting | elapsed | listening | …
    pub kind: UnitKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum UnitKind {
    Service,
    Timer,
    Socket,
    Other,
}

impl UnitKind {
    pub fn classify(unit: &str) -> Self {
        if unit.ends_with(".timer") {
            Self::Timer
        } else if unit.ends_with(".socket") {
            Self::Socket
        } else if unit.ends_with(".service") {
            Self::Service
        } else {
            Self::Other
        }
    }
}

pub trait SystemdClient {
    /// Return the raw `list-units` table for units matching `pattern`.
    fn list_units_raw(&self, pattern: &str) -> Result<String, SystemdError>;

    /// Parsed view of `list-units` filtered by `pattern`.
    fn list_units(&self, pattern: &str) -> Result<Vec<Unit>, SystemdError> {
        Ok(parse_list_units(&self.list_units_raw(pattern)?))
    }
}

/// Default implementation that shells out to `systemctl --user`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemctlClient;

impl SystemdClient for SystemctlClient {
    fn list_units_raw(&self, pattern: &str) -> Result<String, SystemdError> {
        let user = std::env::var("USER").ok();
        list_units_with_runner(pattern, user.as_deref(), |args| {
            let out = Command::new("systemctl").args(args).output()?;
            Ok((
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stdout).into_owned(),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        })
    }
}

fn list_units_args(pattern: &str, machine: Option<&str>) -> Vec<String> {
    let mut args = vec!["--user".to_string()];
    if let Some(machine) = machine {
        args.push(format!("--machine={machine}"));
    }
    args.extend([
        "list-units".to_string(),
        "--all".to_string(),
        "--no-legend".to_string(),
        "--no-pager".to_string(),
        "--plain".to_string(),
        pattern.to_string(),
    ]);
    args
}

fn list_units_with_runner<F>(
    pattern: &str,
    user: Option<&str>,
    mut runner: F,
) -> Result<String, SystemdError>
where
    F: FnMut(&[String]) -> Result<(i32, String, String), std::io::Error>,
{
    let (code, stdout, stderr) = runner(&list_units_args(pattern, None))?;
    if code == 0 {
        return Ok(stdout);
    }

    if let Some(user) = user.filter(|user| !user.is_empty()) {
        let machine = format!("{user}@.host");
        let (fallback_code, fallback_stdout, fallback_stderr) =
            runner(&list_units_args(pattern, Some(&machine)))?;
        if fallback_code == 0 {
            return Ok(fallback_stdout);
        }
        return Err(SystemdError::Exit {
            code: fallback_code,
            stderr: format!(
                "local user bus failed: {}; host-machine fallback failed: {}",
                stderr.trim(),
                fallback_stderr.trim()
            ),
        });
    }

    Err(SystemdError::Exit { code, stderr })
}

pub fn parse_list_units(raw: &str) -> Vec<Unit> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let name = cols[0].to_string();
        let kind = UnitKind::classify(&name);
        out.push(Unit {
            name,
            load: cols[1].into(),
            active: cols[2].into(),
            sub: cols[3].into(),
            kind,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str);
    impl SystemdClient for Fake {
        fn list_units_raw(&self, _: &str) -> Result<String, SystemdError> {
            Ok(self.0.to_string())
        }
    }

    #[test]
    fn parses_table() {
        let raw = "arda-foo.service loaded active running Foo\narda-foo.timer  loaded active waiting Foo timer\n";
        let units = parse_list_units(raw);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].kind, UnitKind::Service);
        assert_eq!(units[1].kind, UnitKind::Timer);
        assert_eq!(units[1].sub, "waiting");
    }

    #[test]
    fn trait_default_method_uses_raw() {
        let c = Fake("arda-x.service loaded failed failed X\n");
        let units = c.list_units("arda-*").unwrap();
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].active, "failed");
    }

    #[test]
    fn classifies_unknown_unit_as_other() {
        assert_eq!(UnitKind::classify("foo.mount"), UnitKind::Other);
    }

    #[test]
    fn retries_through_host_machine_when_local_user_bus_fails() {
        let mut calls = Vec::new();
        let raw = list_units_with_runner("arda-*", Some("mythos"), |args| {
            calls.push(args.to_vec());
            if calls.len() == 1 {
                Ok((1, String::new(), "local bus unavailable".to_string()))
            } else {
                Ok((
                    0,
                    "arda-manwe.service loaded active running Manwe\n".to_string(),
                    String::new(),
                ))
            }
        })
        .expect("host-machine fallback");

        assert!(raw.contains("arda-manwe.service"));
        assert!(!calls[0].iter().any(|arg| arg.starts_with("--machine=")));
        assert!(calls[1].iter().any(|arg| arg == "--machine=mythos@.host"));
    }
}
