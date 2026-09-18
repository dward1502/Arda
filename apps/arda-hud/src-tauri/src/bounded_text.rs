//! Whole-file IPC is for compact projections, never unbounded historical ledgers.
use std::{io, path::Path};

pub const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;

pub fn read_text(path: &Path) -> io::Result<String> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    let oversized = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "HUD text read exceeds 8 MiB; use a compact projection or paginated reader",
        )
    };
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HUD text read requires a regular file",
        ));
    }
    if metadata.len() > MAX_TEXT_BYTES {
        return Err(oversized());
    }
    // Bound the actual read too: the file can grow after metadata was sampled.
    let mut bytes = Vec::new();
    file.take(MAX_TEXT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_TEXT_BYTES {
        return Err(oversized());
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_file_without_returning_partial_content() {
        let path = std::env::temp_dir().join(format!("hud-read-limit-{}", std::process::id()));
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_TEXT_BYTES + 1).unwrap();
        let result = read_text(&path);
        std::fs::remove_file(path).unwrap();
        assert!(result.is_err(), "oversized IPC payload must be rejected");
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn reads_compact_utf8_projection() {
        let path = std::env::temp_dir().join(format!("hud-read-small-{}", std::process::id()));
        std::fs::write(&path, "{\"status\":\"ready\"}").unwrap();
        let result = read_text(&path);
        std::fs::remove_file(path).unwrap();
        assert_eq!(result.unwrap(), "{\"status\":\"ready\"}");
    }
}
