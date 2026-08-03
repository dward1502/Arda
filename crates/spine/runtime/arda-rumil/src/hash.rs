//! Bounded SHA-256 evidence hashing helpers.
//!
//! Hashes are computed only for files within the configured budget.

use crate::error::{Result, RumilError};

#[cfg(feature = "crypto")]
use sha2::{Digest, Sha256};

/// SHA-256 hex digest of a byte slice.
#[cfg(feature = "crypto")]
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Read a file and return its SHA-256 digest, capped to `max_bytes`.
#[cfg(feature = "crypto")]
pub fn hash_file(path: &std::path::Path, max_bytes: u64) -> Result<String> {
    let len = std::fs::metadata(path).map_err(RumilError::Io)?.len();
    if len > max_bytes {
        return Err(RumilError::DeniedByBudget(format!(
            "file {} bytes exceeds budget {} bytes",
            len, max_bytes
        )));
    }
    let data = std::fs::read(path).map_err(RumilError::Io)?;
    Ok(sha256_bytes(&data))
}

#[cfg(all(test, feature = "crypto"))]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_file_within_budget() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "hello rumil").unwrap();
        let digest = hash_file(&path, 1024).unwrap();
        assert!(!digest.is_empty());
    }

    #[test]
    fn hash_file_rejects_oversize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&[0u8; 64]).unwrap();
        assert!(hash_file(&path, 32).is_err());
    }
}
