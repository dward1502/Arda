// Arda HUD — operator dashboard for the live `manwe` gateway.
//
// The HUD is a thin Tauri shell. Its value is the React surface in `../src`,
// which talks to the gateway over HTTP (OpenAI-compatible `manwe` on :7171).
// Rust-side we expose a couple of read-only commands so the frontend can ask
// the native layer about the local gateway without hardcoding host logic in JS.

#[tauri::command]
fn gateway_base_url() -> String {
    // Reserved workspace-wide per the frozen refactor contract. `manwe` owns
    // 7171; nothing else may claim it.
    "http://127.0.0.1:7171".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![gateway_base_url])
        .run(tauri::generate_context!())
        .expect("error while running arda-hud");
}
