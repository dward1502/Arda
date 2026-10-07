//! HTTP observation deadlines must not preempt the admitted provider deadline.
use super::*;

pub(super) async fn send(
    client: &reqwest::Client,
    url: String,
    body: &Value,
    run: &Value,
    stage: &str,
) -> Result<reqwest::Response> {
    // Legacy projections can omit timeout_ms; their graph default is 900s.
    // This changes only the response wait, never node/grant authority or retries.
    let timeout_ms = run["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == stage))
        .and_then(|node| node["timeout_ms"].as_u64())
        .unwrap_or(900_000);
    let timeout = std::time::Duration::from_millis(timeout_ms)
        .checked_add(std::time::Duration::from_secs(60))
        .ok_or_else(|| anyhow!("provider response timeout overflow"))?;
    client
        .post(url)
        .timeout(timeout)
        .json(body)
        .send()
        .await
        .with_context(|| format!("execute explicit Workbench {stage} stage"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn provider_wait_outlives_short_metadata_client_timeout() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            assert!(stream.read(&mut buffer).await.unwrap() > 0);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await;
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(20))
            .build()
            .unwrap();
        let run = json!({"graph":{"nodes":[{"id":"execute","timeout_ms":1000}]}});
        let result = send(&client, url, &json!({}), &run, "execute").await;
        server.await.unwrap();
        assert!(result.unwrap().status().is_success());
    }
}
