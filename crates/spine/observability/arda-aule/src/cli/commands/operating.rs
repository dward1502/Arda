#![cfg(feature = "full-cli")]
use super::super::*;
use arda_vaire::InformantEvent;

pub(crate) fn handle_council(command: CouncilCommands) -> anyhow::Result<()> {
    match command {
        CouncilCommands::RecommendImprovements {
            root,
            scope,
            limit,
            append_recommendations,
        } => {
            let root = arandur::resolve_root(root);
            let scan = arandur::scan_arandur_improvements(&root)?;
            let recommendations = arandur::recommend_arandur_next(&root, !append_recommendations)?;
            let ledger = arandur::summarize_arandur_recommendations(&root)?;
            let out = json!({
                "contract": "arda.council.recommend_improvements.v1",
                "generated_at_utc": Utc::now().to_rfc3339(),
                "scope": scope,
                "limit": limit,
                "append_recommendations": append_recommendations,
                "mutation_policy": if append_recommendations {
                    "append_review_required_recommendations_only"
                } else {
                    "read_only_dry_run"
                },
                "agent_lanes": {
                    "athena": "knowledge evidence",
                    "mnemosyne": "operating memory",
                    "oracle": "review gate",
                    "prometheus": "strategy and task flow",
                    "apollo": "bounded execution",
                    "hades": "audit and cleanup"
                },
                "improvement_scan": scan,
                "recommendations": recommendations,
                "recommendation_ledger": ledger
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&limit_json_array(out, limit))?
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_venture(command: VentureCommands) -> anyhow::Result<()> {
    match command {
        VentureCommands::Evaluate {
            query,
            root,
            scope,
            limit,
        } => {
            let root = arandur::resolve_root(root);
            let athena = AthenaStore::new(root.join("data/athena"))?;
            let athena_matches = athena.query(&query, limit.max(1))?;
            let mnemosyne = MnemosyneService::new(root.join("data/mnemosyne"))?;
            let memory_seeds = mnemosyne.recall_knowledge_seeds(Some(&query), limit.max(1))?;
            let evidence_count = athena_matches.matches.len() + memory_seeds.len();
            let score = venture_score(&athena_matches, &memory_seeds);
            let recommendation = if evidence_count == 0 {
                "insufficient_evidence_ingest_market_sources_first"
            } else if score >= 0.75 {
                "promote_to_review_gated_opportunity_brief"
            } else if score >= 0.45 {
                "continue_research_before_commitment"
            } else {
                "do_not_promote_without_stronger_evidence"
            };
            let out = json!({
                "contract": "arda.venture.evaluate.v1",
                "generated_at_utc": Utc::now().to_rfc3339(),
                "scope": scope,
                "query": query,
                "limit": limit,
                "score": score,
                "recommendation": recommendation,
                "review_required": true,
                "mutation_policy": "read_only_evaluation_no_commitment_no_spend",
                "evidence": {
                    "athena": athena_matches,
                    "mnemosyne_memory_seeds": memory_seeds
                },
                "next_actions": venture_next_actions(recommendation)
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}

fn venture_score(
    athena_matches: &arda_varda::ingest::QueryResponse,
    memory_seeds: &[KnowledgeSeedRecallEntry],
) -> f64 {
    let athena_signal = athena_matches
        .matches
        .iter()
        .map(|entry| entry.score.min(4.0) / 4.0)
        .fold(0.0_f64, f64::max);
    let memory_signal = memory_seeds
        .iter()
        .map(|entry| entry.score.min(2.0) / 2.0)
        .fold(0.0_f64, f64::max);
    let coverage_bonus =
        ((athena_matches.matches.len() + memory_seeds.len()) as f64 / 10.0).min(0.2);
    ((athena_signal * 0.55) + (memory_signal * 0.25) + coverage_bonus).clamp(0.0, 1.0)
}

fn venture_next_actions(recommendation: &str) -> Vec<&'static str> {
    match recommendation {
        "promote_to_review_gated_opportunity_brief" => vec![
            "create opportunity brief with cited evidence paths",
            "ask ORACLE to challenge assumptions",
            "queue bounded prototype or outreach task only after review",
        ],
        "continue_research_before_commitment" => vec![
            "ingest competitor/customer/problem evidence",
            "run venture evaluate again with a narrower query",
            "avoid spend or launch commitment until review gate passes",
        ],
        _ => vec![
            "ingest more source material through ATHENA",
            "capture operator assumptions as Mnemosyne memory seeds",
            "rerun evaluation after evidence coverage improves",
        ],
    }
}

fn limit_json_array(mut value: serde_json::Value, limit: usize) -> serde_json::Value {
    if let Some(items) = value
        .get_mut("improvement_scan")
        .and_then(|scan| scan.get_mut("improvements"))
        .and_then(|items| items.as_array_mut())
    {
        items.truncate(limit.max(1));
    }
    if let Some(items) = value
        .get_mut("recommendations")
        .and_then(|recommendations| recommendations.get_mut("recommendations"))
        .and_then(|items| items.as_array_mut())
    {
        items.truncate(limit.max(1));
    }
    value
}
