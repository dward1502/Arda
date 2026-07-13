use tokio::process::Command;

pub struct SuperSlicer {
    executable: String,
}

impl SuperSlicer {
    pub fn new() -> Self {
        Self { executable: "superslicer".to_string() }
    }

    pub async fn slice(&self, stl_path: &str, profile: &str, output_gcode: &str) -> Result<String> {
        let output = Command::new(&self.executable)
            .arg("--load")
            .arg(profile)
            .arg("--export-gcode")
            .arg(stl_path)
            .arg("-o")
            .arg(output_gcode)
            .output()
            .await?;

        if output.status.success() {
            Ok(output_gcode.to_string())
        } else {
            Err("Slicing failed".into())
        }
    }
}