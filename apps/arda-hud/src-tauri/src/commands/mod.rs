pub mod monitor_surface;
#[expect(
    dead_code,
    reason = "Personal operations commands are staged and tested but not registered with the live HUD invoke handler"
)]
pub mod personal_ops;
#[expect(
    dead_code,
    reason = "Research commands are staged and tested but not registered with the live HUD invoke handler"
)]
pub mod research;
pub mod system_health;
pub mod workbench;
