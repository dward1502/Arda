//! Receipt-driven Warden import and Varda approval boundary.

use arda_outpost_protocol::{
    validate_research_chain, AcknowledgementReceipt, ExternalObservationReceipt,
    PersistedResearchChain, ResearchDispatch, ResearchReceiptLedger, ResearchSuggestion,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::ingest::crawl::crawl4ai_fetch_markdown;
use crate::learning::KnowledgeDelta;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationDecision {
    ApprovedSafeLocal,
    ReviewRequired,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalEvaluationReceipt {
    pub schema_version: String,
    pub suggestion_id: String,
    pub dispatch_id: String,
    pub observation_id: String,
    pub normalized_url: String,
    pub retrieved_at_utc: DateTime<Utc>,
    pub content_hash: String,
    pub decision: EvaluationDecision,
    pub rationale: String,
    pub approval_reference: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalLaneInput<'a> {
    pub suggestion: &'a ResearchSuggestion,
    pub dispatch: &'a ResearchDispatch,
    pub observation: &'a ExternalObservationReceipt,
    pub acknowledgement: &'a AcknowledgementReceipt,
    pub canonical_url: &'a str,
    pub canonical_content: &'a str,
    pub retrieved_at_utc: DateTime<Utc>,
    pub privacy_risk: bool,
    pub contradiction: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExternalLaneError {
    #[error("invalid Warden receipt chain: {0}")]
    Chain(#[from] arda_outpost_protocol::ResearchReceiptError),
    #[error("canonical URL does not match the Warden observation")]
    CanonicalUrlMismatch,
    #[error("canonical content is required; snippets cannot be approved")]
    MissingCanonicalContent,
    #[error("canonical content does not match the Warden content hash")]
    ContentHashMismatch,
    #[error("canonical URL provenance does not match the Warden provenance hash")]
    ProvenanceHashMismatch,
    #[error("approved delta requires an approval receipt")]
    MissingApproval,
    #[error("ledger error: {0}")]
    Io(#[from] std::io::Error),
    #[error("crawl provider error: {0}")]
    Crawl(#[from] arda_core::error::ArdaError),
}

pub fn evaluate_external_lane(
    input: ExternalLaneInput<'_>,
    now: DateTime<Utc>,
) -> Result<ExternalEvaluationReceipt, ExternalLaneError> {
    validate_research_chain(
        input.suggestion,
        input.dispatch,
        input.observation,
        input.acknowledgement,
        now,
    )?;
    if input.canonical_url != input.observation.normalized_url {
        return Err(ExternalLaneError::CanonicalUrlMismatch);
    }
    if input.canonical_content.trim().is_empty() {
        return Err(ExternalLaneError::MissingCanonicalContent);
    }
    if sha256_hex(input.canonical_content.as_bytes()) != input.observation.content_hash {
        return Err(ExternalLaneError::ContentHashMismatch);
    }
    if sha256_hex(input.canonical_url.as_bytes()) != input.observation.provenance_hash {
        return Err(ExternalLaneError::ProvenanceHashMismatch);
    }
    let (decision, rationale, approval_reference) = if input.privacy_risk || input.contradiction {
        (
            EvaluationDecision::ReviewRequired,
            "privacy or contradiction signal requires governed review".to_owned(),
            None,
        )
    } else {
        (
            EvaluationDecision::ApprovedSafeLocal,
            "canonical content, provenance, and advisory parent chain validated".to_owned(),
            Some(format!("approval:{}", input.observation.observation_id)),
        )
    };
    Ok(ExternalEvaluationReceipt {
        schema_version: "arda.athena.external_evaluation.v1".to_owned(),
        suggestion_id: input.suggestion.suggestion_id.clone(),
        dispatch_id: input.dispatch.dispatch_id.clone(),
        observation_id: input.observation.observation_id.clone(),
        normalized_url: input.observation.normalized_url.clone(),
        retrieved_at_utc: input.retrieved_at_utc,
        content_hash: input.observation.content_hash.clone(),
        decision,
        rationale,
        approval_reference,
    })
}

fn sha256_hex(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn approved_delta(
    receipt: &ExternalEvaluationReceipt,
    content: &str,
) -> Result<KnowledgeDelta, ExternalLaneError> {
    if receipt.decision != EvaluationDecision::ApprovedSafeLocal
        || receipt.approval_reference.is_none()
    {
        return Err(ExternalLaneError::MissingApproval);
    }
    Ok(KnowledgeDelta::new(
        &format!(
            "{}#approval={}",
            receipt.normalized_url,
            receipt.approval_reference.as_deref().unwrap()
        ),
        0.8,
        0.2,
        content,
        86_400,
    ))
}

pub fn append_evaluation_receipt(
    receipt: &ExternalEvaluationReceipt,
    path: impl AsRef<Path>,
) -> Result<(), ExternalLaneError> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    serde_json::to_writer(&mut file, receipt).map_err(std::io::Error::other)?;
    use std::io::Write;
    writeln!(file)?;
    file.sync_data()?;
    Ok(())
}

fn supersede_expired_chain(
    ledger: &ResearchReceiptLedger,
    evaluation_ledger_path: impl AsRef<Path>,
    sequence: u64,
    chain: &PersistedResearchChain,
    now: DateTime<Utc>,
) -> Result<Option<ExternalEvaluationReceipt>, ExternalLaneError> {
    if chain.suggestion.expires_at_utc > now {
        return Ok(None);
    }

    // The chain was valid when Warden emitted it, but its bounded authority no
    // longer permits a new canonical fetch. Validate the persisted parent
    // links at the suggestion's creation boundary, then record a terminal
    // non-promotable disposition so an expired head cannot poison the lane.
    validate_research_chain(
        &chain.suggestion,
        &chain.dispatch,
        &chain.observation,
        &chain.acknowledgement,
        chain.suggestion.created_at_utc,
    )?;
    let receipt = ExternalEvaluationReceipt {
        schema_version: "arda.athena.external_evaluation.v1".to_owned(),
        suggestion_id: chain.suggestion.suggestion_id.clone(),
        dispatch_id: chain.dispatch.dispatch_id.clone(),
        observation_id: chain.observation.observation_id.clone(),
        normalized_url: chain.observation.normalized_url.clone(),
        retrieved_at_utc: chain.observation.observed_at_utc,
        content_hash: chain.observation.content_hash.clone(),
        decision: EvaluationDecision::Superseded,
        rationale: "research suggestion expired before Varda evaluation; canonical refetch skipped"
            .to_owned(),
        approval_reference: None,
    };
    append_evaluation_receipt(&receipt, evaluation_ledger_path)?;
    ledger.advance_cursor(
        "observations",
        sequence,
        chain.observation.observation_id.clone(),
    )?;
    Ok(Some(receipt))
}

/// Consume the next persisted Warden chain exactly once against a canonical
/// fetch result. The cursor advances only after evaluation receipt persistence.
pub fn import_next_canonical_result(
    research_ledger_path: impl AsRef<Path>,
    evaluation_ledger_path: impl AsRef<Path>,
    canonical_url: &str,
    canonical_content: &str,
    retrieved_at_utc: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Option<ExternalEvaluationReceipt>, ExternalLaneError> {
    let ledger = ResearchReceiptLedger::open(research_ledger_path)?;
    let cursor = ledger.read_cursor("observations")?;
    let chains = ledger.complete_chains()?;
    let next = chains
        .into_iter()
        .enumerate()
        .find(|(index, _)| (*index as u64) >= cursor.sequence)
        .map(|(index, chain)| (index as u64 + 1, chain));
    let Some((sequence, chain)) = next else {
        return Ok(None);
    };
    if let Some(receipt) =
        supersede_expired_chain(&ledger, &evaluation_ledger_path, sequence, &chain, now)?
    {
        return Ok(Some(receipt));
    }
    let receipt = evaluate_external_lane(
        ExternalLaneInput {
            suggestion: &chain.suggestion,
            dispatch: &chain.dispatch,
            observation: &chain.observation,
            acknowledgement: &chain.acknowledgement,
            canonical_url,
            canonical_content,
            retrieved_at_utc,
            privacy_risk: false,
            contradiction: false,
        },
        now,
    )?;
    append_evaluation_receipt(&receipt, evaluation_ledger_path)?;
    ledger.advance_cursor("observations", sequence, chain.observation.observation_id)?;
    Ok(Some(receipt))
}

/// Fetch and evaluate the next persisted Warden observation through Crawl4AI.
/// The observation cursor advances only after the evaluation receipt is synced.
pub async fn import_next_from_crawl4ai(
    research_ledger_path: impl AsRef<Path>,
    evaluation_ledger_path: impl AsRef<Path>,
    crawl_service_url: &str,
    filter: &str,
    query: Option<&str>,
    retrieved_at_utc: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Option<ExternalEvaluationReceipt>, ExternalLaneError> {
    let ledger = ResearchReceiptLedger::open(research_ledger_path.as_ref())?;
    let cursor = ledger.read_cursor("observations")?;
    let chains = ledger.complete_chains()?;
    let Some(chain) = chains.get(cursor.sequence as usize) else {
        return Ok(None);
    };
    if let Some(receipt) = supersede_expired_chain(
        &ledger,
        &evaluation_ledger_path,
        cursor.sequence + 1,
        chain,
        now,
    )? {
        return Ok(Some(receipt));
    }
    let crawl = crawl4ai_fetch_markdown(
        crawl_service_url,
        &chain.observation.normalized_url,
        filter,
        query,
    )
    .await?;
    if !crawl.success {
        return Err(ExternalLaneError::Crawl(
            arda_core::error::ArdaError::Agent {
                agent: "varda".to_owned(),
                message: "crawl4ai returned an unsuccessful canonical result".to_owned(),
            },
        ));
    }
    import_next_canonical_result(
        research_ledger_path,
        evaluation_ledger_path,
        &chain.observation.normalized_url,
        &crawl.markdown,
        retrieved_at_utc,
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use tempfile::tempdir;

    #[test]
    fn canonical_evidence_approves_and_review_cannot_emit_delta() {
        let now = Utc::now();
        let suggestion = ResearchSuggestion::new(
            "bounded evidence",
            "varda-suggestion",
            now,
            now + Duration::minutes(5),
            2,
            1024,
        )
        .unwrap();
        let dispatch = ResearchDispatch::accepted(&suggestion, "varda-dispatch", now, 1).unwrap();
        let observation = ExternalObservationReceipt::completed(
            &suggestion,
            &dispatch,
            "https://example.com/evidence",
            sha256_hex(b"canonical fetched content"),
            sha256_hex(b"https://example.com/evidence"),
            now,
        )
        .unwrap();
        let acknowledgement =
            AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now).unwrap();
        let input = ExternalLaneInput {
            suggestion: &suggestion,
            dispatch: &dispatch,
            observation: &observation,
            acknowledgement: &acknowledgement,
            canonical_url: &observation.normalized_url,
            canonical_content: "canonical fetched content",
            retrieved_at_utc: now,
            privacy_risk: false,
            contradiction: false,
        };
        assert_eq!(
            sha256_hex(input.canonical_url.as_bytes()),
            observation.provenance_hash,
            "normalized URL: {:?}",
            input.canonical_url
        );
        let approved = evaluate_external_lane(input, now).unwrap();
        assert_eq!(approved.decision, EvaluationDecision::ApprovedSafeLocal);
        assert!(approved_delta(&approved, "approved knowledge")
            .unwrap()
            .is_valid_contract_shape());

        let review = evaluate_external_lane(
            ExternalLaneInput {
                privacy_risk: true,
                ..input
            },
            now,
        )
        .unwrap();
        assert_eq!(review.decision, EvaluationDecision::ReviewRequired);
        assert!(approved_delta(&review, "must not promote").is_err());
    }

    #[test]
    fn live_crawl_import_fetches_next_chain_and_advances_after_receipt() {
        let dir = tempdir().unwrap();
        let now = Utc::now();
        let suggestion = ResearchSuggestion::new(
            "live crawl import",
            "live-crawl-suggestion",
            now,
            now + Duration::minutes(5),
            2,
            1024,
        )
        .unwrap();
        let dispatch =
            ResearchDispatch::accepted(&suggestion, "live-crawl-dispatch", now, 1).unwrap();
        let content = "live canonical content";
        let url = "https://example.com/live";
        let observation = ExternalObservationReceipt::completed(
            &suggestion,
            &dispatch,
            url,
            sha256_hex(content.as_bytes()),
            sha256_hex(url.as_bytes()),
            now,
        )
        .unwrap();
        let acknowledgement =
            AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now).unwrap();
        let research_path = dir.path().join("warden.jsonl");
        ResearchReceiptLedger::open(&research_path)
            .unwrap()
            .append_complete_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            let response = serde_json::json!({
                "url": url,
                "markdown": content,
                "success": true,
            });
            let body = response.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let receipt = runtime
            .block_on(import_next_from_crawl4ai(
                &research_path,
                dir.path().join("evaluations.jsonl"),
                &format!("http://{address}"),
                "fit",
                None,
                now,
                now,
            ))
            .unwrap()
            .unwrap();
        assert_eq!(receipt.normalized_url, observation.normalized_url);
        assert_eq!(
            ResearchReceiptLedger::open(&research_path)
                .unwrap()
                .read_cursor("observations")
                .unwrap()
                .sequence,
            1
        );
    }

    #[test]
    fn expired_chain_is_superseded_without_fetch_and_advances_cursor() {
        let dir = tempdir().unwrap();
        let now = Utc::now();
        let created_at = now - Duration::minutes(10);
        let suggestion = ResearchSuggestion::new(
            "expired crawl import",
            "expired-crawl-suggestion",
            created_at,
            now - Duration::minutes(5),
            2,
            1024,
        )
        .unwrap();
        let dispatch =
            ResearchDispatch::accepted(&suggestion, "expired-crawl-dispatch", created_at, 1)
                .unwrap();
        let url = "https://example.com/expired";
        let observation = ExternalObservationReceipt::completed(
            &suggestion,
            &dispatch,
            url,
            sha256_hex(b"expired canonical content"),
            sha256_hex(url.as_bytes()),
            created_at,
        )
        .unwrap();
        let acknowledgement =
            AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, created_at)
                .unwrap();
        let research_path = dir.path().join("warden.jsonl");
        let evaluation_path = dir.path().join("evaluations.jsonl");
        ResearchReceiptLedger::open(&research_path)
            .unwrap()
            .append_complete_chain(
                &suggestion,
                &dispatch,
                &observation,
                &acknowledgement,
                created_at,
            )
            .unwrap();

        let receipt = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(import_next_from_crawl4ai(
                &research_path,
                &evaluation_path,
                "http://127.0.0.1:1",
                "fit",
                None,
                now,
                now,
            ))
            .unwrap()
            .unwrap();

        assert_eq!(receipt.decision, EvaluationDecision::Superseded);
        assert!(receipt.approval_reference.is_none());
        assert!(approved_delta(&receipt, "must not promote").is_err());
        assert_eq!(
            ResearchReceiptLedger::open(&research_path)
                .unwrap()
                .read_cursor("observations")
                .unwrap()
                .sequence,
            1
        );
        let persisted: ExternalEvaluationReceipt =
            serde_json::from_str(std::fs::read_to_string(evaluation_path).unwrap().trim()).unwrap();
        assert_eq!(persisted.decision, EvaluationDecision::Superseded);
    }
}
