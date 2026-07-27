use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

const DEFAULT_GROUP_CONCURRENCY: usize = 1;
const DEFAULT_QUEUE_TIMEOUT_SECONDS: u64 = 30;

#[derive(Debug)]
struct ResourceGroupSlot {
    limit: usize,
    active: Arc<AtomicUsize>,
    queued: Arc<AtomicUsize>,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResourceGroupSnapshot {
    pub resource_group: String,
    pub active: usize,
    pub limit: usize,
    pub queued: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceLease {
    _inner: Arc<ResourceLeaseInner>,
}

#[derive(Debug)]
struct ResourceLeaseInner {
    _permit: OwnedSemaphorePermit,
    active: Arc<AtomicUsize>,
}

impl Drop for ResourceLeaseInner {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
pub struct ResourceGroupLimiter {
    default_limit: usize,
    slots: Arc<Mutex<BTreeMap<String, Arc<ResourceGroupSlot>>>>,
}

impl Default for ResourceGroupLimiter {
    fn default() -> Self {
        Self::new(configured_group_limit())
    }
}

impl ResourceGroupLimiter {
    pub fn new(default_limit: usize) -> Self {
        Self {
            default_limit: default_limit.max(1),
            slots: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub async fn acquire(
        &self,
        resource_group: &str,
        group_limit: Option<usize>,
    ) -> Result<ResourceLease, String> {
        let mut slots = self.slots.lock().await;
        let slot = slots
            .entry(resource_group.to_string())
            .or_insert_with(|| {
                let limit = group_limit.unwrap_or(self.default_limit).max(1);
                Arc::new(ResourceGroupSlot {
                    limit,
                    active: Arc::new(AtomicUsize::new(0)),
                    queued: Arc::new(AtomicUsize::new(0)),
                    semaphore: Arc::new(Semaphore::new(limit)),
                })
            })
            .clone();
        // Do not hold the catalog mutex while a request waits for capacity.
        // Snapshots and requests for unrelated groups must remain observable
        // and independently schedulable while this group is saturated.
        drop(slots);

        let timeout = Duration::from_secs(configured_queue_timeout_seconds());
        slot.queued.fetch_add(1, Ordering::SeqCst);
        let acquire_result = tokio::time::timeout(timeout, slot.semaphore.clone().acquire_owned())
            .await
            .map_err(|_| {
                format!(
                    "resource group {resource_group} remained busy for {} seconds",
                    timeout.as_secs()
                )
            });
        slot.queued.fetch_sub(1, Ordering::SeqCst);
        let permit =
            acquire_result?.map_err(|_| format!("resource group {resource_group} was closed"))?;
        slot.active.fetch_add(1, Ordering::SeqCst);
        Ok(ResourceLease {
            _inner: Arc::new(ResourceLeaseInner {
                _permit: permit,
                active: slot.active.clone(),
            }),
        })
    }

    pub async fn snapshots(&self) -> Vec<ResourceGroupSnapshot> {
        let slots = self.slots.lock().await;
        slots
            .iter()
            .map(|(resource_group, slot)| {
                let active = slot.active.load(Ordering::SeqCst);
                ResourceGroupSnapshot {
                    resource_group: resource_group.clone(),
                    active,
                    limit: slot.limit,
                    queued: slot.queued.load(Ordering::SeqCst),
                }
            })
            .collect()
    }

    pub async fn is_saturated(&self, resource_group: &str) -> bool {
        self.slots
            .lock()
            .await
            .get(resource_group)
            .is_some_and(|slot| slot.semaphore.available_permits() == 0)
    }
}

impl ResourceLease {
    #[cfg(test)]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self._inner)
    }
}

fn configured_group_limit() -> usize {
    std::env::var("ARDA_MANWE_RESOURCE_GROUP_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_GROUP_CONCURRENCY)
}

fn configured_queue_timeout_seconds() -> u64 {
    std::env::var("ARDA_MANWE_RESOURCE_GROUP_QUEUE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_QUEUE_TIMEOUT_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn group_defaults_are_bounded() {
        let limiter = ResourceGroupLimiter::default();
        assert_eq!(limiter.snapshots().await.len(), 0);
        let first = limiter.acquire("default", None).await.expect("first lease");
        assert!(first.strong_count() >= 1);
    }

    #[tokio::test]
    async fn configured_group_limit_allows_bounded_parallel_leases() {
        let limiter = ResourceGroupLimiter::new(1);
        let first = limiter
            .acquire("shared-gpu", Some(2))
            .await
            .expect("first lease");
        let second = limiter
            .acquire("shared-gpu", Some(2))
            .await
            .expect("second lease");
        let limiter_for_waiter = limiter.clone();
        let waiter =
            tokio::spawn(async move { limiter_for_waiter.acquire("shared-gpu", Some(2)).await });

        tokio::task::yield_now().await;
        let snapshots = limiter.snapshots().await;
        assert_eq!(snapshots[0].active, 2);
        assert_eq!(snapshots[0].limit, 2);
        assert_eq!(snapshots[0].queued, 1);
        assert!(!waiter.is_finished());

        drop(first);
        let third = waiter.await.expect("waiter task").expect("third lease");
        assert_eq!(third.strong_count(), 1);
        drop(second);
    }

    #[tokio::test]
    async fn different_resource_groups_run_independently() {
        let limiter = ResourceGroupLimiter::new(1);
        let first = limiter.acquire("gpu-a", None).await.expect("gpu-a lease");
        let second = limiter.acquire("gpu-b", None).await.expect("gpu-b lease");
        assert_eq!(first.strong_count(), 1);
        assert_eq!(second.strong_count(), 1);
    }

    #[tokio::test]
    async fn saturation_is_visible_without_blocking_on_the_group() {
        let limiter = ResourceGroupLimiter::new(1);
        let lease = limiter.acquire("gpu-a", None).await.expect("gpu-a lease");
        assert!(limiter.is_saturated("gpu-a").await);
        assert!(!limiter.is_saturated("gpu-b").await);
        drop(lease);
        assert!(!limiter.is_saturated("gpu-a").await);
    }
}
