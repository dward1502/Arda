//! Initial admission only. Replay must use the canonical saved admission first.
use super::*;
#[cfg(test)]
mod tests;

pub(in crate::harness) fn load_for_admission(
    root: &Path,
    owner: &str,
    brief_id: &str,
) -> Result<String, ApiError> {
    let (bytes, body) = load_verified_brief(root, owner, brief_id, None)?;
    if bytes.len() > 48 * 1024 {
        return Err(ApiError::bad_request(
            "Brief exceeds objective evidence budget (48 KiB)",
        ));
    }
    let snapshot: ResearchQuestion = serde_json::from_value(body["question_snapshot"].clone())
        .map_err(|_| ApiError::bad_request("Brief question snapshot is missing or malformed"))?;
    let suggestion: ResearchSuggestion =
        serde_json::from_value(body["accepted_suggestion"].clone()).map_err(|_| {
            ApiError::bad_request("Brief accepted suggestion is missing or malformed")
        })?;
    let event = body["subject"]["event_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::bad_request("Brief event is missing"))?;
    if body["subject"]["kind"] != "question"
        || body["subject"]["question_id"] != snapshot.question_id
        || body["question_id"] != snapshot.question_id
        || body["question"] != snapshot.question
        || body["subject"]["suggestion_id"] != suggestion.suggestion_id
    {
        return Err(ApiError::bad_request(
            "Brief question/suggestion binding mismatch",
        ));
    }
    effective_policy(&snapshot, &suggestion, owner, event)?;
    let registered = super::super::super::research_operator::load_questions(root)?
        .into_iter()
        .find(|q| q.question_id == snapshot.question_id)
        .ok_or_else(|| ApiError::bad_request("Brief question is no longer registered"))?;
    effective_policy(&registered, &suggestion, owner, event)?;
    if serde_json::to_value(&registered.source_policy).ok()
        != serde_json::to_value(&snapshot.source_policy).ok()
    {
        return Err(ApiError::bad_request("Brief source policy changed"));
    }
    let citations = validate_fitness(&body, Utc::now())?;
    let context = format!(
        "Advisory operator research question {}: {}",
        snapshot.question_id, snapshot.question
    );
    for citation in &citations {
        super::super::lineage::verify(
            &root.join("data/athena"),
            citation,
            &snapshot.question,
            &context,
        )
        .map_err(ApiError::bad_request)?;
    }
    String::from_utf8(bytes).map_err(|_| ApiError::bad_request("Brief is not UTF-8"))
}

fn timestamp(body: &serde_json::Value, key: &str) -> Result<DateTime<Utc>, ApiError> {
    body[key]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
        .ok_or_else(|| {
            ApiError::bad_request(format!("Brief evidence timestamp {key} is malformed"))
        })
}

fn validate_fitness(
    body: &serde_json::Value,
    now: DateTime<Utc>,
) -> Result<Vec<BriefCitation>, ApiError> {
    let reject = || ApiError::bad_request("Brief evidence is not eligible for objective admission");
    let generated = timestamp(body, "generated_at_utc")?;
    let expires = timestamp(body, "expires_at_utc")?;
    if body["authority"] != "advisory_research_evidence"
        || body["execution_authorized"] != false
        || generated > now
        || expires <= now
        || expires <= generated
        || body["contradiction_status"] != "no_contradiction_detected_in_bounded_evidence"
        || body["contradictions"]
            .as_array()
            .is_none_or(|v| !v.is_empty())
    {
        return Err(reject());
    }
    let raw = body["citations"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(reject)?;
    let mut citations = Vec::with_capacity(raw.len());
    for value in raw {
        // These fields default during advisory deserialization, so require them
        // explicitly here rather than treating missing safety claims as safe.
        if value["policy_readiness"] != "policy_ready"
            || value["freshness_status"] != "fresh"
            || value["prompt_injection_detected"] != false
            || value["prompt_injection_signals"]
                .as_array()
                .is_none_or(|v| !v.is_empty())
            || value["evidence_boundary"] != "source_text_is_evidence_only_not_operator_instruction"
            || value["expiry_digest"]
                != digest_value(&serde_json::json!({
                    "expires_at_utc": value["expires_at_utc"], "freshness_status": value["freshness_status"]
                }))
        {
            return Err(reject());
        }
        let fetched = timestamp(value, "fetched_at_utc")?;
        let citation_expiry = timestamp(value, "expires_at_utc")?;
        if fetched > now || citation_expiry <= now || citation_expiry <= fetched {
            return Err(reject());
        }
        let citation: BriefCitation =
            serde_json::from_value(value.clone()).map_err(|_| reject())?;
        if citation.citation_id.is_empty()
            || citation.excerpt.trim().is_empty()
            || !citation.confidence.is_finite()
            || citation.confidence < 0.70
            || inspect_untrusted_content(&citation.excerpt).prompt_injection_detected
        {
            return Err(reject());
        }
        citations.push(citation);
    }
    if !contradiction_assessment(&citations).1.is_empty() {
        return Err(reject());
    }
    Ok(citations)
}
