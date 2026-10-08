use arda_varda::ingest::AthenaStore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Local HTTP fixtures qualify the supported client/activity boundary, not an
// installed external crawler. Sequential cases respect production crawl limits.
#[tokio::test]
async fn degraded_crawler_responses_never_report_completed_research() {
    for (status, body, returns_result) in [
        (
            "503 Service Unavailable",
            r#"{"error":"unavailable"}"#,
            false,
        ),
        ("200 OK", "not json", false),
        ("200 OK", r#"{"success":true}"#, false),
        (
            "200 OK",
            r#"{"success":false,"markdown":"partial untrusted text"}"#,
            true,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let store = AthenaStore::new_isolated(root.path()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                assert!(headers.len() < 8192);
                headers.push(socket.read_u8().await.unwrap());
            }
            let length = String::from_utf8(headers)
                .unwrap()
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            assert!(length < 8192);
            socket.read_exact(&mut vec![0; length]).await.unwrap();
            let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            store.crawl4ai_fetch_markdown(
                &format!("http://{address}"),
                "https://example.com/research",
                "raw",
                None,
            ),
        )
        .await
        .unwrap();
        match tokio::time::timeout(std::time::Duration::from_secs(5), &mut server).await {
            Ok(joined) => joined.unwrap(),
            Err(_) => {
                server.abort();
                let _ = server.await;
                panic!("fixture server did not complete within five seconds");
            }
        }
        if returns_result {
            assert!(!response.unwrap().success);
        } else {
            assert!(response.is_err(), "{status}: {body}");
        }
        let activity = store.status().unwrap();
        assert_eq!(activity.active_crawls_total, 0);
        assert!(
            activity.last_activity_error.is_some(),
            "unsuccessful response must retain an activity error: {body}"
        );
        assert!(store
            .query("partial untrusted", 3)
            .unwrap()
            .matches
            .is_empty());
    }
}
