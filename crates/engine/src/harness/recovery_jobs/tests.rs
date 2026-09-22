use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

#[tokio::test]
async fn completed_outcomes_never_replace_new_durable_revalidation() {
    for success in [false, true] {
        for next_digest in ["original", "changed"] {
            let jobs = ProviderJobs::default();
            let first = jobs
                .execute("key".into(), "original".into(), async move {
                    if success {
                        Ok(serde_json::json!("old success"))
                    } else {
                        Err(ApiError::conflict("old error"))
                    }
                })
                .await;
            assert_eq!(first.is_ok(), success);
            let calls = Arc::new(AtomicUsize::new(0));
            let called = calls.clone();
            let result = jobs
                .execute("key".into(), next_digest.into(), async move {
                    called.fetch_add(1, Ordering::SeqCst);
                    Ok(serde_json::json!("new durable decision"))
                })
                .await
                .unwrap();
            assert_eq!(result, serde_json::json!("new durable decision"));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(jobs.drain(Duration::from_secs(2)).await, 0);
        }
    }
}

#[tokio::test]
async fn subscriptions_share_owner_and_payload_drift_fails_closed() {
    let jobs = RecoveryJobs::default();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    let request = {
        let jobs = jobs.clone();
        let started = started.clone();
        let release = release.clone();
        let calls = calls.clone();
        tokio::spawn(async move {
            jobs.execute("event".into(), "digest".into(), async move {
                calls.fetch_add(1, Ordering::SeqCst);
                started.notify_one();
                release.notified().await;
                Ok(RecoveryResult::Completed)
            })
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert!(jobs
        .execute("event".into(), "drift".into(), async {
            panic!("payload drift must not start work")
        })
        .await
        .is_err());
    let mut repeat = Box::pin(jobs.execute("event".into(), "digest".into(), async {
        panic!("duplicate must subscribe to the original owner")
    }));
    assert!(futures::poll!(&mut repeat).is_pending());
    release.notify_one();
    assert!(tokio::time::timeout(Duration::from_secs(2), repeat)
        .await
        .unwrap()
        .is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(jobs.drain(Duration::from_secs(2)).await, 0);
    assert!(jobs.jobs.lock().await.is_empty());
}

#[tokio::test]
async fn shutdown_drains_cooperative_owner_and_refuses_new_work() {
    let shutdown = Shutdown::new();
    let jobs = RecoveryJobs::new(shutdown.clone());
    let started = Arc::new(Notify::new());
    let request = {
        let jobs = jobs.clone();
        let started = started.clone();
        tokio::spawn(async move {
            jobs.execute("event".into(), "digest".into(), async move {
                started.notify_one();
                shutdown.wait().await;
                Err(ApiError::stopping())
            })
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    assert_eq!(jobs.drain(Duration::from_secs(2)).await, 0);
    assert!(request.await.unwrap().is_err());
    assert!(jobs
        .execute("new".into(), "digest".into(), async {
            panic!("no work may start after drain")
        })
        .await
        .is_err());
}

#[tokio::test]
async fn expired_drain_keeps_unresolved_join_until_owner_finishes() {
    let jobs = RecoveryJobs::default();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let request = {
        let jobs = jobs.clone();
        let started = started.clone();
        let release = release.clone();
        tokio::spawn(async move {
            jobs.execute("event".into(), "digest".into(), async move {
                started.notify_one();
                release.notified().await;
                Err(ApiError::conflict("unresolved"))
            })
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    assert_eq!(jobs.drain(Duration::ZERO).await, 1);
    assert_eq!(jobs.jobs.lock().await.len(), 1);
    release.notify_one();
    assert_eq!(jobs.drain(Duration::from_secs(2)).await, 0);
    assert!(request.await.unwrap().is_err());
}
