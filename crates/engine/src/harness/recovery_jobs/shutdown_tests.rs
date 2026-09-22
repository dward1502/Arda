use super::*;
use crate::harness::HarnessShutdownReport;
use axum::{routing::get, Router};
use tokio::{io::AsyncWriteExt, sync::Notify};

#[tokio::test]
async fn provider_only_shutdown_retains_disconnected_owner_and_child_until_settled() {
    let jobs = ProviderJobs::default();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, barrier) = tokio::sync::oneshot::channel::<()>();
    let subscriber = {
        let jobs = jobs.clone();
        tokio::spawn(async move {
            jobs.execute("provider".into(), "digest".into(), async move {
                let mut child = tokio::process::Command::new("/bin/sleep")
                    .arg("30")
                    .kill_on_drop(true)
                    .spawn()
                    .unwrap();
                started.send(child.id().unwrap()).unwrap();
                let _ = barrier.await;
                child.kill().await.unwrap();
                child.wait().await.unwrap();
                Err(ApiError::stopping())
            })
            .await
        })
    };
    let pid = tokio::time::timeout(Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    subscriber.abort();
    assert!(subscriber.await.unwrap_err().is_cancelled());
    let mut report = HarnessShutdownReport::collect(
        RecoveryJobs::default(),
        jobs,
        tokio::spawn(async {}),
        Duration::from_millis(30),
    )
    .await;
    assert_eq!(report.unresolved_recoveries, 0);
    assert_eq!(report.unresolved_providers, 1);
    assert!(!report.settle(Duration::from_millis(30)).await);
    assert!(std::path::Path::new(&format!("/proc/{pid}")).exists());
    assert_eq!(report.provider_jobs.jobs.lock().await.len(), 1);
    release.send(()).unwrap();
    assert!(report.settle(Duration::from_secs(2)).await);
    assert_eq!(report.unresolved_providers, 0);
    assert!(report.provider_jobs.jobs.lock().await.is_empty());
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

#[tokio::test]
async fn shutdown_report_retains_owner_after_server_lifetime_with_and_without_subscriber() {
    for connected in [true, false] {
        let shutdown = Shutdown::new();
        let jobs = RecoveryJobs::new(shutdown.clone());
        let started = Arc::new(Notify::new());
        // Sender drop releases the work even if a regression assertion panics.
        let (release, barrier) = tokio::sync::watch::channel(false);
        let app = Router::new().route(
            "/",
            get({
                let jobs = jobs.clone();
                let started = started.clone();
                move || {
                    let jobs = jobs.clone();
                    let started = started.clone();
                    let mut barrier = barrier.clone();
                    async move {
                        let result = jobs
                            .execute("event".into(), "digest".into(), async move {
                                started.notify_one();
                                let _ = barrier.changed().await;
                                Err(ApiError::stopping())
                            })
                            .await;
                        assert!(result.is_err());
                        axum::http::StatusCode::SERVICE_UNAVAILABLE
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = shutdown.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move { stop.wait().await })
                .await
                .unwrap();
        });
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), started.notified())
            .await
            .unwrap();
        let mut client = Some(client);
        if !connected {
            drop(client.take());
        }
        // Move, do not clone: after this task returns, only the report retains
        // the registry. This models the production server-owner handoff.
        let owner = tokio::spawn(async move {
            HarnessShutdownReport::collect(
                jobs,
                ProviderJobs::default(),
                server,
                Duration::from_millis(30),
            )
            .await
        });
        let mut report = tokio::time::timeout(Duration::from_secs(2), owner)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(report.unresolved_recoveries, 1);
        if let Some(mut client) = client {
            use tokio::io::AsyncReadExt;
            let mut response = String::new();
            tokio::time::timeout(Duration::from_secs(2), client.read_to_string(&mut response))
                .await
                .unwrap()
                .unwrap();
            assert!(response.contains("503 Service Unavailable"));
        }
        assert!(!report.settle(Duration::from_millis(30)).await);
        assert!(
            report.server.is_none(),
            "HTTP server must finish without waiting for execution owner"
        );
        {
            let jobs = report.recovery_jobs.jobs.lock().await;
            assert_eq!(jobs.len(), 1);
            assert!(!jobs["event"].join.is_finished());
        }
        release.send_replace(true);
        assert!(report.settle(Duration::from_secs(2)).await);
        assert_eq!(report.unresolved_recoveries, 0);
        assert!(report.recovery_jobs.jobs.lock().await.is_empty());
    }
}

#[tokio::test]
async fn shutdown_report_bounds_server_wait_and_retains_server_join() {
    let (release, barrier) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let _ = barrier.await;
    });
    let mut report = tokio::time::timeout(
        Duration::from_secs(2),
        HarnessShutdownReport::collect(
            RecoveryJobs::default(),
            ProviderJobs::default(),
            server,
            Duration::from_millis(30),
        ),
    )
    .await
    .unwrap();
    assert_eq!(report.unresolved_recoveries, 0);
    assert!(!report.server.as_ref().unwrap().is_finished());
    release.send(()).unwrap();
    assert!(report.settle(Duration::from_secs(2)).await);
}
