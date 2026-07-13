// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRosterSnapshot {
    pub total_agents: usize,
    pub online_agents: usize,
    pub silent_agents: usize,
    pub agents: Vec<AgentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub last_heartbeat: Option<String>,
}

impl AgentRosterSnapshot {
    pub fn from_world_file(path: impl AsRef<Path>, heartbeat_timeout_secs: u64) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let world: WorldState = serde_json::from_str(&content).ok()?;

        let mut online_agents = 0usize;
        let mut silent_agents = 0usize;
        let now = Utc::now();
        let agents = world
            .agents
            .into_iter()
            .map(|a| {
                let heartbeat_fresh =
                    a.last_heartbeat
                        .as_deref()
                        .and_then(parse_utc)
                        .is_some_and(|ts| {
                            now.signed_duration_since(ts).num_seconds()
                                <= heartbeat_timeout_secs as i64
                        });

                let normalized_status =
                    if a.status.eq_ignore_ascii_case("online") && heartbeat_fresh {
                        "ONLINE".to_string()
                    } else if a.status.eq_ignore_ascii_case("online") {
                        "SILENT".to_string()
                    } else {
                        a.status.to_ascii_uppercase()
                    };

                if normalized_status == "ONLINE" {
                    online_agents += 1;
                } else {
                    silent_agents += 1;
                }

                AgentStatus {
                    id: a.id,
                    name: a.name,
                    status: normalized_status,
                    last_heartbeat: a.last_heartbeat,
                }
            })
            .collect::<Vec<_>>();

        Some(Self {
            total_agents: agents.len(),
            online_agents,
            silent_agents,
            agents,
        })
    }

    pub fn from_supervisor_state_file(path: impl AsRef<Path>) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let state: SupervisorState = serde_json::from_str(&content).ok()?;
        if state.agents.is_empty() {
            return None;
        }

        let mut online_agents = 0usize;
        let mut silent_agents = 0usize;
        let agents = state
            .agents
            .into_iter()
            .map(|agent| {
                let normalized_status = if agent.running && agent.healthy {
                    online_agents += 1;
                    "ONLINE"
                } else {
                    silent_agents += 1;
                    "SILENT"
                }
                .to_string();

                AgentStatus {
                    name: display_agent_name(&agent.agent),
                    id: agent.agent,
                    status: normalized_status,
                    last_heartbeat: Some(state.ts_utc.clone()),
                }
            })
            .collect::<Vec<_>>();

        Some(Self {
            total_agents: agents.len(),
            online_agents,
            silent_agents,
            agents,
        })
    }
}

fn parse_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|ts| ts.with_timezone(&Utc))
}

#[derive(Debug, Deserialize)]
struct WorldState {
    agents: Vec<WorldAgent>,
}

#[derive(Debug, Deserialize)]
struct WorldAgent {
    id: String,
    name: String,
    status: String,
    last_heartbeat: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SupervisorState {
    ts_utc: String,
    agents: Vec<SupervisorAgent>,
}

#[derive(Debug, Deserialize)]
struct SupervisorAgent {
    agent: String,
    running: bool,
    healthy: bool,
}

fn display_agent_name(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::AgentRosterSnapshot;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn parses_world_roster() {
        let dir = tempdir().expect("tempdir");
        let world_path = dir.path().join("world.json");
        let fresh = Utc::now().to_rfc3339();
        let stale = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        std::fs::write(
            &world_path,
            format!(
                r#"{{
              "agents": [
                {{"id":"a1","name":"Athena","status":"ONLINE","last_heartbeat":"{fresh}"}},
                {{"id":"a2","name":"Warden","status":"ONLINE","last_heartbeat":"{stale}"}}
              ]
            }}"#
            ),
        )
        .expect("write");

        let roster = AgentRosterSnapshot::from_world_file(&world_path, 300).expect("roster");
        assert_eq!(roster.total_agents, 2);
        assert_eq!(roster.online_agents, 1);
        assert_eq!(roster.silent_agents, 1);
        assert_eq!(roster.agents[0].status, "ONLINE");
        assert_eq!(roster.agents[1].status, "SILENT");
    }

    #[test]
    fn parses_supervisor_state_as_runtime_roster() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("state.json");
        std::fs::write(
            &state_path,
            r#"{
              "ts_utc":"2026-06-06T08:01:22Z",
              "agents": [
                {"agent":"prometheus","pid":"1","running":true,"healthy":true,"socket":"/tmp/p.sock","start_error":""},
                {"agent":"hermes","pid":"2","running":true,"healthy":true,"socket":"/tmp/h.sock","start_error":""},
                {"agent":"athena","pid":"3","running":false,"healthy":false,"socket":"/tmp/a.sock","start_error":"deferred"}
              ]
            }"#,
        )
        .expect("write");

        let roster =
            AgentRosterSnapshot::from_supervisor_state_file(&state_path).expect("supervisor");
        assert_eq!(roster.total_agents, 3);
        assert_eq!(roster.online_agents, 2);
        assert_eq!(roster.silent_agents, 1);
        assert_eq!(roster.agents[0].id, "prometheus");
        assert_eq!(roster.agents[0].status, "ONLINE");
        assert_eq!(
            roster.agents[0].last_heartbeat.as_deref(),
            Some("2026-06-06T08:01:22Z")
        );
    }
}
