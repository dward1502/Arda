use super::{ClaimedLeaf, ObjectiveStore, StageReceipt};
use crate::supervisor::Shutdown;
use anyhow::{Context, Result};
use futures::{stream::FuturesUnordered, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveRuntimeStop {
    Drained,
    Interrupted,
}

/// Live in-process observation, not a persisted lease or whole-system readiness verdict.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ObjectiveRuntimeStatus {
    /// Scheduler-local verdict, not provider health or whole-system readiness.
    pub ready: bool,
    /// Last completed store scan; None means unknown, never zero.
    pub pending_recovery: Option<u64>,
    pub phase: &'static str,
    pub active_leaves: Vec<String>,
    pub next_wake_ms: Option<i64>,
    pub last_error: Option<&'static str>,
}

impl Default for ObjectiveRuntimeStatus {
    fn default() -> Self {
        Self {
            ready: false,
            pending_recovery: None,
            phase: "not_started",
            active_leaves: Vec::new(),
            next_wake_ms: None,
            last_error: None,
        }
    }
}

// A cancelled/panicking owner must not leave a live-looking projection behind.
struct StatusScope {
    status: tokio::sync::watch::Sender<ObjectiveRuntimeStatus>,
    finished: bool,
}

impl StatusScope {
    fn finish_round(
        &mut self,
        error: Option<&'static str>,
        pending: Option<u64>,
        next_wake: Option<i64>,
    ) {
        // Phase and its error classification are one observable snapshot.
        self.status.send_modify(|status| {
            let draining = status.phase == "draining";
            status.phase = if draining {
                "draining"
            } else if error.is_some() {
                "degraded"
            } else {
                "waiting"
            };
            status.pending_recovery = pending;
            status.ready = !draining && error.is_none() && pending == Some(0);
            if error.is_some() {
                status.last_error = error;
            } else if status.ready {
                status.last_error = None;
            }
            status.active_leaves.clear();
            status.next_wake_ms = if draining { None } else { next_wake };
        });
        self.finished = true;
    }

    fn finish(&mut self, phase: &'static str) {
        self.status.send_modify(|status| {
            status.phase = phase;
            status.ready = false;
            status.active_leaves.clear();
            status.next_wake_ms = None;
        });
        self.finished = true;
    }
}

impl Drop for StatusScope {
    fn drop(&mut self) {
        if !self.finished {
            self.finish("interrupted");
        }
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;

    #[test]
    fn drained_round_cannot_publish_ready_before_owner_stops() {
        let (sender, receiver) = tokio::sync::watch::channel(ObjectiveRuntimeStatus::default());
        sender.send_modify(|status| status.phase = "draining");
        let mut scope = StatusScope {
            status: sender,
            finished: false,
        };
        scope.finish_round(None, Some(0), None);
        assert!(!receiver.borrow().ready);
        assert_eq!(receiver.borrow().phase, "draining");
    }

    #[test]
    fn round_completion_publishes_coherent_error_and_clears_activity() {
        let (sender, receiver) = tokio::sync::watch::channel(ObjectiveRuntimeStatus::default());
        for failed in [true, false] {
            sender.send_modify(|status| {
                status.phase = "executing";
                status.active_leaves = vec!["leaf".into()];
                status.next_wake_ms = Some(123);
            });
            let mut scope = StatusScope {
                status: sender.clone(),
                finished: false,
            };
            scope.finish_round(failed.then_some("objective_round_failed"), Some(0), None);
            let snapshot = receiver.borrow().clone();
            assert_eq!(snapshot.phase, if failed { "degraded" } else { "waiting" });
            assert_eq!(
                snapshot.last_error,
                failed.then_some("objective_round_failed")
            );
            assert!(snapshot.active_leaves.is_empty());
            assert!(snapshot.next_wake_ms.is_none());
            drop(scope);
            assert_eq!(receiver.borrow().phase, snapshot.phase);
        }
    }
}

pub trait LeafExecution: Send + Sync {
    /// Retrieve completed evidence only; never fall back to provider execution.
    fn reconcile(
        &self,
        _claim: ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LeafExecutionResult>>> + Send>> {
        Box::pin(async { Ok(None) })
    }

    fn execute(
        &self,
        claim: ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafExecutionResult {
    pub receipts: Vec<StageReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafRoundOutcome {
    pub objective_id: String,
    pub leaf_id: String,
    pub terminal_receipt_digest: String,
}

pub struct ObjectiveRuntime<E> {
    store: ObjectiveStore,
    executor: E,
    worker_id: String,
    capacity: usize,
    lease_duration_ms: i64,
    status: tokio::sync::watch::Sender<ObjectiveRuntimeStatus>,
}

impl<E> ObjectiveRuntime<E>
where
    E: LeafExecution,
{
    pub fn new(
        store: ObjectiveStore,
        executor: E,
        worker_id: impl Into<String>,
        capacity: usize,
        lease_duration_ms: i64,
    ) -> Self {
        Self {
            store,
            executor,
            worker_id: worker_id.into(),
            capacity,
            lease_duration_ms,
            status: tokio::sync::watch::channel(ObjectiveRuntimeStatus::default()).0,
        }
    }

    pub fn store(&self) -> &ObjectiveStore {
        &self.store
    }

    pub fn subscribe_status(&self) -> tokio::sync::watch::Receiver<ObjectiveRuntimeStatus> {
        self.status.subscribe()
    }

    /// Stop claiming on shutdown, then drain the current round within a bound.
    /// An interrupted round leaves leases and receipt-backed stages untouched.
    /// The caller must separately stop/join execution services: dropping an
    /// HTTP client future does not cancel its server-side provider handler.
    pub async fn run_until_shutdown(
        &mut self,
        shutdown: Shutdown,
        poll_interval: Duration,
        drain_timeout: Duration,
    ) -> ObjectiveRuntimeStop {
        let mut scope = StatusScope {
            status: self.status.clone(),
            finished: false,
        };
        let result = self.run_loop(shutdown, poll_interval, drain_timeout).await;
        scope.finish(match result {
            ObjectiveRuntimeStop::Drained => "stopped",
            ObjectiveRuntimeStop::Interrupted => "interrupted",
        });
        result
    }

    async fn run_loop(
        &mut self,
        shutdown: Shutdown,
        poll_interval: Duration,
        drain_timeout: Duration,
    ) -> ObjectiveRuntimeStop {
        let mut changes = self.store.subscribe_changes();
        loop {
            if shutdown.is_triggered() {
                return ObjectiveRuntimeStop::Drained;
            }
            // Mark before reading SQLite: commits during the round remain
            // pending, including completion that unlocks dependent work.
            changes.borrow_and_update();
            let mut round_failed = false;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(i64::MAX);
            {
                let status = self.status.clone();
                let round = async {
                    // The signal can win select before this future was first
                    // polled. Draining that unstarted future must not claim work.
                    if shutdown.is_triggered() {
                        return Ok(Vec::new());
                    }
                    self.run_round(now).await
                };
                tokio::pin!(round);
                let result = tokio::select! {
                    biased;
                    _ = shutdown.wait() => {
                        status.send_modify(|status| {
                            status.phase = "draining";
                            status.ready = false;
                        });
                        return match tokio::time::timeout(drain_timeout, &mut round).await {
                            Ok(result) => {
                                if let Err(error) = result {
                                    tracing::warn!("resident objective drain finished with errors: {error:#}");
                                }
                                ObjectiveRuntimeStop::Drained
                            }
                            Err(_) => ObjectiveRuntimeStop::Interrupted,
                        };
                    }
                    result = &mut round => result,
                };
                if let Err(error) = result {
                    round_failed = true;
                    tracing::warn!(%error, "resident objective round failed");
                }
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let delay = if round_failed {
                poll_interval
            } else {
                // Timer and recovery checks were published atomically with
                // round completion; never advertise unchecked readiness.
                match self.status.borrow().next_wake_ms {
                    Some(wake) => poll_interval.min(Duration::from_millis(
                        wake.saturating_sub(now).max(10) as u64,
                    )),
                    None => poll_interval,
                }
            };
            tokio::select! {
                _ = shutdown.wait() => return ObjectiveRuntimeStop::Drained,
                _ = changes.changed() => {},
                _ = tokio::time::sleep(delay) => {},
            }
        }
    }

    pub async fn run_round(&mut self, now_ms: i64) -> Result<Vec<LeafRoundOutcome>> {
        let mut scope = StatusScope {
            status: self.status.clone(),
            finished: false,
        };
        self.status.send_modify(|status| {
            status.phase = "checking";
            status.ready = false;
            status.active_leaves.clear();
            status.next_wake_ms = None;
        });
        let result = self.run_round_inner(now_ms).await;
        let pending = self.store.pending_recovery();
        let next_wake = self.store.next_wake_ms(now_ms);
        let error = if result.is_err() || pending.is_err() {
            Some("objective_round_failed")
        } else if next_wake.is_err() {
            Some("timer_lookup_failed")
        } else {
            None
        };
        scope.finish_round(
            error,
            pending.as_ref().ok().copied(),
            next_wake.as_ref().ok().copied().flatten(),
        );
        result.and_then(|outcomes| {
            pending?;
            next_wake?;
            Ok(outcomes)
        })
    }

    async fn run_round_inner(&mut self, now_ms: i64) -> Result<Vec<LeafRoundOutcome>> {
        let started = std::time::Instant::now();
        let completion_time = || {
            now_ms.saturating_add(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX))
        };
        let recovery = self.store.claim_reconciliation(
            &self.worker_id,
            now_ms,
            self.lease_duration_ms,
            self.capacity,
        )?;
        // Receipt recovery is an admission barrier, not merely a priority in
        // one concurrent batch. Finish its durable projection before granting
        // fresh work a lease; the next notified round can use full capacity.
        let normal = if recovery.is_empty() {
            self.store.claim_runnable(
                &self.worker_id,
                now_ms,
                self.lease_duration_ms,
                self.capacity,
            )?
        } else {
            Vec::new()
        };
        let claims: Vec<_> = recovery
            .into_iter()
            .map(|claim| (claim, true))
            .chain(normal.into_iter().map(|claim| (claim, false)))
            .collect();
        let mut outcomes = Vec::with_capacity(claims.len());
        self.status.send_modify(|status| {
            status.phase = if claims
                .iter()
                .any(|(claim, recovery)| *recovery || claim.attempt > 1)
            {
                "recovering"
            } else {
                "executing"
            };
            status.active_leaves = claims
                .iter()
                .map(|(claim, _)| claim.leaf_id.clone())
                .collect();
        });
        let mut executions = claims
            .into_iter()
            .map(|(claim, reconciliation_only)| {
                let execution = if reconciliation_only {
                    self.executor.reconcile(claim.clone())
                } else {
                    let executor = &self.executor;
                    let retry = claim.clone();
                    let unbound = self.store.can_prepare_resident_context(&claim);
                    Box::pin(async move {
                        // Bound retries must project completed evidence before
                        // entering the provider-capable continuation path. An
                        // error is not permission to execute; explicit unbound
                        // claims retain their first-context recovery path.
                        if retry.attempt > 1 && !unbound? {
                            if let Some(result) = executor.reconcile(retry.clone()).await? {
                                return Ok(Some(result));
                            }
                        }
                        executor.execute(retry).await.map(Some)
                    })
                };
                async move { (claim, reconciliation_only, execution.await) }
            })
            .collect::<FuturesUnordered<_>>();
        let mut errors = Vec::new();
        // Persist each finished leaf before waiting for its siblings. A pending
        // provider must not keep successful work only in this round's memory.
        while let Some((claim, _reconciliation_only, result)) = executions.next().await {
            self.status
                .send_modify(|status| status.active_leaves.retain(|id| id != &claim.leaf_id));
            let result = match result.with_context(|| {
                format!(
                    "execute objective `{}` leaf `{}`",
                    claim.objective_id, claim.leaf_id
                )
            }) {
                Ok(Some(result)) => result,
                Ok(None) => {
                    self.store.fail_reconciliation(&claim, completion_time())?;
                    errors.push(format!(
                        "retry budget exhausted without completed evidence for leaf `{}`",
                        claim.leaf_id
                    ));
                    continue;
                }
                Err(error) => {
                    errors.push(format!("{error:#}"));
                    continue;
                }
            };
            if result.receipts.is_empty() {
                errors.push(format!(
                    "leaf `{}` returned no canonical receipts",
                    claim.leaf_id
                ));
                continue;
            }
            let mut receipt_error = None;
            for receipt in result.receipts {
                if let Err(error) = self.store.record_stage_receipt(
                    &claim.leaf_id,
                    &claim.lease_owner,
                    claim.attempt,
                    receipt,
                    completion_time(),
                ) {
                    receipt_error = Some(error);
                    break;
                }
            }
            if let Some(error) = receipt_error {
                errors.push(format!(
                    "record objective `{}` leaf `{}` receipt: {error:#}",
                    claim.objective_id, claim.leaf_id
                ));
                continue;
            }
            let completed = (|| -> Result<LeafRoundOutcome> {
                let leaf = self.store.leaf(&claim.leaf_id)?.ok_or_else(|| {
                    anyhow::anyhow!("claimed leaf `{}` disappeared", claim.leaf_id)
                })?;
                let terminal_receipt_digest = leaf.current_receipt_digest.ok_or_else(|| {
                    anyhow::anyhow!(
                        "completed leaf `{}` omitted terminal receipt",
                        claim.leaf_id
                    )
                })?;
                self.store.complete_objective_if_ready(
                    &claim.objective_id,
                    &terminal_receipt_digest,
                    now_ms,
                )?;
                Ok(LeafRoundOutcome {
                    objective_id: claim.objective_id,
                    leaf_id: claim.leaf_id.clone(),
                    terminal_receipt_digest,
                })
            })();
            match completed {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => errors.push(format!(
                    "finalize objective leaf `{}`: {error:#}",
                    claim.leaf_id
                )),
            }
        }
        if !errors.is_empty() {
            anyhow::bail!("objective round failed: {}", errors.join("; "));
        }
        Ok(outcomes)
    }
}
