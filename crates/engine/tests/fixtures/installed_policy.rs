//! Host-qualified installed runtime policy; no ambient profile or credentials copied.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
pub fn write(root: &Path) -> PathBuf {
    let profile_dir = root.join("profile-input");
    fs::create_dir(&profile_dir).unwrap();
    let profile = profile_dir.join("config.yaml");
    fs::write(
        &profile,
        include_str!("../../../../config/retained/hermes-profile.json"),
    )
    .unwrap();
    let sessions = root.join("sessions");
    fs::create_dir(&sessions).unwrap();
    fs::set_permissions(&sessions, fs::Permissions::from_mode(0o700)).unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let source = home.join(".hermes/hermes-agent");
    let site = home.join(".local/lib/python3.12/site-packages");
    assert!(
        source.is_dir() && site.is_dir(),
        "requires the installed Hermes Python 3.12 layout"
    );
    let value = serde_json::json!({"version":1,"entrypoint":{"interpreter":"/usr/bin/python3.12","ordered_import_roots":[source,site,"/usr/lib/python3.12/site-packages","/usr/lib64/python3.12/site-packages"]},"grants":[
        {"id":"system","source":"/usr","destination":"/usr","kind":"directory","access":"read_only","role":"runtime"},
        {"id":"code","source":source,"destination":source,"kind":"directory","access":"read_only","role":"runtime"},
        {"id":"site","source":site,"destination":site,"kind":"directory","access":"read_only","role":"runtime"},
        {"id":"profile","source":profile,"destination":"/hermes/profiles/retained/config.yaml","kind":"file","access":"read_only","role":"profile_input"},
        {"id":"state","source":sessions,"destination":"/hermes/profiles/retained","kind":"directory","access":"read_write","role":"session_state"}
    ],"fixed_environment":{"HOME":"/hermes/profiles/retained","HERMES_HOME":"/hermes/profiles/retained","PATH":"/usr/bin:/bin"}});
    let policy = arda_engine::objectives::runtime_policy::ValidatedRuntimePolicy::parse(
        &serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let path = root.join("runtime-policy.json");
    fs::write(&path, serde_json::to_vec(policy.policy()).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    path
}
pub fn adapter_config(root: &Path) -> PathBuf {
    // Configured execution selects the captured bootstrap, not a host launcher.
    let executable = PathBuf::from("hermes");
    let path = root.join("installed-hermes.toml");
    fs::write(
        &path,
        format!(
            r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "1"
executable = {:?}
max_timeout_ms = 120000
cancellation_grace_ms = 1000
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 1048576
inherit_environment = []
[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
            executable.to_str().unwrap()
        ),
    )
    .unwrap();
    path
}
