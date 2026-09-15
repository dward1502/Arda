//! Execution requests name the caller's lease; they cannot borrow a newer one.
use super::ApiError;
use crate::objectives::snapshot_protocol::Lease;

pub(super) fn execution_config(
    ordinary: &std::path::Path,
    retained: bool,
    override_path: Option<&std::ffi::OsStr>,
) -> std::path::PathBuf {
    if retained {
        if let Some(path) = override_path {
            return path.into();
        }
    }
    ordinary.into()
}

pub(super) fn require_expected_retained_lease(
    actual: Option<&Lease>,
    expected: Option<&Lease>,
) -> Result<(), ApiError> {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual == expected => Ok(()),
        (Some(_), _) => Err(ApiError::conflict(
            "retained provider dispatch requires the caller's current lease",
        )),
        (None, Some(_)) => Err(ApiError::conflict(
            "expected retained provider authority is no longer available",
        )),
        (None, None) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_config_does_not_replace_ordinary_execution_config() {
        let ordinary = std::path::Path::new("/ordinary.toml");
        let retained = std::ffi::OsStr::new("/retained.toml");
        assert_eq!(execution_config(ordinary, false, Some(retained)), ordinary);
        assert_eq!(execution_config(ordinary, true, None), ordinary);
        assert_eq!(
            execution_config(ordinary, true, Some(retained)),
            std::path::Path::new("/retained.toml")
        );
    }
    #[test]
    fn stale_or_missing_expected_lease_cannot_borrow_current_authority() {
        let actual = Lease {
            run_id: "run".into(),
            generation: 2,
            owner: "new-owner".into(),
            expires_ms: 100,
        };
        assert!(require_expected_retained_lease(Some(&actual), Some(&actual)).is_ok());
        assert!(require_expected_retained_lease(Some(&actual), None).is_err());
        assert!(require_expected_retained_lease(None, Some(&actual)).is_err());
        assert!(require_expected_retained_lease(None, None).is_ok());
        for field in ["run", "generation", "owner", "expiry"] {
            let mut stale = actual.clone();
            match field {
                "run" => stale.run_id = "other".into(),
                "generation" => stale.generation = 1,
                "owner" => stale.owner = "old-owner".into(),
                _ => stale.expires_ms = 99,
            }
            assert!(require_expected_retained_lease(Some(&actual), Some(&stale)).is_err());
        }
    }
}
