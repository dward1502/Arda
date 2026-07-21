use anyhow::Result;

fn main() {
    if let Err(error) = run() {
        eprintln!("ptyspawn failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let spec = shell_escape::PopenSpec::new("/bin/bash");
    let child = spec
        .spawn()
        .map_err(|error| anyhow::anyhow!("failed to spawn shell: {error}"))?;
    let pid = child.pid();
    let stdin = child.stdin();
    let stdout = child.stdout();
    let stderr = child.stderr();

    let payload = serde_json::json!({
        "pid": pid,
        "has_stdin": stdin.is_some(),
        "has_stdout": stdout.is_some(),
        "has_stderr": stderr.is_some(),
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
