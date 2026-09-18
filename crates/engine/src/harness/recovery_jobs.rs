//! Request-independent retained recovery ownership. This is not durable authority:
//! completed jobs are reconciled through the canonical admission and run stores.
use super::{projects::ApiError, runs::recovery::RecoveryResult};
use crate::supervisor::Shutdown;
use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};
use tokio::sync::{watch, Mutex};

type Outcome = Result<RecoveryResult, ApiError>;

#[cfg(test)]
mod shutdown_tests;
#[cfg(test)]
mod tests;

struct Job {
    digest: String,
    result: watch::Receiver<Option<Outcome>>,
    join: tokio::task::JoinHandle<()>,
}

/// Shared by all authenticated recovery requests served by one Harness instance.
#[derive(Clone, Default)]
pub struct RecoveryJobs {
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    shutdown: Shutdown,
}

impl RecoveryJobs {
    #[cfg(test)]
    pub(super) async fn settle(&self) -> Vec<Outcome> {
        let mut jobs = self.jobs.lock().await;
        let mut outcomes = Vec::new();
        for (_, job) in jobs.drain() {
            job.join.await.expect("recovery owner panicked");
            outcomes.push(job.result.borrow().clone().expect("owner result"));
        }
        outcomes
    }

    pub(super) fn new(shutdown: Shutdown) -> Self {
        Self {
            shutdown,
            ..Self::default()
        }
    }

    pub(super) fn shutdown(&self) -> Shutdown {
        self.shutdown.clone()
    }

    pub(super) async fn execute<F>(&self, key: String, digest: String, work: F) -> Outcome
    where
        F: Future<Output = Outcome> + Send + 'static,
    {
        let mut jobs = self.jobs.lock().await;
        if self.shutdown.is_triggered() {
            return Err(ApiError::stopping());
        }
        if let Some(job) = jobs.get(&key) {
            if job.digest != digest {
                return Err(ApiError::conflict("recovery event payload changed"));
            }
        }
        // Reap completed owners; their next caller must consult durable state,
        // not a cached success that might outlive cancellation or supersession.
        let finished: Vec<_> = jobs
            .iter()
            .filter(|(_, job)| job.join.is_finished())
            .map(|(key, _)| key.clone())
            .collect();
        for key in finished {
            let job = jobs.remove(&key).expect("locked registry entry");
            if let Err(error) = job.join.await {
                tracing::warn!(%error, "retained recovery owner failed; reconcile durable evidence");
            }
        }
        let mut result = if let Some(job) = jobs.get(&key) {
            job.result.clone()
        } else {
            let (sender, result) = watch::channel(None);
            // Keeper calls are synchronous and bounded. Retain this join even
            // when every HTTP/direct response subscription disappears.
            let runtime = tokio::runtime::Handle::current();
            let join = tokio::task::spawn_blocking(move || {
                sender.send_replace(Some(runtime.block_on(work)));
            });
            jobs.insert(
                key,
                Job {
                    digest,
                    result: result.clone(),
                    join,
                },
            );
            result
        };
        drop(jobs);
        loop {
            if let Some(outcome) = result.borrow_and_update().clone() {
                return outcome;
            }
            let changed = tokio::select! {
                _ = self.shutdown.wait() => return Err(ApiError::stopping()),
                changed = result.changed() => changed,
            };
            if changed.is_err() {
                return Err(ApiError::internal(
                    "retained recovery owner lost; outcome unresolved, reconcile original run",
                ));
            }
        }
    }

    /// Stop admission and await cooperative owners within one shared deadline.
    /// Unfinished joins stay retained, not aborted or reported as successful.
    pub(super) async fn drain(&self, budget: Duration) -> usize {
        self.shutdown.trigger();
        let deadline = tokio::time::Instant::now() + budget;
        let mut jobs = self.jobs.lock().await;
        let keys: Vec<_> = jobs.keys().cloned().collect();
        for key in keys {
            let job = jobs.get_mut(&key).expect("locked registry entry");
            match tokio::time::timeout_at(deadline, &mut job.join).await {
                Ok(result) => {
                    if let Err(error) = result {
                        tracing::warn!(%error, "recovery owner stopped without a response; reconcile evidence");
                    }
                    jobs.remove(&key);
                }
                Err(_) => {
                    tracing::error!(
                        "recovery drain deadline expired; retained work remains unresolved"
                    );
                    break;
                }
            }
        }
        jobs.len()
    }
}
