#[cfg(test)]
mod canary {
    #[test]
    fn transport_surfaces_load() {
        let _ = arda_transport::DaemonConfig::default();
        let _ = arda_transport::TransportError {
            agent: "athena-transport",
            message: "verify the extracted surfaces compile".to_string(),
        };
        let _ = arda_transport::expand_home("~/data/athena/athena.sock");
    }
}
