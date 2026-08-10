//! Optional, approval-bound payment capability and offline x402 fixture verification.
//!
//! This module deliberately has no wallet, signer, provider client, testnet, or live-funds
//! execution path. `payment_fixture_verified` proves only the deterministic offline boundary.

use crate::capability_composition::{CapabilityComposition, CompositionScope};
use crate::run_graph::{ObjectiveId, RunGraph, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const PAYMENT_CAPABILITY_SCHEMA_VERSION: &str = "arda.payment-capability.v1";
pub const OFFLINE_X402_CASE_SCHEMA_VERSION: &str = "arda.offline-x402-case.v1";
pub const PAYMENT_FIXTURE_RECEIPT_SCHEMA_VERSION: &str = "arda.payment-fixture-receipt.v1";
pub const OFFLINE_REPLAY_GUARD_SCHEMA_VERSION: &str = "arda.offline-payment-replay-guard.v1";
pub const PAYMENT_CAPABILITY_ID: &str = "payment";
const FIXTURE_SIGNATURE_SCHEME: &str = "offline-fixture-sha256-v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PaymentCapabilityError {
    #[error("payment capability is not explicitly selected for a business composition")]
    PaymentCapabilityNotSelected,
    #[error("payment project, objective, and run lineage does not match")]
    LineageMismatch,
    #[error("payment contract is invalid")]
    InvalidContract,
    #[error("payment amount is invalid or exceeds its budget")]
    BudgetExceeded,
    #[error("payment approval does not match the exact quote, terms, amount, recipient, network, and run")]
    ApprovalBindingMismatch,
    #[error("payment quote or approval has expired")]
    Expired,
    #[error("payment replay or duplicate idempotency identity detected")]
    ReplayDetected,
    #[error("x402 challenge or response does not match the approved quote")]
    QuoteBindingMismatch,
    #[error("offline fixture signature is invalid")]
    InvalidFixtureSignature,
    #[error("provider failed or returned a non-final response")]
    ProviderFailure,
    #[error("settlement has insufficient finality confirmations")]
    InsufficientFinality,
    #[error("testnet and live payment rails are not authorized by this contract version")]
    LiveRailDenied,
    #[error("payment capability is emergency revoked")]
    EmergencyRevoked,
    #[error("custody reference is not safely redacted")]
    UnsafeCustodyReference,
    #[error("invalid payment fixture JSON: {0}")]
    InvalidJson(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLineage {
    pub project_id: Uuid,
    pub project_contract_digest: String,
    pub objective_id: ObjectiveId,
    pub run_id: RunId,
}

impl PaymentLineage {
    pub fn from_composition(
        composition: &CapabilityComposition,
        run: &RunGraph,
    ) -> Result<Self, PaymentCapabilityError> {
        if composition.validate().is_err() || run.validate().is_err() {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        if composition.scope != CompositionScope::Business
            || !composition
                .capabilities
                .required
                .contains(PAYMENT_CAPABILITY_ID)
        {
            return Err(PaymentCapabilityError::PaymentCapabilityNotSelected);
        }
        if !run.matches_composition_lineage(composition) {
            return Err(PaymentCapabilityError::LineageMismatch);
        }
        Ok(Self {
            project_id: composition.lineage.project_id,
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            objective_id: ObjectiveId::new(composition.lineage.objective_id.clone())
                .map_err(|_| PaymentCapabilityError::InvalidContract)?,
            run_id: RunId::new(composition.lineage.run_id.clone())
                .map_err(|_| PaymentCapabilityError::InvalidContract)?,
        })
    }

    fn validate(&self) -> Result<(), PaymentCapabilityError> {
        if !is_sha256_digest(&self.project_contract_digest)
            || self.objective_id.as_str().trim().is_empty()
            || self.run_id.as_str().trim().is_empty()
        {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentRailIdentity {
    pub rail: String,
    pub provider: String,
    pub network: String,
    pub asset: String,
    pub asset_decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentQuote {
    pub quote_id: String,
    pub amount: String,
    pub currency_asset: String,
    pub expires_at: DateTime<Utc>,
    pub terms_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentBudget {
    pub per_action_limit: String,
    pub cumulative_limit: String,
    pub cumulative_spent_before: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentAcceptanceCondition {
    pub condition: String,
    pub condition_digest: String,
    pub artifact_receipt_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentReplayProtection {
    pub idempotency_key: String,
    pub challenge_nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentApprovalBinding {
    pub approval_receipt_id: Uuid,
    pub quote_id: String,
    pub run_id: RunId,
    pub amount: String,
    pub currency_asset: String,
    pub payer_reference: String,
    pub payee_reference: String,
    pub network: String,
    pub terms_digest: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentSettlementState {
    NotAttempted,
    Submitted,
    Confirmed,
    Failed,
    Refunded,
    Disputed,
    Compensated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentSettlement {
    pub state: PaymentSettlementState,
    pub receipt_id: Option<String>,
    pub confirmations: u64,
    pub provider_reference: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentCompensationState {
    None,
    RefundAvailable,
    RefundRequested,
    Refunded,
    Disputed,
    Compensated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentCompensation {
    pub state: PaymentCompensationState,
    pub process_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentAccountingClassification {
    pub classification: String,
    pub tax_jurisdiction: String,
    pub export_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentNetworkMode {
    OfflineFixture,
    Testnet,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReorgPolicy {
    RevalidateAndSuspend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentSecurityPolicy {
    pub requested_mode: PaymentNetworkMode,
    pub emergency_revoked: bool,
    pub fail_closed_on_provider_error: bool,
    pub operator_visibility_required: bool,
    pub automatic_environment_promotion: bool,
    pub live_funds_authorized: bool,
    pub required_confirmations: u64,
    pub reorg_policy: ReorgPolicy,
    pub max_quote_ttl_seconds: u64,
}

impl PaymentSecurityPolicy {
    fn validate(&self) -> Result<(), PaymentCapabilityError> {
        if self.requested_mode != PaymentNetworkMode::OfflineFixture
            || self.live_funds_authorized
            || self.automatic_environment_promotion
        {
            return Err(PaymentCapabilityError::LiveRailDenied);
        }
        if self.emergency_revoked {
            return Err(PaymentCapabilityError::EmergencyRevoked);
        }
        if !self.fail_closed_on_provider_error
            || !self.operator_visibility_required
            || self.required_confirmations == 0
            || self.max_quote_ttl_seconds == 0
        {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentCapabilityContract {
    pub schema_version: String,
    pub capability_id: String,
    pub lineage: PaymentLineage,
    pub rail: PaymentRailIdentity,
    pub payer_reference: String,
    pub payee_reference: String,
    pub quote: PaymentQuote,
    pub budget: PaymentBudget,
    pub acceptance: PaymentAcceptanceCondition,
    pub replay: PaymentReplayProtection,
    pub approval: PaymentApprovalBinding,
    pub settlement: PaymentSettlement,
    pub compensation: PaymentCompensation,
    pub accounting: PaymentAccountingClassification,
    pub redacted_custody_reference: String,
    pub security: PaymentSecurityPolicy,
}

impl PaymentCapabilityContract {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), PaymentCapabilityError> {
        if self.schema_version != PAYMENT_CAPABILITY_SCHEMA_VERSION
            || self.capability_id != PAYMENT_CAPABILITY_ID
        {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        self.lineage.validate()?;
        self.security.validate()?;
        for value in [
            &self.rail.rail,
            &self.rail.provider,
            &self.rail.network,
            &self.rail.asset,
            &self.payer_reference,
            &self.payee_reference,
            &self.quote.quote_id,
            &self.quote.currency_asset,
            &self.acceptance.condition,
            &self.replay.idempotency_key,
            &self.replay.challenge_nonce,
            &self.compensation.process_reference,
            &self.accounting.classification,
            &self.accounting.tax_jurisdiction,
            &self.accounting.export_code,
        ] {
            require_text(value)?;
        }
        if self.rail.asset_decimals > 18
            || self.quote.currency_asset != self.rail.asset
            || !is_sha256_digest(&self.quote.terms_digest)
            || !is_sha256_digest(&self.acceptance.condition_digest)
            || self.acceptance.artifact_receipt_ids.is_empty()
            || self
                .acceptance
                .artifact_receipt_ids
                .iter()
                .any(|receipt| receipt.trim().is_empty())
        {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        validate_custody_reference(&self.redacted_custody_reference)?;

        let amount = parse_amount(&self.quote.amount, self.rail.asset_decimals)?;
        let per_action = parse_amount(&self.budget.per_action_limit, self.rail.asset_decimals)?;
        let cumulative = parse_amount(&self.budget.cumulative_limit, self.rail.asset_decimals)?;
        let spent = parse_amount(
            &self.budget.cumulative_spent_before,
            self.rail.asset_decimals,
        )?;
        if amount == 0
            || amount > per_action
            || spent
                .checked_add(amount)
                .is_none_or(|total| total > cumulative)
        {
            return Err(PaymentCapabilityError::BudgetExceeded);
        }
        if now >= self.quote.expires_at || now >= self.approval.expires_at {
            return Err(PaymentCapabilityError::Expired);
        }
        let quote_ttl = self
            .quote
            .expires_at
            .signed_duration_since(self.approval.approved_at)
            .num_seconds();
        if quote_ttl <= 0 || quote_ttl as u64 > self.security.max_quote_ttl_seconds {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        if self.approval.quote_id != self.quote.quote_id
            || self.approval.run_id != self.lineage.run_id
            || self.approval.amount != self.quote.amount
            || self.approval.currency_asset != self.quote.currency_asset
            || self.approval.payer_reference != self.payer_reference
            || self.approval.payee_reference != self.payee_reference
            || self.approval.network != self.rail.network
            || self.approval.terms_digest != self.quote.terms_digest
            || self.approval.approved_at >= self.approval.expires_at
        {
            return Err(PaymentCapabilityError::ApprovalBindingMismatch);
        }
        match self.settlement.state {
            PaymentSettlementState::NotAttempted => {
                if self.settlement.receipt_id.is_some()
                    || self.settlement.provider_reference.is_some()
                    || self.settlement.confirmations != 0
                {
                    return Err(PaymentCapabilityError::InvalidContract);
                }
            }
            PaymentSettlementState::Confirmed => {
                if self
                    .settlement
                    .receipt_id
                    .as_deref()
                    .is_none_or(str::is_empty)
                    || self.settlement.confirmations < self.security.required_confirmations
                {
                    return Err(PaymentCapabilityError::InsufficientFinality);
                }
            }
            _ => {
                if self
                    .settlement
                    .provider_reference
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    return Err(PaymentCapabilityError::InvalidContract);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineX402Quote {
    pub quote_id: String,
    pub amount: String,
    pub currency_asset: String,
    pub payee_reference: String,
    pub network: String,
    pub expires_at: DateTime<Utc>,
    pub terms_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineX402Challenge {
    pub challenge_id: String,
    pub quote_id: String,
    pub amount: String,
    pub currency_asset: String,
    pub payee_reference: String,
    pub network: String,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineX402PaymentResponse {
    pub payment_id: String,
    pub quote_id: String,
    pub challenge_id: String,
    pub provider_status: String,
    pub settlement_receipt_id: String,
    pub confirmations: u64,
    pub signature_scheme: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineX402Exchange {
    pub quote: OfflineX402Quote,
    pub challenge: OfflineX402Challenge,
    pub payment_response: OfflineX402PaymentResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineX402Case {
    pub schema_version: String,
    pub contract: PaymentCapabilityContract,
    pub exchange: OfflineX402Exchange,
}

impl OfflineX402Case {
    pub fn from_json_str(raw: &str) -> Result<Self, PaymentCapabilityError> {
        serde_json::from_str(raw)
            .map_err(|error| PaymentCapabilityError::InvalidJson(error.to_string()))
    }

    pub fn verify(
        &self,
        now: DateTime<Utc>,
        replay_guard: &mut OfflineReplayGuard,
        composition: &CapabilityComposition,
        run: &RunGraph,
    ) -> Result<PaymentFixtureReceipt, PaymentCapabilityError> {
        if self.schema_version != OFFLINE_X402_CASE_SCHEMA_VERSION {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        let active_lineage = PaymentLineage::from_composition(composition, run)?;
        if active_lineage != self.contract.lineage {
            return Err(PaymentCapabilityError::LineageMismatch);
        }
        self.contract.validate(now)?;
        let quote = &self.exchange.quote;
        if quote.quote_id != self.contract.quote.quote_id
            || quote.amount != self.contract.quote.amount
            || quote.currency_asset != self.contract.quote.currency_asset
            || quote.payee_reference != self.contract.payee_reference
            || quote.network != self.contract.rail.network
            || quote.expires_at != self.contract.quote.expires_at
            || quote.terms_digest != self.contract.quote.terms_digest
        {
            return Err(PaymentCapabilityError::QuoteBindingMismatch);
        }
        let challenge = &self.exchange.challenge;
        if challenge.quote_id != quote.quote_id
            || challenge.amount != quote.amount
            || challenge.currency_asset != quote.currency_asset
            || challenge.payee_reference != quote.payee_reference
            || challenge.network != quote.network
            || challenge.nonce != self.contract.replay.challenge_nonce
            || challenge.expires_at != quote.expires_at
            || now >= challenge.expires_at
        {
            return Err(PaymentCapabilityError::QuoteBindingMismatch);
        }
        let response = &self.exchange.payment_response;
        if response.quote_id != quote.quote_id
            || response.challenge_id != challenge.challenge_id
            || response.payment_id != self.contract.replay.idempotency_key
            || response.settlement_receipt_id.trim().is_empty()
        {
            return Err(PaymentCapabilityError::QuoteBindingMismatch);
        }
        if response.provider_status != "confirmed" {
            return Err(PaymentCapabilityError::ProviderFailure);
        }
        if response.confirmations < self.contract.security.required_confirmations {
            return Err(PaymentCapabilityError::InsufficientFinality);
        }
        if response.signature_scheme != FIXTURE_SIGNATURE_SCHEME
            || response.signature != self.expected_fixture_signature()
        {
            return Err(PaymentCapabilityError::InvalidFixtureSignature);
        }
        replay_guard.record(
            &self.contract.replay.idempotency_key,
            &self.contract.replay.challenge_nonce,
        )?;
        let verification_digest = self.expected_fixture_signature();
        Ok(PaymentFixtureReceipt {
            schema_version: PAYMENT_FIXTURE_RECEIPT_SCHEMA_VERSION.into(),
            receipt_id: format!("fixture-receipt:{}", &verification_digest[7..23]),
            payment_fixture_verified: true,
            authorizes_testnet: false,
            authorizes_live_funds: false,
            mode: PaymentNetworkMode::OfflineFixture,
            project_id: self.contract.lineage.project_id,
            run_id: self.contract.lineage.run_id.clone(),
            quote_id: quote.quote_id.clone(),
            payment_id: response.payment_id.clone(),
            settlement_receipt_id: response.settlement_receipt_id.clone(),
            confirmations: response.confirmations,
            verification_digest,
            verified_at: now,
        })
    }

    pub fn expected_fixture_signature(&self) -> String {
        let quote = &self.exchange.quote;
        let challenge = &self.exchange.challenge;
        let response = &self.exchange.payment_response;
        let material = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            quote.quote_id,
            challenge.challenge_id,
            response.payment_id,
            quote.amount,
            quote.currency_asset,
            self.contract.payer_reference,
            quote.payee_reference,
            quote.network,
            quote.expires_at.to_rfc3339(),
            quote.terms_digest,
            challenge.nonce,
            response.provider_status,
            response.settlement_receipt_id,
            response.confirmations,
            self.contract.approval.approval_receipt_id,
            self.contract.lineage.run_id.as_str(),
        );
        format!("sha256:{:x}", Sha256::digest(material.as_bytes()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineReplayGuard {
    schema_version: String,
    idempotency_keys: BTreeSet<String>,
    challenge_nonces: BTreeSet<String>,
}

impl Default for OfflineReplayGuard {
    fn default() -> Self {
        Self {
            schema_version: OFFLINE_REPLAY_GUARD_SCHEMA_VERSION.into(),
            idempotency_keys: BTreeSet::new(),
            challenge_nonces: BTreeSet::new(),
        }
    }
}

impl OfflineReplayGuard {
    pub fn from_json_str(raw: &str) -> Result<Self, PaymentCapabilityError> {
        let guard: Self = serde_json::from_str(raw)
            .map_err(|error| PaymentCapabilityError::InvalidJson(error.to_string()))?;
        if guard.schema_version != OFFLINE_REPLAY_GUARD_SCHEMA_VERSION {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        Ok(guard)
    }

    pub fn to_json(&self) -> Result<String, PaymentCapabilityError> {
        serde_json::to_string(self)
            .map_err(|error| PaymentCapabilityError::InvalidJson(error.to_string()))
    }

    fn record(
        &mut self,
        idempotency_key: &str,
        challenge_nonce: &str,
    ) -> Result<(), PaymentCapabilityError> {
        if self.schema_version != OFFLINE_REPLAY_GUARD_SCHEMA_VERSION {
            return Err(PaymentCapabilityError::InvalidContract);
        }
        if self.idempotency_keys.contains(idempotency_key)
            || self.challenge_nonces.contains(challenge_nonce)
        {
            return Err(PaymentCapabilityError::ReplayDetected);
        }
        self.idempotency_keys.insert(idempotency_key.to_string());
        self.challenge_nonces.insert(challenge_nonce.to_string());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentFixtureReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub payment_fixture_verified: bool,
    pub authorizes_testnet: bool,
    pub authorizes_live_funds: bool,
    pub mode: PaymentNetworkMode,
    pub project_id: Uuid,
    pub run_id: RunId,
    pub quote_id: String,
    pub payment_id: String,
    pub settlement_receipt_id: String,
    pub confirmations: u64,
    pub verification_digest: String,
    pub verified_at: DateTime<Utc>,
}

fn parse_amount(value: &str, decimals: u8) -> Result<u128, PaymentCapabilityError> {
    let (whole, fraction) = value
        .split_once('.')
        .ok_or(PaymentCapabilityError::InvalidContract)?;
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (whole.len() > 1 && whole.starts_with('0'))
        || fraction.len() != usize::from(decimals)
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(PaymentCapabilityError::InvalidContract);
    }
    let scale = 10_u128
        .checked_pow(u32::from(decimals))
        .ok_or(PaymentCapabilityError::InvalidContract)?;
    let whole = whole
        .parse::<u128>()
        .map_err(|_| PaymentCapabilityError::InvalidContract)?;
    let fraction = fraction
        .parse::<u128>()
        .map_err(|_| PaymentCapabilityError::InvalidContract)?;
    whole
        .checked_mul(scale)
        .and_then(|scaled| scaled.checked_add(fraction))
        .ok_or(PaymentCapabilityError::InvalidContract)
}

fn validate_custody_reference(value: &str) -> Result<(), PaymentCapabilityError> {
    let lowered = value.to_ascii_lowercase();
    if !lowered.starts_with("redacted:")
        || [
            "private_key",
            "private-key",
            "seed_phrase",
            "seed-phrase",
            "mnemonic",
            "wallet_secret",
            "wallet-secret",
        ]
        .iter()
        .any(|forbidden| lowered.contains(forbidden))
    {
        return Err(PaymentCapabilityError::UnsafeCustodyReference);
    }
    Ok(())
}

fn require_text(value: &str) -> Result<(), PaymentCapabilityError> {
    if value.trim().is_empty() {
        Err(PaymentCapabilityError::InvalidContract)
    } else {
        Ok(())
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
