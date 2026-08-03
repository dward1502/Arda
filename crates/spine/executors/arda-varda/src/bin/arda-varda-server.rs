use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("ARDA_ATHENA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/athena"));
    let bind = std::env::var("ARDA_VARDA_BIND").unwrap_or_else(|_| "127.0.0.1:5111".to_owned());
    let store = arda_varda::ingest::AthenaStore::new(root)?;
    arda_varda::transport::http::run_http_server(store, &bind).await?;
    Ok(())
}
