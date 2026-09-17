//! Typed operations for configured workers; never carry executables or environment selectors.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod tests;
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeOperation {
    Probe {},
    Chat {
        query: String,
        max_turns: u32,
        toolsets: Vec<String>,
        /// Per-invocation workspace authority; omitted legacy requests fail closed.
        #[serde(default)]
        workspace_writable: bool,
    },
    Export {
        session_id: String,
    },
    VerifyArtifacts {
        paths: Vec<String>,
    },
}
impl RuntimeOperation {
    pub(crate) fn from_adapter_arguments(args: &[String]) -> Result<Self> {
        match args {
            [chat, quiet, source_flag, source, turns_flag, turns, rules, tools_flag, tools, query_flag, query]
                if chat == "chat"
                    && quiet == "-Q"
                    && source_flag == "--source"
                    && source == "tool"
                    && turns_flag == "--max-turns"
                    && rules == "--ignore-rules"
                    && query_flag == "-q"
                    && tools_flag == "-t" =>
            {
                Ok(Self::Chat {
                    query: query.clone(),
                    max_turns: turns.parse()?,
                    toolsets: tools.split(',').map(str::to_owned).collect(),
                    workspace_writable: false,
                })
            }
            [sessions, export, output, format_flag, format, session_flag, session, redact, yes]
                if sessions == "sessions"
                    && export == "export"
                    && session_flag == "--session-id"
                    && format_flag == "--format"
                    && format == "jsonl"
                    && output == "-"
                    && redact == "--redact"
                    && yes == "--yes" =>
            {
                Ok(Self::Export {
                    session_id: session.clone(),
                })
            }
            _ => bail!("unsupported retained Hermes operation"),
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Probe {} => (),
            Self::Chat {
                query,
                max_turns,
                toolsets,
                ..
            } => {
                if query.is_empty()
                    || query
                        .len()
                        .saturating_add("--query=".len())
                        .saturating_add(1)
                        > 65_536
                    || query.contains('\0')
                    || !(1..=1000).contains(max_turns)
                    || toolsets.is_empty()
                    || toolsets.len() > 2
                    || toolsets
                        .iter()
                        .any(|s| !matches!(s.as_str(), "terminal" | "file"))
                {
                    bail!("invalid bounded Hermes chat operation");
                }
                let mut sorted = toolsets.clone();
                sorted.sort();
                sorted.dedup();
                if sorted.len() != toolsets.len() {
                    bail!("duplicate toolset");
                }
            }
            Self::Export { session_id } => {
                if session_id.is_empty()
                    || session_id.len() > 256
                    || !session_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-:.".contains(&b))
                {
                    bail!("invalid session identity");
                }
            }
            Self::VerifyArtifacts { paths } => {
                if paths.is_empty() || paths.len() > 128 {
                    bail!("invalid artifact count");
                }
                // JSON escaping counts toward Linux's individual argv limit.
                // Leave room for the terminating NUL; never truncate paths.
                if serde_json::to_vec(paths)?.len() >= 65_536 {
                    bail!("serialized artifact paths exceed argument budget");
                }
                for path in paths {
                    if path.is_empty()
                        || path.len() > 4096
                        || path.contains('\0')
                        || path.starts_with('/')
                        || path
                            .split('/')
                            .any(|part| part.is_empty() || part == "." || part == "..")
                    {
                        bail!("invalid artifact path");
                    }
                }
            }
        }
        Ok(())
    }
    pub fn hermes_arguments(&self) -> Result<Vec<String>> {
        self.validate()?;
        Ok(match self {
            Self::Probe {} => vec!["--help".into()],
            Self::Chat {
                query,
                max_turns,
                toolsets,
                ..
            } => vec![
                "chat".into(),
                "-Q".into(),
                "--source=tool".into(),
                format!("--max-turns={max_turns}"),
                "--ignore-rules".into(),
                format!("--toolsets={}", toolsets.join(",")),
                format!("--query={query}"),
            ],
            Self::Export { session_id } => vec![
                "sessions".into(),
                "export".into(),
                "-".into(),
                "--format=jsonl".into(),
                format!("--session-id={session_id}"),
                "--redact".into(),
                "--yes".into(),
            ],
            Self::VerifyArtifacts { .. } => bail!("verifier is not a Hermes CLI operation"),
        })
    }
}
