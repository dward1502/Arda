use arda_core::capability_composition::{CapabilityComposition, CompositionScope};
use arda_core::payment_capability::{
    OfflineReplayGuard, OfflineX402Case, PaymentCapabilityError, PaymentLineage,
    PaymentNetworkMode, PAYMENT_CAPABILITY_ID,
};
use arda_core::run_graph::{ObjectiveId, Provenance, RunGraph, RunId};
use chrono::{TimeZone, Utc};
use std::path::PathBuf;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap()
}

fn fixture() -> OfflineX402Case {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/payment-capability/v1/fixtures/offline-x402.json");
    OfflineX402Case::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn payment_composition() -> CapabilityComposition {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/capability-composition/v1/fixtures/valid-software-project.json");
    let mut composition =
        CapabilityComposition::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    composition.scope = CompositionScope::Business;
    composition
        .capabilities
        .required
        .insert(PAYMENT_CAPABILITY_ID.into());
    composition
        .capabilities
        .forbidden
        .remove(PAYMENT_CAPABILITY_ID);
    composition
}

fn payment_run(composition: &CapabilityComposition) -> RunGraph {
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new(composition.lineage.run_id.clone()).unwrap(),
        objective_id: ObjectiveId::new(composition.lineage.objective_id.clone()).unwrap(),
        nodes: vec![],
        edges: vec![],
        provenance: Provenance {
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            created_by: "payment-capability-test".into(),
            parent_receipts: vec![],
        },
    }
}

fn verify(
    case: &OfflineX402Case,
    at: chrono::DateTime<Utc>,
    replay: &mut OfflineReplayGuard,
) -> Result<arda_core::payment_capability::PaymentFixtureReceipt, PaymentCapabilityError> {
    let mut composition = payment_composition();
    composition.lineage.project_id = case.contract.lineage.project_id;
    composition.lineage.project_contract_digest =
        case.contract.lineage.project_contract_digest.clone();
    composition.lineage.objective_id = case.contract.lineage.objective_id.as_str().into();
    composition.lineage.run_id = case.contract.lineage.run_id.as_str().into();
    let run = payment_run(&composition);
    case.verify(at, replay, &composition, &run)
}

#[test]
fn payment_capability_is_business_scoped_and_explicitly_selected() {
    let mut composition = payment_composition();
    let run = payment_run(&composition);
    assert!(PaymentLineage::from_composition(&composition, &run).is_ok());

    composition
        .capabilities
        .required
        .remove(PAYMENT_CAPABILITY_ID);
    assert_eq!(
        PaymentLineage::from_composition(&composition, &run),
        Err(PaymentCapabilityError::PaymentCapabilityNotSelected)
    );
    assert_eq!(
        fixture().verify(
            now(),
            &mut OfflineReplayGuard::default(),
            &composition,
            &run,
        ),
        Err(PaymentCapabilityError::PaymentCapabilityNotSelected)
    );
    composition.scope = CompositionScope::Personal;
    composition
        .capabilities
        .required
        .insert(PAYMENT_CAPABILITY_ID.into());
    assert_eq!(
        PaymentLineage::from_composition(&composition, &run),
        Err(PaymentCapabilityError::PaymentCapabilityNotSelected)
    );

    let composition = payment_composition();
    let mut mismatched = payment_run(&composition);
    mismatched.run_id = RunId::new("run:unrelated").unwrap();
    assert_eq!(
        PaymentLineage::from_composition(&composition, &mismatched),
        Err(PaymentCapabilityError::LineageMismatch)
    );
}

#[test]
fn offline_x402_fixture_verifies_without_authorizing_any_live_rail() {
    let case = fixture();
    let mut replay = OfflineReplayGuard::default();
    let receipt = verify(&case, now(), &mut replay).unwrap();
    assert!(receipt.payment_fixture_verified);
    assert!(!receipt.authorizes_testnet);
    assert!(!receipt.authorizes_live_funds);
    assert_eq!(receipt.mode, PaymentNetworkMode::OfflineFixture);
    assert_eq!(receipt.quote_id, case.contract.quote.quote_id);
    assert_eq!(receipt.run_id, case.contract.lineage.run_id);
    assert_eq!(receipt.confirmations, 1);

    let encoded = serde_json::to_string(&(case, receipt))
        .unwrap()
        .to_lowercase();
    for forbidden in ["private_key", "seed_phrase", "mnemonic", "wallet_secret"] {
        assert!(!encoded.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn offline_x402_rejects_replay_expiry_and_quote_substitution() {
    let case = fixture();
    let mut replay = OfflineReplayGuard::default();
    verify(&case, now(), &mut replay).unwrap();
    let mut restarted = OfflineReplayGuard::from_json_str(&replay.to_json().unwrap()).unwrap();
    assert_eq!(
        verify(&case, now(), &mut restarted),
        Err(PaymentCapabilityError::ReplayDetected)
    );
    assert_eq!(
        verify(&case, now(), &mut replay),
        Err(PaymentCapabilityError::ReplayDetected)
    );

    for mutation in ["amount", "currency", "recipient", "network"] {
        let mut changed = fixture();
        match mutation {
            "amount" => changed.exchange.challenge.amount = "9.99".into(),
            "currency" => changed.exchange.challenge.currency_asset = "OTHER".into(),
            "recipient" => changed.exchange.challenge.payee_reference = "payee:attacker".into(),
            "network" => changed.exchange.challenge.network = "wrongnet".into(),
            _ => unreachable!(),
        }
        assert_eq!(
            verify(&changed, now(), &mut OfflineReplayGuard::default()),
            Err(PaymentCapabilityError::QuoteBindingMismatch),
            "mutation {mutation}"
        );
    }

    let expired = fixture();
    assert_eq!(
        verify(
            &expired,
            Utc.with_ymd_and_hms(2026, 8, 12, 12, 0, 0).unwrap(),
            &mut OfflineReplayGuard::default(),
        ),
        Err(PaymentCapabilityError::Expired)
    );
}

#[test]
fn budgets_and_required_accounting_compensation_fields_fail_closed() {
    let mut case = fixture();
    case.contract.budget.per_action_limit = "9.99".into();
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::BudgetExceeded)
    );

    let mut case = fixture();
    case.contract.budget.cumulative_spent_before = "20.00".into();
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::BudgetExceeded)
    );

    for missing in ["compensation", "accounting", "acceptance"] {
        let mut case = fixture();
        match missing {
            "compensation" => case.contract.compensation.process_reference.clear(),
            "accounting" => case.contract.accounting.export_code.clear(),
            "acceptance" => case.contract.acceptance.artifact_receipt_ids.clear(),
            _ => unreachable!(),
        }
        assert_eq!(
            verify(&case, now(), &mut OfflineReplayGuard::default()),
            Err(PaymentCapabilityError::InvalidContract),
            "missing {missing}"
        );
    }
}

#[test]
fn fixture_schema_rejects_private_key_ingress() {
    let raw = serde_json::to_string(&fixture()).unwrap();
    let injected = raw.replacen(
        "\"redacted_custody_reference\"",
        "\"private_key\":\"forbidden\",\"redacted_custody_reference\"",
        1,
    );
    assert!(matches!(
        OfflineX402Case::from_json_str(&injected),
        Err(PaymentCapabilityError::InvalidJson(_))
    ));
}

#[test]
fn approval_is_bound_to_exact_quote_terms_amount_and_run() {
    for mutation in ["quote", "terms", "amount", "payer", "run"] {
        let mut case = fixture();
        match mutation {
            "quote" => case.contract.approval.quote_id = "quote:other".into(),
            "terms" => case.contract.approval.terms_digest = format!("sha256:{}", "0".repeat(64)),
            "amount" => case.contract.approval.amount = "11.00".into(),
            "payer" => case.contract.approval.payer_reference = "payer:other".into(),
            "run" => case.contract.approval.run_id = RunId::new("run:other").unwrap(),
            _ => unreachable!(),
        }
        assert_eq!(
            verify(&case, now(), &mut OfflineReplayGuard::default()),
            Err(PaymentCapabilityError::ApprovalBindingMismatch),
            "mutation {mutation}"
        );
    }
}

#[test]
fn security_policy_fails_closed_for_live_modes_revoke_and_secret_like_custody() {
    let mut case = fixture();
    case.contract.security.requested_mode = PaymentNetworkMode::Testnet;
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::LiveRailDenied)
    );

    for mutation in ["live", "promotion"] {
        let mut case = fixture();
        match mutation {
            "live" => case.contract.security.live_funds_authorized = true,
            "promotion" => case.contract.security.automatic_environment_promotion = true,
            _ => unreachable!(),
        }
        assert_eq!(
            verify(&case, now(), &mut OfflineReplayGuard::default()),
            Err(PaymentCapabilityError::LiveRailDenied),
            "mutation {mutation}"
        );
    }

    for mutation in ["provider", "visibility"] {
        let mut case = fixture();
        match mutation {
            "provider" => case.contract.security.fail_closed_on_provider_error = false,
            "visibility" => case.contract.security.operator_visibility_required = false,
            _ => unreachable!(),
        }
        assert_eq!(
            verify(&case, now(), &mut OfflineReplayGuard::default()),
            Err(PaymentCapabilityError::InvalidContract),
            "mutation {mutation}"
        );
    }

    let mut case = fixture();
    case.contract.security.emergency_revoked = true;
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::EmergencyRevoked)
    );

    let mut case = fixture();
    case.contract.redacted_custody_reference = "private_key:super-secret".into();
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::UnsafeCustodyReference)
    );
}

#[test]
fn fixture_signature_and_finality_receipt_fail_closed() {
    let mut case = fixture();
    case.exchange.payment_response.signature = format!("sha256:{}", "0".repeat(64));
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::InvalidFixtureSignature)
    );

    let mut case = fixture();
    case.exchange.payment_response.confirmations = 0;
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::InsufficientFinality)
    );

    let mut case = fixture();
    case.exchange.payment_response.provider_status = "failed".into();
    assert_eq!(
        verify(&case, now(), &mut OfflineReplayGuard::default()),
        Err(PaymentCapabilityError::ProviderFailure)
    );
}
