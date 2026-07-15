// sigil: REPAIR
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscordBridgeReadinessState {
    Disabled,
    NotConfigured,
    ListenerDown,
    OnlineNoDeliveryProof,
    SendOnlyHealthy,
    ReceiveOnlyHealthy,
    BidirectionalHealthy,
    PolicyGatedHealthy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordBridgeEvidence {
    pub bridge_enabled: bool,
    pub configured: bool,
    pub listener_running: bool,
    pub provider_online: bool,
    pub recent_outbound_success: bool,
    pub recent_inbound_observed: bool,
    pub policy_guard_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordBridgeReadiness {
    pub state: DiscordBridgeReadinessState,
    pub operational: bool,
    pub reason: String,
}

impl DiscordBridgeReadiness {
    pub fn classify(evidence: &DiscordBridgeEvidence) -> Self {
        if !evidence.bridge_enabled {
            return Self::new(
                DiscordBridgeReadinessState::Disabled,
                false,
                "Discord bridge disabled by configuration",
            );
        }
        if !evidence.configured {
            return Self::new(
                DiscordBridgeReadinessState::NotConfigured,
                false,
                "Discord bridge not configured with required credentials/channel",
            );
        }
        if !evidence.listener_running || !evidence.provider_online {
            return Self::new(
                DiscordBridgeReadinessState::ListenerDown,
                false,
                "Discord listener or provider health check is down",
            );
        }

        match (
            evidence.recent_outbound_success,
            evidence.recent_inbound_observed,
            evidence.policy_guard_active,
        ) {
            (false, false, _) => Self::new(
                DiscordBridgeReadinessState::OnlineNoDeliveryProof,
                false,
                "Discord provider online but has no recent delivery proof",
            ),
            (true, false, _) => Self::new(
                DiscordBridgeReadinessState::SendOnlyHealthy,
                true,
                "Discord bridge has recent outbound delivery proof only",
            ),
            (false, true, _) => Self::new(
                DiscordBridgeReadinessState::ReceiveOnlyHealthy,
                true,
                "Discord bridge has recent inbound observation proof only",
            ),
            (true, true, false) => Self::new(
                DiscordBridgeReadinessState::BidirectionalHealthy,
                true,
                "Discord bridge has bidirectional proof but no active policy guard evidence",
            ),
            (true, true, true) => Self::new(
                DiscordBridgeReadinessState::PolicyGatedHealthy,
                true,
                "Discord bridge has bidirectional proof and active policy guard evidence",
            ),
        }
    }

    fn new(state: DiscordBridgeReadinessState, operational: bool, reason: &str) -> Self {
        Self {
            state,
            operational,
            reason: reason.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiscordBridgeEvidence, DiscordBridgeReadiness, DiscordBridgeReadinessState};

    #[test]
    fn discord_bridge_readiness_reports_disabled_before_configuration() {
        let evidence = DiscordBridgeEvidence {
            bridge_enabled: false,
            configured: false,
            listener_running: false,
            provider_online: false,
            recent_outbound_success: false,
            recent_inbound_observed: false,
            policy_guard_active: false,
        };

        let readiness = DiscordBridgeReadiness::classify(&evidence);

        assert_eq!(readiness.state, DiscordBridgeReadinessState::Disabled);
        assert!(!readiness.operational);
        assert!(readiness.reason.contains("disabled"));
    }

    #[test]
    fn discord_bridge_readiness_does_not_overclaim_online_without_delivery_proof() {
        let evidence = DiscordBridgeEvidence {
            bridge_enabled: true,
            configured: true,
            listener_running: true,
            provider_online: true,
            recent_outbound_success: false,
            recent_inbound_observed: false,
            policy_guard_active: true,
        };

        let readiness = DiscordBridgeReadiness::classify(&evidence);

        assert_eq!(
            readiness.state,
            DiscordBridgeReadinessState::OnlineNoDeliveryProof
        );
        assert!(!readiness.operational);
        assert!(readiness.reason.contains("no recent delivery proof"));
    }

    #[test]
    fn discord_bridge_readiness_requires_policy_guard_for_policy_gated_healthy() {
        let evidence = DiscordBridgeEvidence {
            bridge_enabled: true,
            configured: true,
            listener_running: true,
            provider_online: true,
            recent_outbound_success: true,
            recent_inbound_observed: true,
            policy_guard_active: false,
        };

        let readiness = DiscordBridgeReadiness::classify(&evidence);

        assert_eq!(
            readiness.state,
            DiscordBridgeReadinessState::BidirectionalHealthy
        );
        assert!(readiness.operational);
    }

    #[test]
    fn discord_bridge_readiness_reports_policy_gated_when_bidirectional_and_guarded() {
        let evidence = DiscordBridgeEvidence {
            bridge_enabled: true,
            configured: true,
            listener_running: true,
            provider_online: true,
            recent_outbound_success: true,
            recent_inbound_observed: true,
            policy_guard_active: true,
        };

        let readiness = DiscordBridgeReadiness::classify(&evidence);

        assert_eq!(
            readiness.state,
            DiscordBridgeReadinessState::PolicyGatedHealthy
        );
        assert!(readiness.operational);
    }
}
