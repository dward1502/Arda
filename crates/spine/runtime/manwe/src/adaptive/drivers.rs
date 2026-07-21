// sigil: REPAIR
// External driver hooks for the adaptive runtime.
//
// This module defines the callback/event shapes that connect adaptive
// routing to runtime probes, configuration reloads, route history, and
// gateway mode reporting. Implementations are provided at startup so the
// adaptive subtree stays transport-agnostic.

#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::adaptive::candidate::RouteCandidate;
use crate::adaptive::provider::ProviderCapabilitySummary;
use crate::adaptive::state::AdaptiveSnapshot;

#[derive(Debug, Clone, Default)]
pub struct RuntimeProbeReport {
    pub provider_states: BTreeMap<String, ProviderCapabilitySummary>,
    pub refreshed_at_utc: Option<u64>,
}

pub trait RuntimeProbeDriver: Send + Sync {
    fn probe_providers(&self) -> RuntimeProbeReport;
}

#[derive(Debug, Clone, Default)]
pub struct ConfigReloadReport {
    pub reloaded: bool,
    pub provider_count: usize,
    pub error: Option<String>,
}

pub trait ConfigReloadDriver: Send + Sync {
    fn reload_config(&self) -> ConfigReloadReport;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayMode {
    Static,
    Adaptive,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayModeReport {
    pub mode: GatewayMode,
    pub route_count: usize,
    pub adaptive_enabled: bool,
}

pub trait GatewayModeDriver: Send + Sync {
    fn current_mode(&self) -> GatewayModeReport;
}

#[derive(Debug, Clone, Default)]
pub struct RouteHistoryHookReport {
    pub recorded: bool,
    pub route_id: Option<String>,
}

pub trait RouteHistoryHookDriver: Send + Sync {
    fn record_route_history(
        &self,
        candidate: &RouteCandidate,
        outcome: crate::adaptive::session::SessionOutcome,
    ) -> RouteHistoryHookReport;
}
