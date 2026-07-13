use crate::blender::BlenderClient;
use crate::slicer::SuperSlicer;
use crate::workflow::WorkflowPlan;

pub struct ForgeExecutor {
    blender: BlenderClient,
    slicer: SuperSlicer,
}

impl ForgeExecutor {
    pub fn new() -> Self {
        Self {
            blender: BlenderClient::new(),
            slicer: SuperSlicer::new(),
        }
    }

    pub async fn execute(&self, plan: WorkflowPlan) -> Result<()> {
        for step in plan.steps {
            if step.contains("blender") {
                self.blender.upgrade_boardroom_desk(/* path */).await?;
            } else if step.contains("slice") {
                self.slicer.slice(/* ... */).await?;
            }
        }
        Ok(())
    }
}