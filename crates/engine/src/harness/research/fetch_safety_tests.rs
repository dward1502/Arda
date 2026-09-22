use super::*;
use tokio::io::AsyncWriteExt;

#[test]
fn mapped_private_addresses_are_not_public_targets() {
    for address in [
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "100.64.0.1",
        "0.1.2.3",
        "198.18.0.1",
    ] {
        assert!(is_private_ip(address.parse().unwrap()), "{address}");
    }
}

#[tokio::test]
async fn streamed_body_rejects_limit_before_response_finishes() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nabcdef\r\n")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://{address}"))
        .send()
        .await
        .unwrap();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(1), bounded_body(response, 4)).await;
    server.abort();
    let _ = server.await;
    assert!(result
        .unwrap()
        .unwrap_err()
        .contains("source exceeds 4 bytes"));
}

#[tokio::test]
async fn https_policy_refuses_http_before_resolution() {
    let error = canonical_fetch(
        &reqwest::Url::parse("http://example.invalid/").unwrap(),
        std::time::Duration::from_secs(1),
        &ResearchBetaPolicy::default(),
        true,
    )
    .await
    .unwrap_err();
    assert!(error.contains("question source policy requires HTTPS"));
}
