use std::{env, path::PathBuf};

fn main() -> Result<(), String> {
    if env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }

    let includes = [protos_dir()];
    let protos = [
        proto_path("health_model.proto"),
        proto_path("route_governance.proto"),
    ];

    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/grpc");
    tonic_prost_build::configure()
        .out_dir(out_dir)
        .build_server(true)
        .build_client(true)
        .compile_protos(&protos, &includes)
        .map_err(|err| format!("tonic-prost-build failed: {err}"))?;

    Ok(())
}

fn protos_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto")
}

fn proto_path(name: &str) -> PathBuf {
    protos_dir().join(name)
}
