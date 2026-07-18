use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: crate::expand_home("data/athena/athena.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5111),
        }
    }
}
