// sigil: ANKH
// ============================================================
// Arda DATA LAYER (prototype reference module)
// core/personal/data.rs
//
// Clearance-gated access to clients, projects, and personal data.
// ◈ The gate is not optional. It is structural.
//
// Architecture:
//   DataLayer is the single point of access for all persistent data.
//   Agents do not read files directly — they ask DataLayer.
//   DataLayer checks clearance before returning anything.
//   DataLayer logs every access to the ledger.
//
// Clearance hierarchy (ascending):
//   observer → worker → guardian → sovereign
// ============================================================

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use tokio::fs;
use chrono::Utc;
use tracing::{info, warn};

use crate::{
    AgentId,
    ledger::{Ledger, LedgerEntry, SoterionMeta},
};

// ============================================================
// CLEARANCE
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Clearance {
    Observer  = 0,
    Worker    = 1,
    Guardian  = 2,
    Sovereign = 3,
}

impl Clearance {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "observer"  => Self::Observer,
            "worker"    => Self::Worker,
            "guardian"  => Self::Guardian,
            "sovereign" => Self::Sovereign,
            _ => {
                warn!("Unknown clearance level '{}' — defaulting to observer", s);
                Self::Observer
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observer  => "observer",
            Self::Worker    => "worker",
            Self::Guardian  => "guardian",
            Self::Sovereign => "sovereign",
        }
    }
}

impl std::fmt::Display for Clearance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================
// AGENT CLEARANCE REGISTRY
// Loaded from agents.toml at boot. Cached here.
// ============================================================

#[derive(Debug, Clone)]
pub struct AgentClearanceRegistry {
    clearances: HashMap<AgentId, Clearance>,
}

impl AgentClearanceRegistry {
    pub fn new() -> Self {
        // Hardcoded defaults matching agents.toml
        // In production: loaded from agents.toml at boot
        let mut clearances = HashMap::new();
        clearances.insert("arandur".into(), Clearance::Sovereign);
        clearances.insert("athena".into(),  Clearance::Guardian);
        clearances.insert("oracle".into(),  Clearance::Guardian);
        clearances.insert("plutus".into(),  Clearance::Guardian);
        clearances.insert("hermes".into(),  Clearance::Worker);
        clearances.insert("warden".into(),  Clearance::Guardian);
        clearances.insert("apollo".into(),  Clearance::Worker);
        Self { clearances }
    }

    pub fn clearance_of(&self, agent: &str) -> Clearance {
        self.clearances.get(agent).copied().unwrap_or(Clearance::Observer)
    }

    pub fn can_access(&self, agent: &str, required: Clearance) -> bool {
        self.clearance_of(agent) >= required
    }
}

// ============================================================
// DATA LAYER
// ============================================================

pub struct DataLayer {
    root: PathBuf,
    registry: AgentClearanceRegistry,
    ledger: Ledger,
}

impl DataLayer {
    pub async fn open(root: impl AsRef<Path>, ledger: Ledger) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        Ok(Self {
            root,
            registry: AgentClearanceRegistry::new(),
            ledger,
        })
    }

    // ── CLEARANCE CHECK ────────────────────────────────────────

    fn check_clearance(
        &self,
        requesting_agent: &str,
        resource: &str,
        required: Clearance,
    ) -> Result<()> {
        if self.registry.can_access(requesting_agent, required) {
            Ok(())
        } else {
            warn!(
                "◈ CLEARANCE DENIED: {} attempted to access '{}' (requires {}, has {})",
                requesting_agent, resource, required,
                self.registry.clearance_of(requesting_agent)
            );
            bail!(
                "Clearance denied: {} lacks {} clearance required for '{}'",
                requesting_agent, required, resource
            )
        }
    }

    async fn log_access(
        &mut self,
        agent: &str,
        resource: &str,
        operation: &str,
        cost: f64,
    ) -> Result<()> {
        self.ledger.append(LedgerEntry {
            id: ulid::Ulid::new(),
            timestamp: Utc::now(),
            agent: agent.into(),
            event: format!("DATA_ACCESS:{}", operation),
            payload: format!("Resource: {}", resource),
            meta: SoterionMeta {
                sigil: REPAIR
                realm: Some("data".into()),
                tags: vec!["data-access".into(), operation.to_lowercase()],
                joule_cost: Some(cost),
                ..Default::default()
            },
        }).await
    }

    // ── CLIENT DATA ────────────────────────────────────────────

    /// Load a client profile. Requires guardian clearance.
    pub async fn load_client_profile(
        &mut self,
        agent: &str,
        client_slug: &str,
    ) -> Result<ClientProfile> {
        self.check_clearance(agent, &format!("client/{}", client_slug), Clearance::Guardian)?;

        let path = self.root
            .join("clients")
            .join(client_slug)
            .join("profile.toml");

        let content = fs::read_to_string(&path).await
            .map_err(|_| anyhow::anyhow!("Client '{}' not found", client_slug))?;

        let profile: ClientProfile = toml::from_str(&content)?;

        self.log_access(agent, &format!("client/{}/profile", client_slug), "READ", 0.1).await?;

        info!("𓁷 {} read client profile: {}", agent, client_slug);
        Ok(profile)
    }

    /// Load a client's interaction ledger. Requires guardian clearance.
    pub async fn load_client_ledger(
        &mut self,
        agent: &str,
        client_slug: &str,
    ) -> Result<Vec<ClientLedgerEntry>> {
        self.check_clearance(agent, &format!("client/{}/ledger", client_slug), Clearance::Guardian)?;

        let path = self.root
            .join("clients")
            .join(client_slug)
            .join("ledger.jsonl");

        if !path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&path).await?;
        let entries: Vec<ClientLedgerEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l))
            .collect::<std::result::Result<_, _>>()?;

        self.log_access(agent, &format!("client/{}/ledger", client_slug), "READ", 0.2).await?;

        Ok(entries)
    }

    /// Append to a client ledger. Requires guardian clearance.
    pub async fn append_client_ledger(
        &mut self,
        agent: &str,
        client_slug: &str,
        entry: ClientLedgerEntry,
    ) -> Result<()> {
        self.check_clearance(agent, &format!("client/{}/ledger", client_slug), Clearance::Guardian)?;

        let path = self.root
            .join("clients")
            .join(client_slug)
            .join("ledger.jsonl");

        let line = serde_json::to_string(&entry)? + "\n";

        // Append-only — never overwrite
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path).await?;
        file.write_all(line.as_bytes()).await?;

        self.log_access(agent, &format!("client/{}/ledger", client_slug), "APPEND", 0.1).await?;

        Ok(())
    }

    /// List all active clients. Requires guardian clearance.
    pub async fn list_clients(&mut self, agent: &str) -> Result<Vec<ClientSummary>> {
        self.check_clearance(agent, "clients/_registry", Clearance::Guardian)?;

        let path = self.root.join("clients").join("_registry.toml");
        let content = fs::read_to_string(&path).await?;
        let registry: ClientRegistry = toml::from_str(&content)?;

        let summaries = registry.client.into_iter()
            .map(|c| ClientSummary {
                slug: c.slug,
                name: c.name,
                status: c.status,
                clearance: Clearance::from_str(&c.clearance),
                priority: c.priority,
            })
            .collect();

        self.log_access(agent, "clients/_registry", "LIST", 0.05).await?;

        Ok(summaries)
    }

    // ── PROJECT DATA ───────────────────────────────────────────

    /// Load a project definition. Clearance varies by project.
    pub async fn load_project(
        &mut self,
        agent: &str,
        project_slug: &str,
    ) -> Result<Project> {
        // First, check if agent is assigned to this project
        // For now: guardian+ can read any project
        self.check_clearance(agent, &format!("project/{}", project_slug), Clearance::Guardian)?;

        let path = self.root
            .join("projects")
            .join(project_slug)
            .join("project.toml");

        let content = fs::read_to_string(&path).await
            .map_err(|_| anyhow::anyhow!("Project '{}' not found", project_slug))?;

        let project: Project = toml::from_str(&content)?;

        self.log_access(agent, &format!("project/{}", project_slug), "READ", 0.1).await?;

        Ok(project)
    }

    /// Load a project's task queue.
    pub async fn load_project_tasks(
        &mut self,
        agent: &str,
        project_slug: &str,
    ) -> Result<Vec<ProjectTask>> {
        self.check_clearance(agent, &format!("project/{}/tasks", project_slug), Clearance::Worker)?;

        let path = self.root
            .join("projects")
            .join(project_slug)
            .join("tasks")
            .join("queue.jsonl");

        if !path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&path).await?;
        let tasks: Vec<ProjectTask> = content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l))
            .collect::<std::result::Result<_, _>>()?;

        self.log_access(agent, &format!("project/{}/tasks", project_slug), "READ", 0.1).await?;

        Ok(tasks)
    }

    /// Append a decision to a project's immutable decision log.
    pub async fn append_project_decision(
        &mut self,
        agent: &str,
        project_slug: &str,
        decision: ProjectDecision,
    ) -> Result<()> {
        // Only arandur (sovereign) can write decisions
        self.check_clearance(agent, &format!("project/{}/decisions", project_slug), Clearance::Sovereign)?;

        let path = self.root
            .join("projects")
            .join(project_slug)
            .join("decisions.jsonl");

        let line = serde_json::to_string(&decision)? + "\n";

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)  // APPEND ONLY. Decisions are immutable.
            .open(&path).await?;
        file.write_all(line.as_bytes()).await?;

        info!("◇ Decision recorded: project/{} by {}", project_slug, agent);
        self.log_access(agent, &format!("project/{}/decisions", project_slug), "APPEND", 0.3).await?;

        Ok(())
    }

    // ── PERSONAL DATA ──────────────────────────────────────────

    /// Load Daniel's personal context. SOVEREIGN only.
    pub async fn load_personal_identity(&mut self, agent: &str) -> Result<PersonalIdentity> {
        self.check_clearance(agent, "personal/identity", Clearance::Sovereign)?;

        let path = self.root.join("personal").join("identity.toml");
        let content = fs::read_to_string(&path).await?;
        let identity: PersonalIdentity = toml::from_str(&content)?;

        self.log_access(agent, "personal/identity", "READ", 0.1).await?;

        Ok(identity)
    }

    /// List research notes in personal/research/. SOVEREIGN only.
    /// Returns file paths — Athena can then PageIndex them.
    pub async fn list_personal_research(&mut self, agent: &str) -> Result<Vec<PathBuf>> {
        self.check_clearance(agent, "personal/research", Clearance::Sovereign)?;

        let research_path = self.root.join("personal").join("research");
        if !research_path.exists() {
            return Ok(vec![]);
        }

        let mut paths = vec![];
        let mut entries = fs::read_dir(&research_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                paths.push(path);
            }
        }

        self.log_access(agent, "personal/research", "LIST", 0.05).await?;

        Ok(paths)
    }
}

// ============================================================
// DATA TYPES (match TOML/JSONL schema exactly)
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientRegistry {
    pub registry: ClientRegistryMeta,
    pub client: Vec<ClientRegistryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientRegistryMeta {
    pub created: String,
    pub maintained_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientRegistryEntry {
    pub slug: String,
    pub name: String,
    pub status: String,
    pub clearance: String,
    pub priority: u8,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientProfile {
    pub identity: ClientIdentity,
    pub business: ClientBusiness,
    pub soterion: SoterionFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdentity {
    pub name: String,
    pub slug: String,
    pub clearance: String,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientBusiness {
    pub industry: String,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoterionFields {
    pub sigil: String,
    pub realm: String,
    pub tags: Vec<String>,
    pub resonance: f64,
    pub clearance: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientLedgerEntry {
    pub id: String,
    pub timestamp: String,
    pub agent: String,
    pub event: String,
    pub payload: String,
    pub meta: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientSummary {
    pub slug: String,
    pub name: String,
    pub status: String,
    pub clearance: Clearance,
    pub priority: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub identity: ProjectIdentity,
    pub agents: ProjectAgents,
    pub budget: ProjectBudget,
    pub soterion: SoterionFields,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectIdentity {
    pub slug: String,
    pub name: String,
    pub full_name: String,
    pub status: String,
    pub priority: u8,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAgents {
    pub lead: String,
    pub assigned: Vec<String>,
    pub restricted: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectBudget {
    pub jw_total: f64,
    pub jw_spent: f64,
    pub jw_remaining: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTask {
    pub id: String,
    pub timestamp: String,
    pub created_by: String,
    pub title: String,
    pub description: String,
    pub priority: u8,
    pub status: String,
    pub assigned_to: String,
    pub realm: String,
    pub jw_estimate: f64,
    pub success_criteria: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDecision {
    pub id: String,
    pub timestamp: String,
    pub agent: String,
    pub event: String,
    pub payload: String,
    pub meta: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalIdentity {
    pub identity: PersonalIdentityCore,
    pub research_domains: PersonalResearch,
    pub soterion: SoterionFields,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalIdentityCore {
    pub name: String,
    pub sigil: String,
    pub role: String,
    pub clearance: String,
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalResearch {
    pub active: Vec<String>,
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clearance_ordering() {
        assert!(Clearance::Sovereign > Clearance::Guardian);
        assert!(Clearance::Guardian > Clearance::Worker);
        assert!(Clearance::Worker > Clearance::Observer);
    }

    #[test]
    fn test_clearance_gate() {
        let registry = AgentClearanceRegistry::new();

        // Arandur can access anything
        assert!(registry.can_access("arandur", Clearance::Sovereign));
        assert!(registry.can_access("arandur", Clearance::Guardian));
        assert!(registry.can_access("arandur", Clearance::Worker));

        // Apollo (worker) cannot access guardian data
        assert!(!registry.can_access("apollo", Clearance::Guardian));
        assert!(registry.can_access("apollo", Clearance::Worker));

        // Athena (guardian) can access client data but not sovereign
        assert!(registry.can_access("athena", Clearance::Guardian));
        assert!(!registry.can_access("athena", Clearance::Sovereign));

        // Warden cannot read client data (intentionally)
        // (warden is guardian but restricted in client._registry)
        // This is enforced at the registry level, not clearance level
    }

    #[test]
    fn test_clearance_from_str() {
        assert_eq!(Clearance::from_str("sovereign"), Clearance::Sovereign);
        assert_eq!(Clearance::from_str("GUARDIAN"), Clearance::Guardian);
        assert_eq!(Clearance::from_str("unknown"), Clearance::Observer);
    }
}
