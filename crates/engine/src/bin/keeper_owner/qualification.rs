//! Keep failed workers and their pins owned until a nonblocking reap proves exit.
use super::pending::PendingAdmission;
use std::{cell::RefCell, process::Child, rc::Rc};

struct Unreaped<P> {
    child: Child,
    pins: Option<P>,
}
impl<P> Unreaped<P> {
    fn observe(&mut self, mut reaped: impl FnMut(&mut Child) -> bool) -> bool {
        if reaped(&mut self.child) {
            self.pins.take();
            true
        } else {
            false
        }
    }
    fn reap(&mut self) -> bool {
        self.observe(|child| matches!(child.try_wait(), Ok(Some(_))))
    }
}
impl<P> Drop for Unreaped<P> {
    fn drop(&mut self) {
        let _ = self.child.kill();
        if !self.reap() {
            // Keeper teardown must not release pins while a failed child may
            // still use them. Retain them for the remaining process lifetime;
            // the durable preparing row still requires explicit reconciliation.
            if let Some(pins) = self.pins.take() {
                std::mem::forget(pins);
            }
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Cleanup(Rc<RefCell<Vec<Unreaped<PendingAdmission>>>>);
impl Cleanup {
    /// One failed preparation can remain pending; admission is refused until it
    /// reaps, bounding retained resources without blocking other control traffic.
    pub(crate) fn pending(&self) -> bool {
        let mut pending = self.0.borrow_mut();
        pending.retain_mut(|worker| !worker.reap());
        !pending.is_empty()
    }
}

pub(super) struct Qualification {
    pub child: Option<Child>,
    pub pending: Option<PendingAdmission>,
    pub cleanup: Cleanup,
}
impl Qualification {
    pub fn into_child(mut self) -> Child {
        self.child.take().expect("qualification owns child")
    }
}
impl Drop for Qualification {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let mut worker = Unreaped {
                child,
                pins: self.pending.take(),
            };
            if !worker.reap() {
                self.cleanup.0.borrow_mut().push(worker);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        process::Command,
        time::{Duration, Instant},
    };

    struct Pins(Rc<Cell<usize>>);
    impl Drop for Pins {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn preparation_error_cleanup_has_no_blocking_child_wait() {
        let source = include_str!("qualification.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(
            !production.contains(".wait("),
            "serial owner cleanup must not block in wait"
        );
    }

    #[test]
    fn unresolved_observation_retains_pins_until_real_reap() {
        let released = Rc::new(Cell::new(0));
        let child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let mut worker = Unreaped {
            child,
            pins: Some(Pins(released.clone())),
        };
        worker.child.kill().unwrap();
        // Deterministic model of a child whose exit cannot yet be observed.
        assert!(!worker.observe(|_| false));
        assert!(worker.pins.is_some());
        assert_eq!(released.get(), 0);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !worker.reap() {
            assert!(Instant::now() < deadline, "fixture child did not reap");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(released.get(), 1);
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn qualification_error_returns_and_owned_child_is_eventually_reaped() {
        let cleanup = Cleanup::default();
        let child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let began = Instant::now();
        drop(Qualification {
            child: Some(child),
            pending: None,
            cleanup: cleanup.clone(),
        });
        assert!(began.elapsed() < Duration::from_secs(1));
        let deadline = Instant::now() + Duration::from_secs(3);
        while cleanup.pending() {
            assert!(Instant::now() < deadline, "fixture cleanup did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
