use super::*;

#[tokio::test]
async fn errors_without_attestation_preserve_channel_but_not_success_authority() {
    for status in [429, 503] {
        for valid_success in [true, false] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!(
                "http://{}/v1/chat/completions",
                listener.local_addr().unwrap()
            );
            let (host, payload) = UnixStream::pair().unwrap();
            host.set_nonblocking(true).unwrap();
            payload.set_nonblocking(true).unwrap();
            let mut payload = tokio::net::UnixStream::from_std(payload).unwrap();
            let policy = Policy {
                model: "fixture".into(),
                provider: "edge_fixture".into(),
                routing: json!({"local_only":true}),
            };
            let task = tokio::spawn(async move { serve_at(host, policy, &endpoint).await });
            let request = serde_json::to_vec(&json!({"model":"fixture","messages":[]})).unwrap();
            for step in 0..2 {
                payload.write_u32(request.len() as u32).await.unwrap();
                payload.write_all(&request).await.unwrap();
                let (mut upstream, _) =
                    tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    headers.push(upstream.read_u8().await.unwrap());
                }
                let headers = String::from_utf8(headers).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; length];
                upstream.read_exact(&mut body).await.unwrap();
                let response = if step == 0 {
                    format!("HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: 14\r\nX-Upstream-Private: canary\r\n\r\nprivate-canary")
                } else {
                    let provider = if valid_success {
                        "edge_fixture"
                    } else {
                        "wrong"
                    };
                    format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\nx-manwe-route-id: fixture\r\nx-manwe-provider-id: {provider}\r\nx-manwe-model-id: fixture\r\nx-manwe-route-class: tool_oriented\r\n\r\n{{}}")
                };
                upstream.write_all(response.as_bytes()).await.unwrap();
                drop(upstream);
                if step == 1 && !valid_success {
                    assert!(tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        payload.read_u32()
                    )
                    .await
                    .unwrap()
                    .is_err());
                    break;
                }
                let n = tokio::time::timeout(std::time::Duration::from_secs(2), payload.read_u32())
                    .await
                    .unwrap()
                    .unwrap();
                let mut bytes = vec![0; n as usize];
                payload.read_exact(&mut bytes).await.unwrap();
                let result: Value = serde_json::from_slice(&bytes).unwrap();
                if step == 0 {
                    assert_eq!(
                        result,
                        json!({"status":status,"headers":{"content-type":"application/json"},"body":json!({"error":{"message":format!("Local Manwe rejected inference (HTTP {status})"),"type":"upstream_error","code":status}}).to_string()})
                    );
                } else {
                    assert_eq!(result["status"], 200);
                    assert_eq!(result["headers"]["x-manwe-provider-id"], "edge_fixture");
                }
            }
            drop(payload);
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), task)
                .await
                .unwrap()
                .unwrap();
            if valid_success {
                assert_eq!(
                    result
                        .unwrap_err()
                        .downcast_ref::<std::io::Error>()
                        .unwrap()
                        .kind(),
                    std::io::ErrorKind::UnexpectedEof
                );
            } else {
                assert_eq!(
                    result.unwrap_err().to_string(),
                    "unapproved inference route"
                );
            }
        }
    }
}
