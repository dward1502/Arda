use arda_varda::{benchmark::run_retrieval_benchmark, ingest::AthenaStore};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("arda-varda-benchmark: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let fixture_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/retrieval_benchmark_v1.json")
    });
    if args.next().is_some() {
        return Err("usage: arda-varda-benchmark [fixture.json]".into());
    }

    let benchmark_root = std::env::temp_dir().join(format!(
        "arda-varda-benchmark-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let store = AthenaStore::new_isolated(&benchmark_root)?;
    let result = run_retrieval_benchmark(&fixture_path, &store);
    let cleanup_result = std::fs::remove_dir_all(&benchmark_root);
    let report = result?;
    cleanup_result?;

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
