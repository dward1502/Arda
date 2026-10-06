//! Validate the exact keeper evidence set; this grants no retirement authority.
use super::*;
use serde_json::Value;

/// The caller must separately read the bound keeper store under exclusion and
/// revalidate managed cessation. Receipt strings are never cessation evidence.
pub fn validate_keeper_receipt_set(
    authorization: &AbandonmentAuthorization,
    receipts: &[Value],
) -> Result<String> {
    use sha2::{Digest, Sha256};
    authorization.manifest.validate()?;
    if authorization.event_id.trim().is_empty()
        || authorization.operator_id.trim().is_empty()
        || !digest_valid(&authorization.payload_digest)
        || authorization.authorized_at_ms < 0
        || authorization.expires_at_ms <= authorization.authorized_at_ms
    {
        bail!("invalid keeper abandonment authorization binding");
    }
    if receipts.len() != authorization.manifest.targets.len() {
        bail!("keeper abandonment receipt set is incomplete or oversized");
    }
    // Match the keeper's typed authorization serialization, not a Value map.
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(authorization)?));
    let mut canonical = std::collections::BTreeMap::new();
    for receipt in receipts {
        let target: AbandonmentTarget = serde_json::from_value(receipt["target"].clone())?;
        if !authorization.manifest.targets.contains(&target)
            || canonical.contains_key(&target.run_id)
        {
            bail!("keeper abandonment receipt target is duplicate or outside authorization");
        }
        let record = &receipt["keeper_record"];
        if !record.is_object() || record["run"].as_str() != Some(&target.run_id) {
            bail!("keeper abandonment record run mismatch");
        }
        // Exact equality validates envelope fields and the typed target only.
        // The nested keeper_record is opaque except for its object/run check;
        // this helper does not validate its complete schema or historical hash.
        // The caller must still compare this full record to the durable keeper row
        // and its historical typed digest under owner/endpoint exclusion.
        let expected = serde_json::json!({
            "version":1,"disposition":"operator_abandonment",
            "application_id":digest,"authorization_event_id":authorization.event_id,
            "authorization_digest":digest,"operator_id":authorization.operator_id,
            "target":target,"keeper_record":record,
            "mutation_provenance_digest":authorization.manifest.mutation_provenance_digest,
            "original_paired_lineage_unrecoverable":true,
            "cessation_proof":"managed_cgroup_teardown_or_reboot",
            "worker_cleanup_ack":false,"cleanup_verified":false,
            "artifacts_retained":true,"engine_reservations_retired":false
        });
        if *receipt != expected {
            bail!("keeper abandonment receipt binding or preservation flags mismatch");
        }
        canonical.insert(target.run_id, receipt.clone());
    }
    record_digest(&serde_json::json!({"version":1,"keeper_receipts_by_run":canonical}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (AbandonmentAuthorization, Vec<Value>) {
        let (_, _, manifest) = super::super::tests::fixture();
        let auth = AbandonmentAuthorization {
            event_id: "event".into(),
            payload_digest: "c".repeat(64),
            operator_id: "operator".into(),
            authorized_at_ms: 10,
            expires_at_ms: 100,
            manifest,
        };
        let digest = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(serde_json::to_vec(&auth).unwrap()))
        };
        let receipts = auth
            .manifest
            .targets
            .iter()
            .map(|target| {
                serde_json::json!({
                    "version":1,"disposition":"operator_abandonment",
                    "application_id":digest,"authorization_event_id":auth.event_id,
                    "authorization_digest":digest,"operator_id":auth.operator_id,
                    "target":target,"keeper_record":{"run":target.run_id},
                    "mutation_provenance_digest":auth.manifest.mutation_provenance_digest,
                    "original_paired_lineage_unrecoverable":true,
                    "cessation_proof":"managed_cgroup_teardown_or_reboot",
                    "worker_cleanup_ack":false,"cleanup_verified":false,
                    "artifacts_retained":true,"engine_reservations_retired":false
                })
            })
            .collect();
        (auth, receipts)
    }

    #[test]
    fn exact_keeper_set_is_order_independent_and_never_a_release() {
        let (auth, mut receipts) = fixture();
        let digest = validate_keeper_receipt_set(&auth, &receipts).unwrap();
        assert!(digest_valid(&digest));
        receipts.reverse();
        assert_eq!(
            digest,
            validate_keeper_receipt_set(&auth, &receipts).unwrap()
        );
    }

    #[test]
    fn receipt_validation_is_not_time_or_cessation_authority() {
        let (auth, receipts) = fixture();
        // The historical authorization expired long ago; validation is structural
        // and remains usable for read-only replay, never permission to apply.
        assert!(validate_keeper_receipt_set(&auth, &receipts).is_ok());
        for mode in ["event", "operator", "payload", "negative", "interval"] {
            let mut bad = auth.clone();
            match mode {
                "event" => bad.event_id.clear(),
                "operator" => bad.operator_id.clear(),
                "payload" => bad.payload_digest.clear(),
                "negative" => bad.authorized_at_ms = -1,
                _ => bad.expires_at_ms = bad.authorized_at_ms,
            }
            assert!(
                validate_keeper_receipt_set(&bad, &receipts).is_err(),
                "{mode}"
            );
        }
    }

    #[test]
    fn keeper_set_rejects_missing_extra_duplicate_and_changed_evidence() {
        let (auth, receipts) = fixture();
        let mut bad = receipts.clone();
        bad.pop();
        assert!(validate_keeper_receipt_set(&auth, &bad).is_err());
        let mut bad = receipts.clone();
        bad.push(receipts[0].clone());
        assert!(validate_keeper_receipt_set(&auth, &bad).is_err());
        let mut bad = receipts.clone();
        bad[4] = bad[0].clone();
        assert!(validate_keeper_receipt_set(&auth, &bad).is_err());
        for (key, value) in [
            ("version", serde_json::json!(2)),
            ("disposition", serde_json::json!("terminal_revocation")),
            ("authorization_digest", serde_json::json!("d".repeat(64))),
            ("application_id", serde_json::json!("d".repeat(64))),
            ("authorization_event_id", serde_json::json!("other")),
            ("operator_id", serde_json::json!("other")),
            (
                "mutation_provenance_digest",
                serde_json::json!("d".repeat(64)),
            ),
            ("worker_cleanup_ack", serde_json::json!(true)),
            ("cleanup_verified", serde_json::json!(true)),
            ("engine_reservations_retired", serde_json::json!(true)),
            ("artifacts_retained", serde_json::json!(false)),
            (
                "original_paired_lineage_unrecoverable",
                serde_json::json!(false),
            ),
            ("cessation_proof", serde_json::json!("claimed")),
            ("unexpected", serde_json::json!(true)),
        ] {
            let mut bad = receipts.clone();
            bad[4][key] = value;
            assert!(
                validate_keeper_receipt_set(&auth, &bad).is_err(),
                "accepted {key}"
            );
        }
        let mut bad = receipts.clone();
        bad[4]["target"]["keeper_owner"] = "other".into();
        assert!(validate_keeper_receipt_set(&auth, &bad).is_err());
        let mut bad = receipts.clone();
        bad[4]["keeper_record"]["run"] = "other".into();
        assert!(validate_keeper_receipt_set(&auth, &bad).is_err());
    }
}
