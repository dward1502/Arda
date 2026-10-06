use std::fs;

fn main() {
    let paths = [
        "/var/home/mythos/Eregion/Arda/core/state/contracts/wgtt_contract.json",
        "/var/home/mythos/Eregion/Arda/core/state/contracts/skylightpros_contract.json",
    ];

    for path in &paths {
        let raw = fs::read_to_string(path).expect("read contract");
        match arda_core::project_contract::ProjectContract::from_json_str(&raw) {
            Ok(contract) => {
                println!("VALID: {} (project_id: {})", contract.identity.name, contract.identity.project_id);
                println!("  root: {}", contract.workspace.root.as_str());
                println!("  commands: {}", contract.commands.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "));
                println!("  checks: {}", contract.checks.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "));
                println!("  authority: {:?}", contract.permissions.authority);
            }
            Err(e) => {
                println!("INVALID: {}: {}", path, e);
                std::process::exit(1);
            }
        }
    }
    println!("\nBoth contracts validated successfully.");
}