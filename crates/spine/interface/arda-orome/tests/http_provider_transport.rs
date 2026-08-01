#![cfg(feature = "service-runtime")]

use arda_orome::provider::{
    EdgeCommunicationPolicy, FleetScope, HttpJsonTransport, ProviderConfig, ProviderRuntime,
    ProviderType, RoutingIntent, TransportRequest,
};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn receipt_server(response_body: &'static str) -> (String, tokio::task::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0_u8; 16 * 1024];
        let mut used = 0;
        loop {
            let read = stream.read(&mut buffer[used..]).await.unwrap();
            assert!(read > 0, "request ended before a JSON body arrived");
            used += read;
            let request = &buffer[..used];
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if used < header_end + content_length {
                continue;
            }
            let body: Value =
                serde_json::from_slice(&request[header_end..header_end + content_length]).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            return body;
        }
    });
    (format!("http://{address}/messages"), task)
}

fn provider(endpoint: String) -> ProviderConfig {
    ProviderConfig {
        id: "warden".to_string(),
        kind: ProviderType::Http,
        name: "Warden".to_string(),
        endpoint,
        capabilities: vec!["messages".to_string()],
    }
}

#[tokio::test]
async fn trusted_fleet_http_dispatch_requires_allowlist_and_returns_delivery_receipt() {
    let (endpoint, captured_request) =
        receipt_server(r#"{"message_id":"warden-receipt-7","chunks":["accepted"]}"#).await;
    let runtime =
        ProviderRuntime::new(vec![provider(endpoint)]).with_edge_policy(EdgeCommunicationPolicy {
            allowed_scopes: vec![FleetScope::Local, FleetScope::TrustedFleet],
            trusted_fleet_provider_ids: vec!["warden".to_string()],
            require_external_approval: true,
        });

    let result = runtime
        .dispatch(
            RoutingIntent::direct("warden"),
            TransportRequest::new("arda-message-1", "observe").for_scope(FleetScope::TrustedFleet),
            &HttpJsonTransport::default(),
        )
        .await;

    assert!(result.succeeded());
    assert!(result.receipts[0].delivery_proven());
    assert_eq!(
        result.receipts[0].provider_message_id.as_deref(),
        Some("warden-receipt-7")
    );
    let request = captured_request.await.unwrap();
    assert_eq!(request["message_id"], "arda-message-1");
    assert_eq!(request["fleet_scope"], "trusted_fleet");
    assert_eq!(request["payload"], "observe");
}

#[tokio::test]
async fn trusted_fleet_target_is_blocked_before_network_without_explicit_allowlist() {
    let runtime = ProviderRuntime::new(vec![provider("http://127.0.0.1:9/messages".to_string())])
        .with_edge_policy(EdgeCommunicationPolicy {
            allowed_scopes: vec![FleetScope::Local, FleetScope::TrustedFleet],
            trusted_fleet_provider_ids: Vec::new(),
            require_external_approval: true,
        });

    let result = runtime
        .dispatch(
            RoutingIntent::direct("warden"),
            TransportRequest::new("blocked", "observe").for_scope(FleetScope::TrustedFleet),
            &HttpJsonTransport::default(),
        )
        .await;

    assert_eq!(
        result.error.as_deref(),
        Some("trusted_fleet_target_not_allowed")
    );
    assert!(result.receipts.is_empty());
}

#[tokio::test]
async fn http_success_without_provider_message_id_is_not_delivery_proof() {
    let (endpoint, captured_request) = receipt_server(r#"{"chunks":[]}"#).await;
    let runtime = ProviderRuntime::new(vec![provider(endpoint)]);

    let result = runtime
        .dispatch(
            RoutingIntent::direct("warden"),
            TransportRequest::new("missing-receipt", "observe"),
            &HttpJsonTransport::default(),
        )
        .await;

    assert!(!result.succeeded());
    assert!(!result.receipts[0].delivery_proven());
    assert!(result.receipts[0]
        .error
        .as_deref()
        .is_some_and(|error| error.contains("missing_provider_receipt")));
    captured_request.await.unwrap();
}
