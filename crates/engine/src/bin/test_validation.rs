
use std::path::Path;

fn main() {
    let home = std::env::var("HOME").unwrap();
    let home_path = Path::new(&home);
    
    let workspace = Path::new("/var/home/mythos/Eregion/Arda");
    let durable = Path::new("/var/home/mythos/.local/share/arda/retained/keeper");
    let runtime = Path::new("/run/user/1000/arda-snapshot-keeper");
    let allocation_base = Path::new("/var/home/mythos/.local/share/arda/retained/sessions");
    
    eprintln!("HOME={}", home);
    eprintln!("workspace under HOME: {}", workspace.starts_with(home_path));
    eprintln!("durable under HOME: {}", durable.starts_with(home_path));
    eprintln!("runtime under HOME: {}", runtime.starts_with(home_path));
    eprintln!("allocation_base under HOME: {}", allocation_base.starts_with(home_path));
    
    // Test each validation call
    let r1 = arda_engine::objectives::validate_snapshot_owner_paths(
        Path::new("/usr"),
        &[&durable, &runtime],
    );
    eprintln!("VAL1 (/usr vs durable,runtime): {:?}", r1);
    
    let r2 = arda_engine::objectives::validate_snapshot_owner_paths(
        workspace,
        &[&durable, &runtime],
    );
    eprintln!("VAL2 (workspace vs durable,runtime): {:?}", r2);
    
    let r3 = arda_engine::objectives::validate_snapshot_owner_paths(
        workspace,
        &[allocation_base],
    );
    eprintln!("VAL3 (workspace vs allocation_base): {:?}", r3);
    
    // Test each grant source
    let grants = [
        ("system", Path::new("/usr")),
        ("hermes", Path::new("/var/home/mythos/.hermes/hermes-agent")),
        ("user_site", Path::new("/var/home/mythos/.local/lib/python3.12/site-packages")),
        ("profile", Path::new("/var/home/mythos/.config/arda/retained/config.yaml")),
    ];
    
    for (name, source) in &grants {
        let r4 = arda_engine::objectives::validate_snapshot_owner_paths(source, &[allocation_base]);
        eprintln!("VAL4A ({} vs allocation_base): {:?}", name, r4);
        let r5 = arda_engine::objectives::validate_snapshot_owner_paths(source, &[&durable, &runtime]);
        eprintln!("VAL4B ({} vs durable,runtime): {:?}", name, r5);
    }
}
