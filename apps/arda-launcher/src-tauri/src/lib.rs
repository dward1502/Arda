//! Arda launcher backend.
//!
//! Thin Tauri + app harness surface over the Arda spine.
//!
//! The onboarding flow is implemented under `onboarding/`, and exposed to
//! the frontend via Tauri commands below.

pub mod onboarding;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
