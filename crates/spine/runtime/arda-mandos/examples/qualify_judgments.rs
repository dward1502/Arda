// sigil: REPAIR
//! Offline R08 qualification: real Mandos judgments, never execution authority.
use arda_mandos::{OracleEngine, OracleQuery};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    task: String,
    context: Vec<String>,
}

fn qualify(bytes: &[u8]) -> anyhow::Result<Value> {
    let cases: Vec<Case> = serde_json::from_slice(bytes)?;
    anyhow::ensure!(!cases.is_empty(), "empty corpus");
    let mut seen = BTreeSet::new();
    let mut engine = OracleEngine::new();
    let mut results = Vec::new();
    for case in cases {
        anyhow::ensure!(seen.insert(case.id.clone()), "duplicate case id");
        let mut query = OracleQuery::new(case.id, case.task, "r08-offline-qualification");
        query.context = case.context;
        results.push(engine.evaluate(query)?.redacted_for_export());
    }
    Ok(
        json!({"schema":"r08.judgments.v1", "corpus_sha256":format!("{:x}", Sha256::digest(bytes)), "execution_authority":false, "verdicts":results}),
    )
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: qualify_judgments CORPUS.json"))?;
    let report = qualify(&std::fs::read(path)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_and_empty_corpora() {
        assert!(qualify(b"[]").is_err());
        assert!(qualify(br#"[{"id":"a","task":"local note","context":[]},{"id":"a","task":"local note","context":[]}]"#).is_err());
    }
    #[test]
    fn emits_real_versioned_advice_bound_to_corpus() {
        let bytes = br#"[{"id":"a","task":"local research note","context":[]}]"#;
        let out = qualify(bytes).unwrap();
        assert_eq!(out["execution_authority"], false);
        assert_eq!(out["corpus_sha256"], format!("{:x}", Sha256::digest(bytes)));
        let expected = OracleEngine::new()
            .evaluate(OracleQuery::new(
                "a",
                "local research note",
                "r08-offline-qualification",
            ))
            .unwrap();
        assert_eq!(
            out["verdicts"][0]["outcome"],
            serde_json::to_value(expected.outcome).unwrap()
        );
        assert_eq!(
            out["verdicts"][0]["policy_version"],
            expected.policy_version
        );
    }
}
