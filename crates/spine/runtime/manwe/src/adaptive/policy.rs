// sigil: REPAIR
// Adaptive route policy: typed allow/block/health requirements.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::adaptive::candidate::CandidateCapabilities;
use crate::adaptive::error::{AdaptiveError, Result};
use crate::adaptive::types::{HealthState, RequestKind, RouteClass};

#[derive(Debug, Clone, Error)]
pub enum PolicyError {
    #[error("required tag is empty")]
    EmptyRequiredTag,
    #[error("allow/block entry is empty")]
    EmptyEntry,
    #[error("duplicate allow/block entry: {0}")]
    DuplicateEntry(String),
    #[error("allow and block overlap: {0}")]
    Overlap(String),
}

#[derive(Debug, Clone)]
pub struct RoutePolicy {
    required_capabilities: Vec<RequestKind>,
    allowlist: BTreeSet<String>,
    blocklist: BTreeSet<String>,
    minimum_health: HealthState,
    required_feature_tags: BTreeSet<String>,
    allowed_route_classes: BTreeSet<RouteClass>,
}

impl RoutePolicy {
    pub fn builder() -> RoutePolicyBuilder {
        RoutePolicyBuilder::default()
    }

    pub(crate) fn new(
        required_capabilities: Vec<RequestKind>,
        allowlist: BTreeSet<String>,
        blocklist: BTreeSet<String>,
        minimum_health: HealthState,
        required_feature_tags: BTreeSet<String>,
        allowed_route_classes: BTreeSet<RouteClass>,
    ) -> Self {
        Self {
            required_capabilities,
            allowlist,
            blocklist,
            minimum_health,
            required_feature_tags,
            allowed_route_classes,
        }
    }

    pub fn required_capabilities(&self) -> &[RequestKind] {
        &self.required_capabilities
    }

    pub fn allowlist(&self) -> &BTreeSet<String> {
        &self.allowlist
    }

    pub fn blocklist(&self) -> &BTreeSet<String> {
        &self.blocklist
    }

    pub fn minimum_health(&self) -> HealthState {
        self.minimum_health
    }

    pub fn required_feature_tags(&self) -> &BTreeSet<String> {
        &self.required_feature_tags
    }

    pub fn allowed_route_classes(&self) -> &BTreeSet<RouteClass> {
        &self.allowed_route_classes
    }

    pub fn permits_route_class(&self, route_class: RouteClass) -> bool {
        self.allowed_route_classes.contains(&route_class)
    }

    pub fn health_met(&self, candidate_health: HealthState) -> bool {
        health_rank(candidate_health) >= health_rank(self.minimum_health)
    }

    pub fn is_allowed_provider(&self, provider_id: &str) -> bool {
        self.allowlist.is_empty() || self.allowlist.contains(provider_id)
    }

    pub fn is_blocked_provider(&self, provider_id: &str) -> bool {
        self.blocklist.contains(provider_id)
    }

    pub fn blocked_by_tag(&self, candidate: &CandidateCapabilities) -> Result<()> {
        let has_tag = |tag: &str| match tag {
            "tools" => candidate.tools,
            "structured_output" => candidate.structured_output,
            "streaming" => candidate.streaming,
            "reasoning" => candidate.capable_tasks.iter().any(|t| t == "reasoning"),
            "code" => candidate.capable_tasks.iter().any(|t| t == "code"),
            "chat" => candidate.capable_tasks.iter().any(|t| t == "chat"),
            "summary" => candidate.capable_tasks.iter().any(|t| t == "summary"),
            "background" => candidate
                .capable_tasks
                .iter()
                .any(|t| t == "background"),
            "research" => candidate
                .capable_tasks
                .iter()
                .any(|t| t == "research"),
            "vision" => candidate
                .capable_tasks
                .iter()
                .any(|t| t == "research"),
            "thinking" => candidate
                .capable_tasks
                .iter()
                .any(|t| t == "reasoning"),
            _ => false,
        };

        for tag in self.required_feature_tags.iter() {
            if !has_tag(tag) {
                return Err(AdaptiveError::PolicyRejected(format!(
                    "missing required capability: {tag}"
                )));
            }
        }

        Ok(())
    }

    pub fn validate_candidate(&self, candidate: &crate::adaptive::candidate::RouteCandidate) -> Result<()> {
        if !self.is_allowed_provider(candidate.provider_id.as_str()) {
            return Err(AdaptiveError::PolicyRejected(format!(
                "provider not allowed by policy: {}",
                candidate.provider_id
            )));
        }
        if self.is_blocked_provider(candidate.provider_id.as_str()) {
            return Err(AdaptiveError::PolicyRejected(format!(
                "provider blocked by policy: {}",
                candidate.provider_id
            )));
        }
        if !self.permits_route_class(candidate.route_class) {
            return Err(AdaptiveError::PolicyRejected(format!(
                "route class blocked by policy: {:?}",
                candidate.route_class
            )));
        }
        if !self.health_met(candidate.health_state) {
            return Err(AdaptiveError::PolicyRejected(format!(
                "candidate health {} below policy minimum {}",
                candidate.health_state.as_str(),
                self.minimum_health.as_str(),
            )));
        }
        self.blocked_by_tag(&candidate.capabilities)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RoutePolicyBuilder {
    required_capabilities: Vec<RequestKind>,
    allowlist: BTreeSet<String>,
    blocklist: BTreeSet<String>,
    minimum_health: Option<HealthState>,
    required_feature_tags: BTreeSet<String>,
    allowed_route_classes: BTreeSet<RouteClass>,
}

impl RoutePolicyBuilder {
    pub fn with_capability(mut self, capability: RequestKind) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    pub fn with_required_tag(mut self, tag: impl Into<String>) -> Result<Self> {
        let tag = tag.into().trim().to_lowercase();
        if tag.is_empty() {
            return Err(PolicyError::EmptyRequiredTag.into());
        }
        self.required_feature_tags.insert(tag);
        Ok(self)
    }

    pub fn with_allow_provider(mut self, provider_id: impl Into<String>) -> Result<Self> {
        let provider_id = provider_id.into();
        let provider_id = provider_id.trim().to_string();
        if provider_id.is_empty() {
            return Err(PolicyError::EmptyEntry.into());
        }
        if self.blocklist.contains(&provider_id) {
            return Err(PolicyError::Overlap(provider_id).into());
        }
        self.allowlist.insert(provider_id.clone());
        Ok(self)
    }

    pub fn with_block_provider(mut self, provider_id: impl Into<String>) -> Result<Self> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(PolicyError::EmptyEntry.into());
        }
        let provider_id = provider_id.trim().to_string();
        if self.allowlist.contains(&provider_id) {
            return Err(PolicyError::Overlap(provider_id).into());
        }
        self.blocklist.insert(provider_id);
        Ok(self)
    }

    pub fn with_route_class(mut self, route_class: RouteClass) -> Self {
        self.allowed_route_classes.insert(route_class);
        self
    }

    pub fn with_minimum_health(mut self, minimum_health: HealthState) -> Self {
        self.minimum_health = Some(minimum_health);
        self
    }

    pub fn build(self) -> Result<RoutePolicy> {
        let minimum_health = self.minimum_health.unwrap_or(HealthState::Unknown);

        if self.required_capabilities.is_empty() {
            return Err(PolicyError::EmptyRequiredTag.into());
        }

        Ok(RoutePolicy::new(
            self.required_capabilities,
            self.allowlist,
            self.blocklist,
            minimum_health,
            self.required_feature_tags,
            self.allowed_route_classes,
        ))
    }
}

impl From<PolicyError> for AdaptiveError {
    fn from(value: PolicyError) -> Self {
        AdaptiveError::PolicyInvalid(value.to_string())
    }
}

fn health_rank(state: HealthState) -> u8 {
    match state {
        HealthState::Unknown => 0,
        HealthState::Probing => 1,
        HealthState::Healthy => 2,
        HealthState::Degraded => 3,
        HealthState::Down => 4,
    }
}


