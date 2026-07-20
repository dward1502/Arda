use std::{env, path::PathBuf};

fn main() -> Result<(), String> {
    if env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }

    let includes = [protos_dir()];
    let protos = [proto_path("supervisor.proto")];

    prost_build::Config::new()
        .compile_protos(&protos, &includes)
        .map_err(|err| format!("prost-build failed: {err}"))?;

    tonic_build::generate_protos(&protos, &includes)
        .map_err(|err| format!("tonic-build failed: {err}"))?;

    Ok(())
}

fn protos_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto")
}

fn proto_path(name: &str) -> PathBuf {
    protos_dir().join(name)
}
