use crate::workflow::{ForgeWorkItem, WorkflowPlan, EngineeringDomain};
use crate::contract::ArtifactPolicy;

pub struct ForgePlanner;

impl ForgePlanner {
    pub fn new() -> Self { Self }

    pub async fn create_plan(&self, item: &ForgeWorkItem) -> anyhow::Result<WorkflowPlan> {
        let steps = match item.domain {
            EngineeringDomain::SceneAssets => vec![
                "Research reference assets".to_string(),
                "Open or create Blender scene".to_string(),
                "Upgrade geometry + UVs".to_string(),
                "Bake PBR textures with emissive inlays".to_string(),
                "Export glTF + metadata".to_string(),
            ],
            EngineeringDomain::Fabrication => vec![
                "Export STL".to_string(),
                "Slice with SuperSlicer".to_string(),
            ],
            _ => vec!["Generic research and build step".to_string()],
        };

        Ok(WorkflowPlan {
            work_item: item.clone(),
            steps,
            artifact_policy: ArtifactPolicy::PrototypeAllowed,
            estimated_time_minutes: 15,
        })
    }
}