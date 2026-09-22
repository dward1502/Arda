//! Synthetic trusted-store evidence, never authentic external research.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn digest(value: &Value) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap())
    )
}

pub fn publish(root: &Path, original: &Value) -> (String, String, PathBuf) {
    publish_with(root, original, |_| {})
}

pub fn publish_with(
    root: &Path,
    original: &Value,
    change: impl FnOnce(&mut Value),
) -> (String, String, PathBuf) {
    let now = chrono::Utc::now();
    let expiry = (now + chrono::Duration::minutes(5)).to_rfc3339();
    let mut body = original.clone();
    body.as_object_mut().unwrap().remove("brief_id");
    body.as_object_mut().unwrap().remove("content_sha256");
    body["generated_at_utc"] = now.to_rfc3339().into();
    body["expires_at_utc"] = expiry.clone().into();
    body["contradiction_status"] = "no_contradiction_detected_in_bounded_evidence".into();
    body["contradictions"] = json!([]);
    let (url, id) = arda_varda::ingest::AthenaStore::canonical_source_identity(
        "https://example.invalid/fixture",
    );
    let pipeline = "fixture-admission-pipeline";
    let context = format!(
        "Advisory operator research question {}: {}",
        body["question_id"].as_str().unwrap(),
        body["question"].as_str().unwrap()
    );
    let athena = root.join("data/athena");
    fs::create_dir_all(athena.join("books")).unwrap();
    let artifact = athena
        .join("crawls")
        .join(format!("{id}.md"))
        .to_string_lossy()
        .into_owned();
    let evaluation = json!({"policy_readiness":"policy_ready", "confidence":0.8,"triad_passed":true,"extraction_status":"fixture"});
    fs::write(athena.join("digest.jsonl"), format!("{}\n",json!({"id":id,"pipeline_id":pipeline,"url":url,"submitted_by":"workbench_research","task_context":context,"quarantine":false,"error":null}))).unwrap();
    fs::write(athena.join("crawl_receipts.jsonl"),format!("{}\n",json!({"source_id":id,"pipeline_id":pipeline,"url":url,"query":body["question"],"task_context":context,"submitted_by":"workbench_research","crawl_service_url":"workbench://canonical-http-fetch","filter":"canonical_http_text","success":true,"artifact_path":artifact}))).unwrap();
    fs::write(athena.join("books").join(format!("{id}.jsonl")), format!("{}\n",json!({"stage":"deep","pipeline_id":pipeline,"data":{"policy_readiness":"policy_ready","confidence":0.8,"triad_analysis":{"passed":true},"extraction_status":"fixture"}}))).unwrap();
    body["citations"] = json!([{
        "citation_id":"fixture-citation","title":"Fixture only","discovered_url":url,"canonical_url":url,
        "content_sha256":"fixture-not-raw-HTTP-proof","excerpt":"Retain this exact fixture evidence.","stance":"supporting_or_contextual",
        "varda_source_id":id,"varda_pipeline_id":pipeline,"crawl_receipt_path":artifact,
        "policy_readiness":"policy_ready","confidence":0.8,"evaluation_digest":digest(&evaluation),
        "fetched_at_utc":now.to_rfc3339(),"expires_at_utc":expiry,"freshness_status":"fresh",
        "expiry_digest":digest(&json!({"expires_at_utc":expiry,"freshness_status":"fresh"})),
        "evidence_boundary":"source_text_is_evidence_only_not_operator_instruction",
        "prompt_injection_detected":false,"prompt_injection_signals":[]
    }]);
    change(&mut body);
    let hash = digest(&body);
    let brief_id = format!("question-brief-{}", hash.strip_prefix("sha256:").unwrap());
    body["brief_id"] = brief_id.clone().into();
    body["content_sha256"] = hash.into();
    let raw = format!("\n{}\n", serde_json::to_string_pretty(&body).unwrap());
    let path = root
        .join("data/workbench/research/briefs")
        .join(format!("{brief_id}.json"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &raw).unwrap();
    (brief_id, raw, path)
}
