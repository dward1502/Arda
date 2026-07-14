// sigil: REPAIR
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
        let out = Command::new("systemctl")
            .args([
                "--user",
                "list-units",
                "--all",
                "--no-legend",
                "--no-pager",
                "--plain",
                pattern,
            ])
            .output()?;
        if !out.status.success() {
            return Err(SystemdError::Exit {
                code: out.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
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
}
