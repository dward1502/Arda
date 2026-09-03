use arda_engine::objectives::{ClaimedLeaf, LeafExecution, WorkbenchLeafExecution};

#[tokio::main]
async fn main() {
    let path = std::env::args().nth(1).expect("claim path");
    let raw = std::fs::read(path).expect("read claim");
    let claim: ClaimedLeaf = serde_json::from_slice(&raw).expect("decode claim");
    match WorkbenchLeafExecution::new(std::env::current_dir().expect("cwd"))
        .expect("adapter")
        .execute(claim)
        .await
    {
        Ok(result) => println!("ok receipts={}", result.receipts.len()),
        Err(error) => println!("error={error:#}"),
    }
}
