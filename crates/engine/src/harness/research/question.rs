//! One-shot question-scoped advisory research. No run/objective is fabricated.
use super::*;
pub(in crate::harness) mod admission;
use arda_outpost_protocol::{ResearchQuestion, ResearchSuggestion, WatchlistState};
use std::io::{Read, Write};
#[cfg(test)]
mod tests;

pub(in crate::harness) async fn answer(
    state: &HarnessState,
    question_id: &str,
    suggestion: ResearchSuggestion,
    event_id: &str,
) -> Result<(String, Vec<String>), ApiError> {
    let question = super::super::research_operator::load_questions(&state.workbench_root)?
        .into_iter()
        .find(|q| q.question_id == question_id)
        .ok_or_else(|| ApiError::not_found("Research question is missing"))?;
    let policy = effective_policy(&question, &suggestion, &state.operator_id, event_id)?;
    let generated_at = Utc::now();
    let expires_at = question
        .expires_at_utc
        .min(generated_at + ChronoDuration::minutes(15));
    let context = format!(
        "Advisory operator research question {}: {}",
        question.question_id, question.question
    );
    let evaluated = evaluate_sources(
        state,
        SourceQuery {
            question: &question.question,
            context: &context,
            https_only: true,
        },
        question.source_policy.max_sources_per_run.min(MAX_SOURCES),
        expires_at,
        &policy,
    )
    .await?;
    let (status, contradictions) = contradiction_assessment(&evaluated.citations);
    let uncertainty = uncertainty_items(
        &evaluated.citations,
        &evaluated.source_failures,
        &contradictions,
    );
    let summary = summarize(
        &question.question,
        &evaluated.citations,
        &evaluated.source_failures,
    );
    let brief = serde_json::json!({
        "schema_version": "arda.research.question-brief.v1",
        "subject": {"kind": "question", "question_id": question.question_id, "owner": question.owner,
            "suggestion_id": suggestion.suggestion_id, "event_id": event_id},
        "question_id": question.question_id, "question": question.question,
        "question_snapshot": question, "accepted_suggestion": suggestion,
        "generated_at_utc": generated_at.to_rfc3339(), "expires_at_utc": expires_at.to_rfc3339(),
        "authority": "advisory_research_evidence", "execution_authorized": false,
        "evaluation_mode": "athena-deterministic-scaffold-v1",
        "scope": policy, "warden_provider": evaluated.warden.report.provider,
        "warden_memory_receipt": evaluated.warden.memory.memory_id,
        "summary": summary, "citations": evaluated.citations,
        "claims": claims_from_citations(&evaluated.citations),
        "source_failures": evaluated.source_failures, "contradiction_status": status,
        "contradictions": contradictions, "uncertainty": uncertainty,
        "missing_evidence": missing_evidence_items(&evaluated.citations, &evaluated.source_failures),
        "receipt_references": receipt_references(&evaluated.citations, evaluated.warden.memory.memory_id.as_deref())
    });
    let (brief_id, path) = publish(&state.workbench_root, brief)?;
    let assimilation = persist_assimilation_discoveries(
        &state.workbench_root,
        &brief_id,
        &path,
        &evaluated.citations,
        generated_at,
    );
    // A secondary assimilation failure must never hide the published advisory.
    let status = if assimilation.is_ok() {
        "recorded"
    } else {
        "failed"
    };
    let status_dir = state
        .workbench_root
        .join("data/workbench/research/assimilation-status");
    let write_status = || -> std::io::Result<()> {
        fs::create_dir_all(&status_dir)?;
        let mut file = tempfile::NamedTempFile::new_in(&status_dir)?;
        file.write_all(status.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(status_dir.join(&brief_id))
            .map_err(|e| e.error)?;
        fs::File::open(&status_dir)?.sync_all()
    };
    if write_status().is_err() {
        tracing::warn!("Research published; assimilation status could not be saved");
    }
    read_answer(&state.workbench_root, &state.operator_id, &brief_id)
}

pub(in crate::harness) fn recover_answer(
    root: &Path,
    owner: &str,
    event_id: &str,
) -> Result<Option<(String, Vec<String>)>, ApiError> {
    let directory = root.join("data/workbench/research/briefs");
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ApiError::internal(e.to_string())),
    };
    let mut result = None;
    for entry in entries {
        let path = entry.map_err(|e| ApiError::internal(e.to_string()))?.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let body: serde_json::Value = match read_brief_bytes(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            Some(body) => body,
            None => {
                // This scan has no target association yet. A corrupt candidate
                // cannot authorize replay; known targets use direct verified reads.
                tracing::warn!("Skipping unreadable research publication during reconciliation");
                continue;
            }
        };
        if body["subject"]["event_id"] == event_id && body["subject"]["owner"] == owner {
            if result.is_some() {
                return Err(ApiError::internal(
                    "Multiple research publications for one event",
                ));
            }
            let id = path
                .file_stem()
                .and_then(|v| v.to_str())
                .ok_or_else(|| ApiError::internal("Invalid publication path"))?;
            result = Some(read_bound_answer(root, owner, id, Some(event_id))?);
        }
    }
    Ok(result)
}

/// Authenticated callers only. Verify the immutable content before projecting it.
pub(in crate::harness) fn read_answer(
    root: &Path,
    owner: &str,
    brief_id: &str,
) -> Result<(String, Vec<String>), ApiError> {
    read_bound_answer(root, owner, brief_id, None)
}

/// Original bytes are retained separately from the canonical semantic digest.
/// Verification establishes local integrity/binding, not publisher authenticity.
pub(in crate::harness) fn load_verified_brief(
    root: &Path,
    owner: &str,
    brief_id: &str,
    expected_event: Option<&str>,
) -> Result<(Vec<u8>, serde_json::Value), ApiError> {
    let suffix = brief_id
        .strip_prefix("question-brief-")
        .filter(|id| {
            id.len() == 64
                && id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or_else(|| ApiError::bad_request("Invalid question brief identifier"))?;
    let path = root
        .join("data/workbench/research/briefs")
        .join(format!("{brief_id}.json"));
    let bytes = read_brief_bytes(&path)?;
    let mut body: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| ApiError::internal("Question brief is malformed"))?;
    let stored_id = body.as_object_mut().and_then(|b| b.remove("brief_id"));
    let stored_digest = body
        .as_object_mut()
        .and_then(|b| b.remove("content_sha256"));
    let expected = format!("sha256:{suffix}");
    if stored_id.as_ref().and_then(|v| v.as_str()) != Some(brief_id)
        || stored_digest.as_ref().and_then(|v| v.as_str()) != Some(expected.as_str())
        || digest_value(&body) != expected
        || body["schema_version"] != "arda.research.question-brief.v1"
        || body["execution_authorized"] != false
    {
        return Err(ApiError::internal("Question brief integrity check failed"));
    }
    if body["subject"]["owner"] != owner {
        return Err(ApiError::forbidden(
            "Question brief belongs to another operator",
        ));
    }
    if expected_event.is_some_and(|event| body["subject"]["event_id"].as_str() != Some(event)) {
        return Err(ApiError::internal(
            "Question brief belongs to another event",
        ));
    }
    Ok((bytes, body))
}

fn read_brief_bytes(path: &Path) -> Result<Vec<u8>, ApiError> {
    const MAX_BYTES: u64 = 256 * 1024;
    let file = fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ApiError::not_found("Question brief is missing")
        } else {
            ApiError::internal(error.to_string())
        }
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ApiError::bad_request("Question brief exceeds byte limit"));
    }
    Ok(bytes)
}

pub(in crate::harness) fn read_bound_answer(
    root: &Path,
    owner: &str,
    brief_id: &str,
    expected_event: Option<&str>,
) -> Result<(String, Vec<String>), ApiError> {
    let (_, body) = load_verified_brief(root, owner, brief_id, expected_event)?;
    let question_id = body["question_id"]
        .as_str()
        .ok_or_else(|| ApiError::internal("Question brief has no subject"))?;
    let summary = body["summary"]
        .as_str()
        .ok_or_else(|| ApiError::internal("Question brief has no summary"))?;
    let citations: Vec<BriefCitation> = serde_json::from_value(body["citations"].clone())
        .map_err(|_| ApiError::internal("Question brief citations are malformed"))?;
    let uncertainty: Vec<String> = serde_json::from_value(body["uncertainty"].clone())
        .map_err(|_| ApiError::internal("Question brief uncertainty is malformed"))?;
    let expiry = body["expires_at_utc"]
        .as_str()
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .ok_or_else(|| ApiError::internal("Question brief expiry is malformed"))?;
    let mut delivered = format!("Research question {question_id}: {summary}\nEvaluation: deterministic scaffold, not provider-backed verification.");
    if expiry < Utc::now() {
        delivered.push_str(
            "\nEXPIRED advisory snapshot — not current evidence; no automatic refresh performed.",
        );
    }
    for citation in &citations {
        delivered.push_str(&format!(
            "\n{} — {}\n{}",
            citation.title.chars().take(120).collect::<String>(),
            citation.canonical_url,
            citation.excerpt
        ));
    }
    for warning in uncertainty.iter().take(3) {
        delivered.push_str(&format!("\nUncertainty: {warning}"));
    }
    if fs::read_to_string(
        root.join("data/workbench/research/assimilation-status")
            .join(brief_id),
    )
    .ok()
    .as_deref()
        != Some("recorded")
    {
        delivered.push_str(
            "\nAssimilation is pending or failed; the published advisory remains available.",
        );
    }
    delivered.push_str("\nNo commitment was created.");
    Ok((
        delivered,
        vec![
            format!("arda://research/briefs/{brief_id}"),
            format!("arda://research/questions/{question_id}"),
        ],
    ))
}

fn effective_policy(
    question: &ResearchQuestion,
    suggestion: &ResearchSuggestion,
    owner: &str,
    event_id: &str,
) -> Result<ResearchBetaPolicy, ApiError> {
    question
        .validate_at(Utc::now())
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    suggestion
        .validate_at(Utc::now())
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    if question.owner != owner
        || question.state != WatchlistState::Enabled
        || !question
            .backend_suggestion_ids
            .contains(&suggestion.suggestion_id)
        || suggestion.query != question.question
        || suggestion.idempotency_key
            != format!(
                "research-question:{}:gateway:{event_id}",
                question.question_id
            )
        || suggestion.expires_at_utc != question.expires_at_utc
        || suggestion.max_results != question.budgets.max_results
        || suggestion.budget_bytes != question.budgets.max_fetch_bytes
    {
        return Err(ApiError::forbidden(
            "Research question/suggestion owner or event binding mismatch",
        ));
    }
    // This first one-shot consumer supports only the gateway's explicit public-web
    // contract. Fail closed on custom scopes instead of silently broadening them.
    if question.source_policy.policy_id != "public-web"
        || question.source_policy.allow_private_targets
        || question.source_policy.allowed_sources != ["https://"]
    {
        return Err(ApiError::bad_request(
            "One-shot research requires the public HTTPS source policy",
        ));
    }
    let mut policy = ResearchBetaPolicy::default();
    policy.max_results = policy.max_results.min(question.budgets.max_results);
    policy.max_fetch_bytes = policy.max_fetch_bytes.min(question.budgets.max_fetch_bytes);
    policy.max_tokens = policy.max_tokens.min(question.budgets.max_tokens);
    policy.max_attempts = policy.max_attempts.min(question.budgets.max_attempts);
    policy
        .validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(policy)
}

fn publish(
    root: &Path,
    mut body: serde_json::Value,
) -> Result<(String, std::path::PathBuf), ApiError> {
    let digest = digest_value(&body);
    let brief_id = format!("question-brief-{}", digest.trim_start_matches("sha256:"));
    body["brief_id"] = brief_id.clone().into();
    body["content_sha256"] = digest.into();
    let directory = root.join("data/workbench/research/briefs");
    fs::create_dir_all(&directory).map_err(|e| ApiError::internal(e.to_string()))?;
    let path = directory.join(format!("{brief_id}.json"));
    let bytes = serde_json::to_vec(&body).map_err(|e| ApiError::internal(e.to_string()))?;
    let mut file = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    file.write_all(&bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| ApiError::internal(e.to_string()))?;
    match file.persist_noclobber(&path) {
        Ok(_) => (),
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&path).map_err(|e| ApiError::internal(e.to_string()))? != bytes {
                return Err(ApiError::internal(
                    "Immutable research brief content mismatch",
                ));
            }
        }
        Err(e) => return Err(ApiError::internal(e.to_string())),
    }
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((brief_id, path))
}
