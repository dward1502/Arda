use tokio::process::Command;
use std::path::Path;

pub struct BlenderClient {
    blender_path: String,
    python_script_path: String,
}

impl BlenderClient {
    pub fn new() -> Self {
        Self {
            blender_path: "/usr/bin/blender".to_string(), // configurable
            python_script_path: "scripts/blender_tasks.py".to_string(),
        }
    }

    pub async fn execute_task(&self, task_script: &str, args: &[String]) -> Result<String> {
        let output = Command::new(&self.blender_path)
            .arg("--background")
            .arg("--python-expr")
            .arg(task_script)
            .args(args)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!("Blender failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    // High-level helpers
    pub async fn upgrade_boardroom_desk(&self, blend_file: &Path) -> Result<()> {
        // Will call Python operator or direct script
        println!("🔨 Upgrading boardroom desk → obsidian with glowing inlays");
        // ... implementation via tasks module
        Ok(())
    }
}

pub async fn upgrade_boardroom_desk(&self, input_blend: Option<&str>) -> anyhow::Result<()> {
    let mut cmd = tokio::process::Command::new(&self.blender_path);
    
    cmd.arg("--background")
       .arg("--python")
       .arg("scripts/blender_tasks.py")
       .arg("upgrade_boardroom_desk");

    if let Some(blend) = input_blend {
        cmd.arg("--");
        cmd.arg(blend);
    }

    let output = cmd.output().await?;
    
    if output.status.success() {
        println!("✅ Forge-Mind: Boardroom desk upgraded successfully!");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Blender task failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}