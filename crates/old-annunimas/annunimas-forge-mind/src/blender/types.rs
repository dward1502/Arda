#[derive(Debug, Clone)]
pub enum BlenderTask {
    UpgradeBoardroomDesk { input_blend: String, output_dir: String },
    GeneratePbrTextures { object_name: String, style: String },
    ExportGlb { output_path: String },
}