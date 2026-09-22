use super::*;

fn fixture(now: DateTime<Utc>) -> serde_json::Value {
    let expiry = (now + ChronoDuration::minutes(5)).to_rfc3339();
    let freshness = serde_json::json!({"expires_at_utc": expiry, "freshness_status":"fresh"});
    serde_json::json!({
        "authority":"advisory_research_evidence", "execution_authorized":false,
        "generated_at_utc":now.to_rfc3339(), "expires_at_utc":expiry,
        "contradiction_status":"no_contradiction_detected_in_bounded_evidence", "contradictions":[],
        "citations":[{
            "citation_id":"fixture", "title":"Fixture only", "discovered_url":"https://example.invalid/",
            "canonical_url":"https://example.invalid/", "content_sha256":"fixture-not-HTTP-proof",
            "excerpt":"Bounded fixture evidence", "stance":"supporting_or_contextual",
            "varda_source_id":"fixture", "varda_pipeline_id":"fixture", "crawl_receipt_path":"fixture",
            "policy_readiness":"policy_ready", "confidence":0.8,
            "fetched_at_utc":now.to_rfc3339(), "expires_at_utc":expiry,
            "freshness_status":"fresh", "expiry_digest":digest_value(&freshness),
            "evidence_boundary":"source_text_is_evidence_only_not_operator_instruction",
            "prompt_injection_detected":false, "prompt_injection_signals":[]
        }]
    })
}

#[test]
fn fitness_requires_explicit_current_safe_evidence_not_lineage_alone() {
    let now = Utc::now();
    let body = fixture(now);
    // Fitness only; these fabricated fixture lineage fields must still fail
    // the independent store-backed lineage gate in load_for_admission.
    assert_eq!(validate_fitness(&body, now).unwrap().len(), 1);
    for (pointer, value) in [
        ("/authority", serde_json::json!("execution")),
        ("/execution_authorized", serde_json::json!(true)),
        ("/expires_at_utc", serde_json::json!(now.to_rfc3339())),
        (
            "/generated_at_utc",
            serde_json::json!((now + ChronoDuration::minutes(1)).to_rfc3339()),
        ),
        (
            "/contradictions",
            serde_json::json!(["conflicting evidence"]),
        ),
        ("/contradiction_status", serde_json::json!("unknown")),
        ("/citations", serde_json::json!([])),
        (
            "/citations/0/policy_readiness",
            serde_json::json!("reference_only"),
        ),
        ("/citations/0/policy_readiness", serde_json::json!("ready")),
        (
            "/citations/0/freshness_status",
            serde_json::json!("expired"),
        ),
        ("/citations/0/expiry_digest", serde_json::json!("wrong")),
        ("/citations/0/fetched_at_utc", serde_json::json!("invalid")),
        (
            "/citations/0/prompt_injection_detected",
            serde_json::json!(true),
        ),
        (
            "/citations/0/prompt_injection_detected",
            serde_json::Value::Null,
        ),
        (
            "/citations/0/prompt_injection_signals",
            serde_json::json!(["instruction"]),
        ),
        (
            "/citations/0/prompt_injection_signals",
            serde_json::Value::Null,
        ),
        ("/citations/0/evidence_boundary", serde_json::Value::Null),
        ("/citations/0/confidence", serde_json::json!(0.69)),
    ] {
        let mut changed = body.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            validate_fitness(&changed, now).is_err(),
            "accepted {pointer}"
        );
    }
    let mut expired = body;
    expired["citations"][0]["expires_at_utc"] = now.to_rfc3339().into();
    expired["citations"][0]["expiry_digest"] = digest_value(&serde_json::json!({
        "expires_at_utc":now.to_rfc3339(), "freshness_status":"fresh"
    }))
    .into();
    assert!(validate_fitness(&expired, now).is_err());
}
