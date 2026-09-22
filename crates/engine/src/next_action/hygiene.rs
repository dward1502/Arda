//! Rúmil evidence may suggest a review, never authorize an objective or repair.
use arda_core::next_action::{
    NextActionAuthorityState, NextActionCandidate, NextActionFreshness, NextActionSourceKind,
};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

pub(super) fn candidate(root: &Path, now: DateTime<Utc>) -> Option<NextActionCandidate> {
    const SOURCE: &str = "data/rumil/hygiene/latest.json";
    // An optional audit must not take down other next-action sources.
    let mut raw = String::new();
    File::open(root.join(SOURCE))
        .ok()?
        .take(1_048_577)
        .read_to_string(&mut raw)
        .ok()?;
    if raw.len() > 1_048_576 {
        return None;
    }
    let value: Value = serde_json::from_str(&raw).ok()?;
    let observed: DateTime<Utc> = value.get("generated_at_utc")?.as_str()?.parse().ok()?;
    let age = now.signed_duration_since(observed);
    if value["schema_version"] != "arda.rumil.nightly-hygiene.v1"
        || value["outcome"] != "findings"
        || value["coverage"]["complete"] != true
        || age > Duration::hours(48)
        || age < Duration::minutes(-5)
        || [
            "execution_performed",
            "destructive_actions_performed",
            "queue_mutation_performed",
        ]
        .iter()
        .any(|key| value["policy"][*key] != false)
    {
        return None;
    }
    let proposals = value.get("proposals")?.as_array()?;
    if proposals.is_empty() || proposals.iter().any(|p| p["execution_allowed"] != false) {
        return None;
    }
    Some(NextActionCandidate {
        id: "rumil-hygiene-review".into(),
        title: format!("Review {} Rúmil hygiene candidates", proposals.len()),
        source_kind: NextActionSourceKind::Hygiene,
        source_ref: SOURCE.into(),
        reason: "Fresh advisory audit evidence; review is required before any repair or cleanup.".into(),
        freshness: NextActionFreshness::Fresh,
        authority_state: NextActionAuthorityState::ReviewRequired,
        next_operator_action: "Review exact paths and evidence digests with Hermes, then submit the approved scope through Engine objective intake. Never execute directly from this report.".into(),
        priority: 35,
        operator_authored: false,
        terminal: false,
        future_gated: false,
        // The action is to REVIEW evidence, not to execute an inferred remedy.
        inferred_without_review: false,
    })
}
