// sigil: ANKH
use crate::contract::ArtifactPolicy;
use serde::{Deserialize, Serialize};

pub const GOVERNANCE_VALIDATORS: [&str; 5] = [
    "triad",
    "bacon_lite",
    "joulework",
    "love_equation",
    "soterion_trace",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineeringDomain {
    SoftwareSystems,
    HardwareIntegration,
    PhysicalFabrication,
    SystemsResearch,
    TechnicalDocumentation,
    SceneAssets, // ARDA HUD boardroom, world, etc.
    Fabrication, // 3D printing
    CadEngineering,
    Research,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStage {
    Research,
    Design,
    Build,
    Verify,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    PbrTextures,
    Glb,
    PrintReadyStl,
    Gcode,
    BlenderFile,
    MetadataJson,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ForgeWorkItem {
    pub domain: EngineeringDomain,
    pub description: String,
    pub has_research: bool,
    pub has_build_artifact: bool,
    pub target_output: Vec<ArtifactType>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ForgeWorkflowPlan {
    pub domain: EngineeringDomain,
    pub next_stage: WorkflowStage,
    pub artifact_policy: ArtifactPolicy,
    pub governance_validators: [&'static str; 5],
    pub documentation_required: bool,
    pub work_item: ForgeWorkItem,
    pub steps: Vec<String>,
    pub estimated_time_minutes: u32,
}

impl ForgeWorkItem {
    pub fn next_stage(&self) -> WorkflowStage {
        if !self.has_research {
            WorkflowStage::Research
        } else if !self.has_build_artifact {
            WorkflowStage::Build
        } else if matches!(self.domain, EngineeringDomain::TechnicalDocumentation) {
            WorkflowStage::Document
        } else {
            WorkflowStage::Verify
        }
    }

    pub fn plan(&self) -> ForgeWorkflowPlan {
        let next_stage = self.next_stage();
        let artifact_policy = if !self.has_research {
            ArtifactPolicy::ResearchOnly
        } else if self.has_build_artifact {
            ArtifactPolicy::VerificationRequired
        } else {
            ArtifactPolicy::PrototypeAllowed
        };

        ForgeWorkflowPlan {
            domain: self.domain,
            next_stage,
            artifact_policy,
            governance_validators: GOVERNANCE_VALIDATORS,
            documentation_required: matches!(
                self.domain,
                EngineeringDomain::TechnicalDocumentation
            ) || matches!(next_stage, WorkflowStage::Document),
            work_item: self.clone(),
            steps: Vec::new(),
            estimated_time_minutes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresearched_work_stays_research_only() {
        let plan = ForgeWorkItem {
            domain: EngineeringDomain::SystemsResearch,
            description: "Test".to_string(),
            has_research: false,
            has_build_artifact: false,
            target_output: vec![],
        }
        .plan();

        assert_eq!(plan.next_stage, WorkflowStage::Research);
        assert_eq!(plan.artifact_policy, ArtifactPolicy::ResearchOnly);
        assert!(!plan.documentation_required);
    }

    #[test]
    fn researched_unbuilt_work_allows_prototype() {
        let plan = ForgeWorkItem {
            domain: EngineeringDomain::HardwareIntegration,
            description: "Test".to_string(),
            has_research: true,
            has_build_artifact: false,
            target_output: vec![],
        }
        .plan();

        assert_eq!(plan.next_stage, WorkflowStage::Build);
        assert_eq!(plan.artifact_policy, ArtifactPolicy::PrototypeAllowed);
    }

    #[test]
    fn completed_documentation_work_routes_to_document_stage() {
        let plan = ForgeWorkItem {
            domain: EngineeringDomain::TechnicalDocumentation,
            description: "Test".to_string(),
            has_research: true,
            has_build_artifact: true,
            target_output: vec![],
        }
        .plan();

        assert_eq!(plan.next_stage, WorkflowStage::Document);
        assert_eq!(plan.artifact_policy, ArtifactPolicy::VerificationRequired);
        assert!(plan.documentation_required);
        assert_eq!(plan.governance_validators.len(), 5);
    }
}
