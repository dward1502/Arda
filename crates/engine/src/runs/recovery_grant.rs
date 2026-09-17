//! Immutable recovery-window data. Authentication and canonical binding checks
//! belong to the operator admission path; this data alone is not a credential.
use arda_core::run_graph::{NodeId, RunId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RECOVERY_WINDOW_MS: u64 = 30 * 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryBindings {
    pub objective_id: String,
    pub objective_revision: i64,
    pub stop_generation: u64,
    pub leaf_id: String,
    pub run_id: RunId,
    pub failed_event_sequence: u64,
    pub failed_event_digest: String,
    pub execute_receipt_digest: String,
    pub project_contract_digest: String,
    pub context_capsule_digest: String,
    pub context_use_receipt_digest: String,
    pub retained_authority_digest: String,
    pub verify_node_id: NodeId,
    pub review_node_id: NodeId,
    pub close_node_id: NodeId,
    /// Original durable starts, not objective lease claims.
    pub prior_verify_starts: u64,
    /// Absolute ceilings: Verify original + one; Review original unchanged.
    pub provider_start_ceilings: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryGrant {
    pub authenticated_event_id: String,
    pub authenticated_payload_digest: String,
    pub bindings: RecoveryBindings,
    pub activated_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

impl RecoveryGrant {
    pub fn validate(&self) -> Result<(), &'static str> {
        let b = &self.bindings;
        let expected_verify_ceiling = b
            .prior_verify_starts
            .checked_add(1)
            .ok_or("recovery verifier ceiling overflow")?;
        if self.authenticated_event_id.trim().is_empty()
            || b.objective_id.trim().is_empty()
            || b.objective_revision <= 0
            || b.stop_generation > i64::MAX as u64
            || b.leaf_id.trim().is_empty()
            || b.failed_event_sequence == 0
            || b.prior_verify_starts == 0
            || b.verify_node_id == b.review_node_id
            || b.verify_node_id == b.close_node_id
            || b.review_node_id == b.close_node_id
            || b.provider_start_ceilings.len() != 2
            || b.provider_start_ceilings
                .get(b.verify_node_id.as_str())
                .copied()
                != Some(expected_verify_ceiling)
            || b.provider_start_ceilings
                .get(b.review_node_id.as_str())
                .copied()
                .unwrap_or(0)
                == 0
            || self
                .expires_at_unix_ms
                .checked_sub(self.activated_at_unix_ms)
                != Some(RECOVERY_WINDOW_MS)
        {
            return Err("invalid bounded recovery grant");
        }
        for digest in [
            &self.authenticated_payload_digest,
            &b.failed_event_digest,
            &b.execute_receipt_digest,
            &b.project_contract_digest,
            &b.context_capsule_digest,
            &b.context_use_receipt_digest,
            &b.retained_authority_digest,
        ] {
            let Some(hex) = digest.strip_prefix("sha256:") else {
                return Err("recovery binding lacks a SHA-256 digest");
            };
            if hex.len() != 64
                || !hex
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err("recovery binding has an invalid SHA-256 digest");
            }
        }
        Ok(())
    }

    /// A caller must first compare all bindings against current canonical state.
    /// Replay never supplies a fresh activation time or a new absolute ceiling.
    pub fn permits_start(&self, node: &NodeId, next_start: u64, now: u64) -> bool {
        self.is_active(now)
            && self
                .bindings
                .provider_start_ceilings
                .get(node.as_str())
                .is_some_and(|limit| {
                    next_start > 0
                        && next_start <= *limit
                        && (node != &self.bindings.verify_node_id
                            || self.bindings.prior_verify_starts.checked_add(1) == Some(next_start))
                })
    }

    pub fn is_active(&self, now: u64) -> bool {
        self.validate().is_ok() && now >= self.activated_at_unix_ms && now < self.expires_at_unix_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant() -> RecoveryGrant {
        let verify = NodeId::new("verify").unwrap();
        let review = NodeId::new("review").unwrap();
        let digest = format!("sha256:{}", "a".repeat(64));
        RecoveryGrant {
            authenticated_event_id: "authenticated-test-event".into(),
            authenticated_payload_digest: digest.clone(),
            activated_at_unix_ms: 100,
            expires_at_unix_ms: 100 + RECOVERY_WINDOW_MS,
            bindings: RecoveryBindings {
                objective_revision: 1,
                stop_generation: 1,
                objective_id: "objective".into(),
                leaf_id: "leaf".into(),
                run_id: RunId::new("run").unwrap(),
                failed_event_sequence: 17,
                failed_event_digest: digest.clone(),
                execute_receipt_digest: digest.clone(),
                project_contract_digest: digest.clone(),
                context_capsule_digest: digest.clone(),
                context_use_receipt_digest: digest.clone(),
                retained_authority_digest: digest,
                verify_node_id: verify.clone(),
                review_node_id: review.clone(),
                close_node_id: NodeId::new("close").unwrap(),
                prior_verify_starts: 2,
                provider_start_ceilings: BTreeMap::from([
                    (verify.as_str().into(), 3),
                    (review.as_str().into(), 2),
                ]),
            },
        }
    }
    #[test]
    fn finite_window_and_exact_additional_start_survive_serialization() {
        let g = grant();
        let restored: RecoveryGrant =
            serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
        assert_eq!(restored, g);
        let verify = &g.bindings.verify_node_id;
        assert!(g.permits_start(verify, 3, 100));
        for attempt in [0, 1, 2, 4] {
            assert!(!g.permits_start(verify, attempt, 100));
        }
        assert!(!g.permits_start(verify, 3, 99));
        assert!(!g.permits_start(verify, 3, g.expires_at_unix_ms));
        assert!(!g.permits_start(&NodeId::new("execute").unwrap(), 1, 100));
        assert!(!g.permits_start(&g.bindings.close_node_id, 1, 100));
        assert!(g.permits_start(&g.bindings.review_node_id, 2, 100));
        assert!(!g.permits_start(&g.bindings.review_node_id, 3, 100));
    }
    #[test]
    fn journal_replay_cannot_restart_window_or_change_binding() {
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = crate::runs::RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        assert_eq!(store.activate_recovery(g.clone()).unwrap(), g);
        let mut replay = g.clone();
        replay.activated_at_unix_ms += RECOVERY_WINDOW_MS;
        replay.expires_at_unix_ms += RECOVERY_WINDOW_MS;
        assert_eq!(store.activate_recovery(replay.clone()).unwrap(), g);
        replay.bindings.execute_receipt_digest = format!("sha256:{}", "b".repeat(64));
        assert!(store.activate_recovery(replay).is_err());
        let reopened = crate::runs::RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        assert_eq!(
            reopened.activate_recovery(g).unwrap().activated_at_unix_ms,
            100
        );
        assert_eq!(reopened.recover().unwrap().events.len(), 1);
    }

    #[test]
    fn competing_activation_produces_one_durable_window() {
        let temp = tempfile::TempDir::new().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|index| {
                let path = temp.path().to_owned();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut g = grant();
                    g.activated_at_unix_ms += index;
                    g.expires_at_unix_ms += index;
                    let store =
                        crate::runs::RunStore::open(path, g.bindings.run_id.clone()).unwrap();
                    barrier.wait();
                    store.activate_recovery(g).unwrap()
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(results.iter().all(|g| g == &results[0]));
        let store = crate::runs::RunStore::open(temp.path(), grant().bindings.run_id).unwrap();
        assert_eq!(store.recover().unwrap().events.len(), 1);
    }

    #[test]
    fn competing_recovery_starts_consume_one_attempt_and_reopen_cannot_replay() {
        use crate::runs::{AppendOutcome, RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        let transition = |key: &str, state| RunEventDraft {
            node_id: g.bindings.verify_node_id.clone(),
            idempotency_key: key.into(),
            kind: RunEventKind::NodeTransition { state },
            receipt_digest: None,
        };
        for (i, state) in [
            NodeState::Running,
            NodeState::Failed,
            NodeState::Running,
            NodeState::Failed,
            NodeState::Ready,
        ]
        .into_iter()
        .enumerate()
        {
            store
                .append(transition(&format!("prior-{i}"), state))
                .unwrap();
        }
        let draft = transition("verify:provider-running:3", NodeState::Running);
        assert!(store
            .append_recovery_start_with_clock(&g, draft.clone(), || Some(100))
            .is_err());
        store.activate_recovery(g.clone()).unwrap();
        assert!(store.append(draft.clone()).is_err());
        assert!(store
            .append_recovery_start_with_clock(&g, draft.clone(), || Some(g.expires_at_unix_ms))
            .is_err());
        let mut changed = g.clone();
        changed.bindings.context_capsule_digest = format!("sha256:{}", "b".repeat(64));
        assert!(store
            .append_recovery_start_with_clock(&changed, draft.clone(), || Some(100))
            .is_err());
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (store, g, draft, barrier) =
                    (store.clone(), g.clone(), draft.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    store
                        .append_recovery_start_with_clock(&g, draft, || Some(100))
                        .unwrap()
                })
            })
            .collect();
        let outcomes: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, AppendOutcome::Appended { .. }))
                .count(),
            1
        );
        let reopened = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        assert!(matches!(
            reopened
                .append_recovery_start_with_clock(&g, draft.clone(), || Some(g.expires_at_unix_ms))
                .unwrap(),
            AppendOutcome::AlreadyApplied { .. }
        ));
        let mut different_key = draft;
        different_key.idempotency_key = "another-start".into();
        assert!(reopened
            .append_recovery_start_with_clock(&g, different_key.clone(), || Some(100))
            .is_err());
        reopened
            .append(transition("third-failed", NodeState::Failed))
            .unwrap();
        reopened
            .append(transition("fourth-ready", NodeState::Ready))
            .unwrap();
        assert!(reopened
            .append_recovery_start_with_clock(&g, different_key, || Some(100))
            .is_err());
    }

    #[test]
    fn recovery_delimiter_remains_under_journal_lock_and_replay_never_sends() {
        use crate::runs::{AppendOutcome, RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        let transition = |key: &str, state| RunEventDraft {
            node_id: g.bindings.verify_node_id.clone(),
            idempotency_key: key.into(),
            kind: RunEventKind::NodeTransition { state },
            receipt_digest: None,
        };
        for (i, state) in [
            NodeState::Running,
            NodeState::Failed,
            NodeState::Running,
            NodeState::Failed,
            NodeState::Ready,
        ]
        .into_iter()
        .enumerate()
        {
            store
                .append(transition(&format!("prior-{i}"), state))
                .unwrap();
        }
        store.activate_recovery(g.clone()).unwrap();
        let draft = transition("verify:provider-running:3", NodeState::Running);
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(store.events_path().parent().unwrap().join("journal.lock"))
            .unwrap();
        let (outcome, sent) = store
            .recovery_start_then_with_clock(
                &g,
                draft.clone(),
                || Some(100),
                || {
                    assert_eq!(
                        fs2::FileExt::try_lock_exclusive(&lock).unwrap_err().kind(),
                        std::io::ErrorKind::WouldBlock,
                        "launch escaped journal cancellation fence"
                    );
                    // Direct test-only read avoids trying to reenter the held lock.
                    let journal = std::fs::read_to_string(store.events_path()).unwrap();
                    assert!(
                        journal.contains("verify:provider-running:3"),
                        "send preceded durable start"
                    );
                    "sent"
                },
            )
            .unwrap();
        assert!(matches!(outcome, AppendOutcome::Appended { .. }));
        assert_eq!(sent, Some("sent"));
        let reopened = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        let (outcome, sent) = reopened
            .recovery_start_then_with_clock(&g, draft, || None, || panic!("replay dispatched"))
            .unwrap();
        assert!(matches!(outcome, AppendOutcome::AlreadyApplied { .. }));
        assert!(sent.is_none());
    }

    #[test]
    fn recovery_start_preserves_review_ceiling_and_rejects_other_nodes() {
        use crate::runs::{RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        store.activate_recovery(g.clone()).unwrap();
        let transition = |node: &str, key: &str, state| RunEventDraft {
            node_id: NodeId::new(node).unwrap(),
            idempotency_key: key.into(),
            kind: RunEventKind::NodeTransition { state },
            receipt_digest: None,
        };
        for node in ["execute", "close", "sibling"] {
            store
                .append(transition(node, &format!("{node}-ready"), NodeState::Ready))
                .unwrap();
            assert!(store
                .append_recovery_start_with_clock(
                    &g,
                    transition(node, &format!("{node}-running"), NodeState::Running),
                    || Some(100)
                )
                .is_err());
        }
        for attempt in 1..=3 {
            store
                .append(transition(
                    "review",
                    &format!("review-ready-{attempt}"),
                    NodeState::Ready,
                ))
                .unwrap();
            let result = store.append_recovery_start_with_clock(
                &g,
                transition(
                    "review",
                    &format!("review-running-{attempt}"),
                    NodeState::Running,
                ),
                || Some(100),
            );
            assert_eq!(result.is_ok(), attempt <= 2);
            store
                .append(transition(
                    "review",
                    &format!("review-failed-{attempt}"),
                    NodeState::Failed,
                ))
                .unwrap();
        }
    }

    #[test]
    fn recovery_start_rejects_cancelled_and_previously_succeeded_nodes() {
        use crate::runs::{RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        for terminal in [NodeState::Cancelled, NodeState::Succeeded] {
            let temp = tempfile::TempDir::new().unwrap();
            let g = grant();
            let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
            store.activate_recovery(g.clone()).unwrap();
            for state in [terminal, NodeState::Ready] {
                store
                    .append(RunEventDraft {
                        node_id: g.bindings.review_node_id.clone(),
                        idempotency_key: format!("prior-{state:?}"),
                        kind: RunEventKind::NodeTransition { state },
                        receipt_digest: None,
                    })
                    .unwrap();
            }
            assert!(store
                .append_recovery_start_with_clock(
                    &g,
                    RunEventDraft {
                        node_id: g.bindings.review_node_id.clone(),
                        idempotency_key: "restart".into(),
                        kind: RunEventKind::NodeTransition {
                            state: NodeState::Running
                        },
                        receipt_digest: None,
                    },
                    || Some(100)
                )
                .is_err());
            assert_eq!(store.recover().unwrap().events.len(), 3);
        }
    }

    #[test]
    fn different_keys_cannot_admit_concurrent_recovery_workers() {
        use crate::runs::{RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        store.activate_recovery(g.clone()).unwrap();
        store
            .append(RunEventDraft {
                node_id: g.bindings.review_node_id.clone(),
                idempotency_key: "review-ready".into(),
                kind: RunEventKind::NodeTransition {
                    state: NodeState::Ready,
                },
                receipt_digest: None,
            })
            .unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|index| {
                let (store, g, barrier) = (store.clone(), g.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    store.append_recovery_start_with_clock(
                        &g,
                        RunEventDraft {
                            node_id: g.bindings.review_node_id.clone(),
                            idempotency_key: format!("distinct-{index}"),
                            kind: RunEventKind::NodeTransition {
                                state: NodeState::Running,
                            },
                            receipt_digest: None,
                        },
                        || Some(100),
                    )
                })
            })
            .collect();
        let results: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(store.recover().unwrap().events.len(), 3);
    }

    #[test]
    fn recovery_start_samples_clock_after_waiting_for_journal_lock() {
        use crate::runs::{RunEventDraft, RunEventKind, RunStore};
        use arda_core::run_graph::NodeState;
        use std::sync::{
            atomic::{AtomicU64, Ordering},
            mpsc, Arc,
        };
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        store.activate_recovery(g.clone()).unwrap();
        let draft = |state| RunEventDraft {
            node_id: g.bindings.review_node_id.clone(),
            idempotency_key: format!("review-{state:?}"),
            kind: RunEventKind::NodeTransition { state },
            receipt_digest: None,
        };
        store.append(draft(NodeState::Ready)).unwrap();
        let running = draft(NodeState::Running);
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(store.events_path().parent().unwrap().join("journal.lock"))
            .unwrap();
        fs2::FileExt::lock_exclusive(&lock).unwrap();
        let clock = Arc::new(AtomicU64::new(g.expires_at_unix_ms - 1));
        let (clock_called, observed) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let thread = {
            let (store, g, clock) = (store.clone(), g.clone(), clock.clone());
            std::thread::spawn(move || {
                started.send(()).unwrap();
                store.append_recovery_start_with_clock(&g, running, || {
                    let now = clock.load(Ordering::SeqCst);
                    clock_called.send(now).unwrap();
                    Some(now)
                })
            })
        };
        ready.recv().unwrap();
        assert!(matches!(
            observed.recv_timeout(std::time::Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        clock.store(g.expires_at_unix_ms, Ordering::SeqCst);
        fs2::FileExt::unlock(&lock).unwrap();
        assert_eq!(
            observed
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            g.expires_at_unix_ms
        );
        assert!(thread.join().unwrap().is_err());
        assert_eq!(store.recover().unwrap().events.len(), 2);
        assert!(store
            .append_recovery_start_with_clock(&g, draft(NodeState::Running), || None)
            .is_err());
        assert!(store
            .append_recovery_start(&g, draft(NodeState::Running))
            .is_err());
    }

    #[test]
    fn grants_require_revision_and_stop_fence_without_legacy_defaults() {
        for field in ["objective_revision", "stop_generation"] {
            let mut encoded = serde_json::to_value(grant()).unwrap();
            encoded["bindings"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RecoveryGrant>(encoded).is_err());
        }
        let mut g = grant();
        g.bindings.stop_generation = u64::MAX;
        assert!(g.validate().is_err());
        g = grant();
        g.bindings.objective_revision = 0;
        assert!(g.validate().is_err());
    }

    #[test]
    fn malformed_or_expanded_grant_fails_closed() {
        let mut g = grant();
        g.expires_at_unix_ms += 1;
        assert!(!g.is_active(100));
        g = grant();
        g.bindings
            .provider_start_ceilings
            .insert(g.bindings.verify_node_id.as_str().into(), 4);
        assert!(!g.is_active(100));
        g = grant();
        g.bindings.context_capsule_digest = "not-a-digest".into();
        assert!(!g.is_active(100));
        g = grant();
        g.bindings.prior_verify_starts = u64::MAX;
        g.bindings.provider_start_ceilings.remove("verify");
        g.bindings
            .provider_start_ceilings
            .insert("execute".into(), 1);
        assert!(!g.permits_start(&NodeId::new("execute").unwrap(), 1, 100));
    }

    #[test]
    fn generic_append_cannot_activate_and_duplicate_history_fails_closed() {
        use crate::runs::{RunEventDraft, RunEventKind, RunStore};
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        assert!(store
            .append(RunEventDraft {
                node_id: g.bindings.verify_node_id.clone(),
                idempotency_key: "bypass".into(),
                kind: RunEventKind::RecoveryActivated {
                    grant: Box::new(g.clone())
                },
                receipt_digest: Some(g.authenticated_payload_digest.clone()),
            })
            .is_err());
        assert!(store.recover().unwrap().events.is_empty());
        store.activate_recovery(g.clone()).unwrap();
        let event = store.recover().unwrap().events.remove(0);
        for mutation in 0..6 {
            let mut corrupt = event.clone();
            match mutation {
                0 => corrupt.sequence = 2,
                1 => corrupt.node_id = NodeId::new("execute").unwrap(),
                2 => corrupt.idempotency_key = "bypass".into(),
                3 => corrupt.receipt_digest = None,
                4 | 5 => {
                    if let RunEventKind::RecoveryActivated { grant } = &mut corrupt.kind {
                        if mutation == 4 {
                            grant.bindings.run_id = RunId::new("other").unwrap();
                        } else {
                            grant.expires_at_unix_ms += 1;
                        }
                    }
                }
                _ => unreachable!(),
            }
            let mut raw = if mutation == 0 {
                format!("{}\n", serde_json::to_string(&event).unwrap())
            } else {
                String::new()
            };
            raw.push_str(&format!("{}\n", serde_json::to_string(&corrupt).unwrap()));
            std::fs::write(store.events_path(), raw).unwrap();
            assert!(store.recover().is_err(), "mutation {mutation}");
            assert!(store.activate_recovery(g.clone()).is_err());
        }
    }

    #[test]
    fn journal_reader_waits_for_locked_partial_write() {
        use std::io::Write;
        let temp = tempfile::TempDir::new().unwrap();
        let g = grant();
        let store = crate::runs::RunStore::open(temp.path(), g.bindings.run_id.clone()).unwrap();
        store.activate_recovery(g).unwrap();
        let raw = std::fs::read(store.events_path()).unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(store.events_path().parent().unwrap().join("journal.lock"))
            .unwrap();
        fs2::FileExt::lock_exclusive(&lock).unwrap();
        let mut journal = std::fs::File::create(store.events_path()).unwrap();
        journal.write_all(&raw[..raw.len() / 2]).unwrap();
        let reader = store.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || tx.send(reader.recover()).unwrap());
        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        journal.write_all(&raw[raw.len() / 2..]).unwrap();
        journal.sync_all().unwrap();
        fs2::FileExt::unlock(&lock).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(5))
                .unwrap()
                .unwrap()
                .events
                .len(),
            1
        );
        thread.join().unwrap();
    }
}
