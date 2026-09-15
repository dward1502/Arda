//! Fencing for one retained tree. Rebind never prepares or replaces the tree.
use anyhow::{bail, Context, Result};
pub use arda_engine::objectives::snapshot_protocol::Lease;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct Binding {
    pub lease: Lease,
    deadline: Instant,
}

pub fn now_ms() -> Result<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

impl Binding {
    pub fn commit(saved: &mut Option<Self>, lease: Lease, now: i64) -> Result<()> {
        if lease.run_id.is_empty() || lease.owner.is_empty() || lease.generation < 1 {
            bail!("invalid snapshot lease identity");
        }
        if let Some(prior) = saved {
            if prior.lease == lease {
                // ACK-loss retry must not extend the original monotonic deadline.
                return Ok(());
            }
            if prior.lease.run_id != lease.run_id || lease.generation <= prior.lease.generation {
                bail!("snapshot lease is fenced or same-generation payload changed");
            }
        }
        let remaining = lease
            .expires_ms
            .checked_sub(now)
            .context("snapshot lease duration overflow")?
            .max(0);
        // First delivery can follow expiry of the durable Engine intent. Record
        // only its exact fence, allowing recovery to advance to a new generation.
        // remaining() rejects execution even after a backward wall-clock jump;
        // an identical retry never extends this monotonic deadline.
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(remaining as u64))
            .context("snapshot lease deadline overflow")?;
        *saved = Some(Self { lease, deadline });
        Ok(())
    }

    pub fn remaining(&self, lease: &Lease, now: i64) -> Result<Duration> {
        if &self.lease != lease {
            bail!("snapshot execution lease is fenced");
        }
        let wall = self
            .lease
            .expires_ms
            .checked_sub(now)
            .filter(|v| *v > 0)
            .context("snapshot lease has expired")?;
        let monotonic = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .context("snapshot lease has expired")?;
        Ok(monotonic.min(Duration::from_millis(wall as u64)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lease() -> Lease {
        Lease {
            run_id: "run".into(),
            generation: 1,
            owner: "first".into(),
            expires_ms: 1000,
        }
    }
    #[test]
    fn rebind_fences_old_generation_and_preserves_run() {
        let mut binding = None;
        Binding::commit(&mut binding, lease(), 0).unwrap();
        let next = Lease {
            generation: 2,
            owner: "second".into(),
            ..lease()
        };
        Binding::commit(&mut binding, next.clone(), 1).unwrap();
        assert!(Binding::commit(&mut binding, lease(), 2).is_err());
        assert!(binding.as_ref().unwrap().remaining(&lease(), 2).is_err());
        assert!(binding.as_ref().unwrap().remaining(&next, 2).is_ok());
        assert!(Binding::commit(
            &mut binding,
            Lease {
                run_id: "other".into(),
                generation: 3,
                ..next
            },
            2
        )
        .is_err());
    }
    #[test]
    fn late_first_commit_records_only_an_expired_fence() {
        let mut binding = None;
        Binding::commit(&mut binding, lease(), 2000).unwrap();
        assert!(binding.as_ref().unwrap().remaining(&lease(), 2000).is_err());
        assert!(binding.as_ref().unwrap().remaining(&lease(), 0).is_err());
        let deadline = binding.as_ref().unwrap().deadline;
        Binding::commit(&mut binding, lease(), 0).unwrap();
        assert_eq!(binding.as_ref().unwrap().deadline, deadline);
        assert!(Binding::commit(
            &mut binding,
            Lease {
                expires_ms: 3000,
                ..lease()
            },
            2000,
        )
        .is_err());
        let next = Lease {
            generation: 2,
            expires_ms: 3000,
            ..lease()
        };
        Binding::commit(&mut binding, next.clone(), 2000).unwrap();
        assert!(binding.as_ref().unwrap().remaining(&next, 2000).is_ok());
        assert!(Binding::commit(&mut binding, lease(), 2000).is_err());
    }
    #[test]
    fn acknowledgement_retry_cannot_extend_deadline_or_change_payload() {
        let mut binding = None;
        Binding::commit(&mut binding, lease(), 0).unwrap();
        let deadline = binding.as_ref().unwrap().deadline;
        Binding::commit(&mut binding, lease(), 2000).unwrap();
        assert_eq!(binding.as_ref().unwrap().deadline, deadline);
        assert!(binding.as_ref().unwrap().remaining(&lease(), 2000).is_err());
        assert!(Binding::commit(
            &mut binding,
            Lease {
                expires_ms: 3000,
                ..lease()
            },
            1
        )
        .is_err());
        assert!(Binding::commit(
            &mut binding,
            Lease {
                owner: "other".into(),
                ..lease()
            },
            1
        )
        .is_err());
    }
}
