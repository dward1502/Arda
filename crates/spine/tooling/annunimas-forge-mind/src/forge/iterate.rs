//! Vision-feedback iteration loop on top of `forge generate`.
//!
//! Each round: forge generate → bpy render N angles → vision LLM compares
//! candidate angles to operator-supplied target → governance scores the
//! iteration envelope. Loop ends on (a) match_score ≥ accept_threshold,
//! (b) governance veto, or (c) budget exhaustion.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::forge::generate::{generate_asset, GenerateOverrides, GenerateSpec};
use crate::forge::governance::{evaluate, IterationGovernance};
use crate::forge::render::{render_glb_angles, RenderConfig};
use crate::tools::comfyui::ComfyUiClient;
use crate::tools::vision::{ComparisonReport, VisionClient};

pub const DEFAULT_ACCEPT_THRESHOLD: f64 = 0.85;
pub const DEFAULT_BUDGET_ITERS: u32 = 5;

#[derive(Debug, Clone)]
pub struct IterateSpec {
    pub target_image: PathBuf,
    pub asset_id: String,
    pub domain: String,
    pub initial_prompt: String,
    pub negative_prompt: String,
    pub assets_root: PathBuf,
    pub scene_binding: String,
    pub material_family: String,
    pub overrides: GenerateOverrides,
    pub budget_iters: u32,
    pub accept_threshold: f64,
    pub render_config: RenderConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct IterationRecord {
    pub iteration: u32,
    pub prompt: String,
    pub glb_path: PathBuf,
    pub reference_image_path: Option<PathBuf>,
    pub angle_renders: Vec<PathBuf>,
    pub comparison: ComparisonReport,
    pub governance: IterationGovernance,
    pub accepted: bool,
    pub elapsed_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IterateResult {
    pub asset_id: String,
    pub domain: String,
    pub accepted_iteration: Option<u32>,
    pub iterations: Vec<IterationRecord>,
    pub final_glb: PathBuf,
    pub summary_path: PathBuf,
    pub total_elapsed_secs: f64,
}

pub async fn iterate_asset(
    comfy: ComfyUiClient,
    vision: VisionClient,
    spec: IterateSpec,
) -> anyhow::Result<IterateResult> {
    if !spec.target_image.exists() {
        anyhow::bail!(
            "target reference image not found: {}",
            spec.target_image.display()
        );
    }
    let started = Instant::now();

    let asset_dir = spec.assets_root.join(&spec.domain).join(&spec.asset_id);
    let iter_root = asset_dir.join("iterations");
    std::fs::create_dir_all(&iter_root)?;

    let final_glb = asset_dir.join(format!("{}.glb", spec.asset_id));
    let summary_path = asset_dir.join("iterate_summary.json");

    let mut current_prompt = spec.initial_prompt.clone();
    let mut records: Vec<IterationRecord> = Vec::new();
    let mut accepted_at: Option<u32> = None;

    for round in 1..=spec.budget_iters {
        let iter_started = Instant::now();
        let iter_label = format!("iter{:02}", round);

        // Each iteration's generation goes under <domain>/<asset_id>/iterations/iterNN/
        let iter_domain = format!("{}/{}/iterations", spec.domain, spec.asset_id);
        let gen_spec = GenerateSpec {
            asset_id: iter_label.clone(),
            domain: iter_domain.clone(),
            positive_prompt: current_prompt.clone(),
            negative_prompt: spec.negative_prompt.clone(),
            assets_root: spec.assets_root.clone(),
            scene_binding: spec.scene_binding.clone(),
            material_family: spec.material_family.clone(),
            overrides: spec.overrides.clone(),
            post_cleanup_blender: false,
        };
        tracing::info!(
            target: "forge.iterate",
            round, %current_prompt, "starting round"
        );
        let generated = generate_asset(comfy.clone(), gen_spec).await?;

        // Render this iteration's GLB to N angles for vision comparison.
        let iter_dir = iter_root.join(&iter_label);
        std::fs::create_dir_all(&iter_dir)?;
        let renders = render_glb_angles(
            &generated.glb_path,
            &iter_dir,
            &iter_label,
            &spec.render_config,
        )?;
        let render_refs: Vec<&Path> = renders.iter().map(|p| p.as_path()).collect();

        // Vision compare.
        let report = vision
            .compare(&spec.target_image, &render_refs, &current_prompt, round)
            .await?;
        tracing::info!(
            target: "forge.iterate",
            round,
            match_score = report.match_score,
            missing = report.missing.len(),
            wrong = report.wrong.len(),
            "vision comparison complete"
        );

        // Governance evaluation + audit log.
        let governance = evaluate(
            &spec.asset_id,
            round,
            spec.budget_iters,
            &report,
            &current_prompt,
        )?;

        let accepted = report.match_score >= spec.accept_threshold;
        let veto = governance.veto_stop;
        let next_prompt = if !accepted && !report.suggested_prompt_edit.trim().is_empty() {
            report.suggested_prompt_edit.clone()
        } else {
            current_prompt.clone()
        };

        let rec = IterationRecord {
            iteration: round,
            prompt: current_prompt.clone(),
            glb_path: generated.glb_path.clone(),
            reference_image_path: generated.reference_image_path.clone(),
            angle_renders: renders,
            comparison: report,
            governance,
            accepted,
            elapsed_secs: iter_started.elapsed().as_secs_f64(),
        };
        let iter_meta_path = iter_dir.join("iteration.json");
        std::fs::write(&iter_meta_path, serde_json::to_string_pretty(&rec)?)?;
        records.push(rec);

        if accepted {
            accepted_at = Some(round);
            // Promote this iteration's GLB to the canonical asset path.
            let accepted_record = records.last().ok_or_else(|| {
                anyhow::anyhow!(
                    "forge iteration accepted but no iteration record exists for {} round {}",
                    spec.asset_id,
                    round
                )
            })?;
            std::fs::copy(&accepted_record.glb_path, &final_glb)?;
            tracing::info!(
                target: "forge.iterate",
                round, dest = %final_glb.display(),
                "iteration accepted; promoted to canonical asset path"
            );
            break;
        }
        if veto {
            let veto_reason = records
                .last()
                .map(|record| record.governance.veto_reason.clone())
                .unwrap_or_else(|| Some("forge iteration vetoed before record persistence".into()));
            tracing::warn!(
                target: "forge.iterate",
                round,
                reason = ?veto_reason,
                "governance veto; stopping loop"
            );
            break;
        }

        current_prompt = next_prompt;
    }

    // If no acceptance, promote the highest-match-score iteration to the canonical path
    // (still useful as the best-so-far; sidecar in metadata.json points to which round it was).
    if accepted_at.is_none() && !records.is_empty() {
        let best = records
            .iter()
            .max_by(|a, b| {
                a.comparison
                    .match_score
                    .partial_cmp(&b.comparison.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "forge iteration completed records for {} but no best candidate could be selected",
                    spec.asset_id
                )
            })?;
        std::fs::copy(&best.glb_path, &final_glb)?;
        tracing::warn!(
            target: "forge.iterate",
            best_round = best.iteration,
            best_score = best.comparison.match_score,
            "budget exhausted without accept; promoted best-so-far"
        );
    }

    let result = IterateResult {
        asset_id: spec.asset_id.clone(),
        domain: spec.domain.clone(),
        accepted_iteration: accepted_at,
        iterations: records,
        final_glb,
        summary_path: summary_path.clone(),
        total_elapsed_secs: started.elapsed().as_secs_f64(),
    };
    std::fs::write(&summary_path, serde_json::to_string_pretty(&result)?)?;
    Ok(result)
}
