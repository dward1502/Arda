use arda_outpost_scout::{AuthorityClass, ObservationClassification, ObservationScope};
use arda_outpost_scout::{ResearchRequest, SearxngClient, ALLOWLISTED_PUBLIC_WEB_POLICY};
use chrono::{Duration, Utc};
use std::io::{Read, Write};
use std::net::TcpListener;

fn serve_searxng_response(body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fixture request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("read fixture request");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("GET /search?"));
        assert!(request.contains("q=rust+agent+governance"));
        assert!(request.contains("format=json"));
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn searxng_search_is_bounded_and_becomes_advisory_observation() {
    let body = r#"{
        "query":"rust agent governance",
        "results":[
            {"title":"Result one","url":"https://example.com/one","content":"First snippet","engine":"brave","score":1.0},
            {"title":"Result two","url":"https://example.com/two","content":"Second snippet","engine":"duckduckgo","score":0.8}
        ]
    }"#;
    let (base_url, server) = serve_searxng_response(body);
    let client = SearxngClient::new(base_url).expect("client");

    let report = client
        .search(&ResearchRequest {
            query: "rust agent governance".to_string(),
            limit: 50,
            source_policy: ALLOWLISTED_PUBLIC_WEB_POLICY.to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
        })
        .await
        .expect("search report");
    server.join().expect("fixture server");

    assert_eq!(report.query, "rust agent governance");
    assert_eq!(report.limit, 10);
    assert_eq!(report.source_policy, ALLOWLISTED_PUBLIC_WEB_POLICY);
    assert!(report.expires_at > Utc::now());
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.results[0].url, "https://example.com/one");

    let observation = report.into_observation("node-pi5-warden");
    assert_eq!(
        observation.scope,
        ObservationScope::Custom("internet_research".into())
    );
    assert_eq!(
        observation.classification,
        ObservationClassification::RawMeasurement
    );
    assert_eq!(observation.authority, AuthorityClass::Advisory);
    assert_eq!(observation.source, "node-pi5-warden");
    assert_eq!(
        observation.provenance.as_deref(),
        Some("searxng://rust agent governance")
    );
    assert_eq!(observation.payload["results"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn empty_research_query_is_rejected_without_network_access() {
    let client = SearxngClient::new("http://127.0.0.1:9").expect("client");
    let error = client
        .search(&ResearchRequest {
            query: "   ".to_string(),
            limit: 4,
            source_policy: ALLOWLISTED_PUBLIC_WEB_POLICY.to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
        })
        .await
        .expect_err("empty query must fail");

    assert!(error.to_string().contains("research query cannot be empty"));
}

#[tokio::test]
async fn research_results_without_valid_http_sources_are_rejected() {
    let body = r#"{
        "results":[
            {"title":"Unsourced result","url":"file:///tmp/result","content":"preview","engine":"fixture","score":1.0}
        ]
    }"#;
    let (base_url, server) = serve_searxng_response(body);
    let client = SearxngClient::new(base_url).expect("client");

    let error = client
        .search(&ResearchRequest {
            query: "rust agent governance".to_string(),
            limit: 3,
            source_policy: ALLOWLISTED_PUBLIC_WEB_POLICY.to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
        })
        .await
        .expect_err("non-HTTP(S) source URL must fail");
    server.join().expect("fixture server");

    assert!(error.to_string().contains("valid HTTP(S) source URL"));
}

#[test]
fn request_validation_rejects_unlisted_policy_and_excessive_expiry() {
    let now = Utc::now();
    let unlisted = ResearchRequest {
        query: "bounded research".to_string(),
        limit: 3,
        source_policy: "unrestricted_web".to_string(),
        expires_at: Some(now + Duration::minutes(5)),
    };
    assert!(unlisted
        .validate_at(now)
        .expect_err("unlisted policy must fail")
        .to_string()
        .contains("unsupported source policy"));

    let excessive_expiry = ResearchRequest {
        query: "bounded research".to_string(),
        limit: 3,
        source_policy: ALLOWLISTED_PUBLIC_WEB_POLICY.to_string(),
        expires_at: Some(now + Duration::hours(25)),
    };
    assert!(excessive_expiry
        .validate_at(now)
        .expect_err("excessive expiry must fail")
        .to_string()
        .contains("24-hour bound"));
}
