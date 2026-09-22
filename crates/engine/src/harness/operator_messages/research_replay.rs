//! Durable research delivery independent of transport acceptance. The OS lock
//! proves liveness; elapsed wall time never proves an attempt has terminated.
use super::*;
use fs2::FileExt;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[cfg(test)]
mod tests;

pub(super) const DEADLINE_SECONDS: u64 = 60;

#[derive(Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Claimed,
    Executing,
    Terminal,
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize)]
struct Record {
    payload_digest: String,
    started_at: chrono::DateTime<Utc>,
    #[serde(default)]
    phase: Phase,
    response: Option<GatewayOperatorResponse>,
}

pub(super) struct Operation {
    path: PathBuf,
    // Closing the descriptor releases the cross-process lock even on cancellation.
    _lock: fs::File,
}

impl Drop for Operation {
    fn drop(&mut self) {
        // Explicit unlock also releases transient descriptors inherited by a
        // concurrently spawning child before it reaches exec/CLOEXEC.
        let _ = FileExt::unlock(&self._lock);
    }
}

pub(super) enum Claim {
    Started(Operation),
    Complete(GatewayOperatorResponse),
}

pub(super) fn claim(
    root: &Path,
    event_id: &str,
    digest: &str,
    session: &str,
    owner: &str,
) -> Result<Claim, ApiError> {
    let directory = root.join("data/workbench/research/operations");
    fs::create_dir_all(&directory).map_err(io_error)?;
    let path = directory.join(format!("{:x}.json", Sha256::digest(event_id.as_bytes())));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))
        .map_err(io_error)?;
    lock.try_lock_exclusive().map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            ApiError::internal("Research is pending; retry this event to retrieve its result")
        } else {
            io_error(error)
        }
    })?;
    let operation = Operation { path, _lock: lock };
    let record = match fs::read(&operation.path) {
        Ok(bytes) => serde_json::from_slice::<Record>(&bytes).map_err(|_| {
            ApiError::internal("Research operation record is unreadable; refusing replay")
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let record = Record {
                payload_digest: digest.into(),
                started_at: Utc::now(),
                phase: Phase::Claimed,
                response: None,
            };
            write_record(&operation.path, &record)?;
            return Ok(Claim::Started(operation));
        }
        Err(error) => return Err(io_error(error)),
    };
    if record.payload_digest != digest {
        return Err(ApiError::bad_request(
            "Research event is bound to different content",
        ));
    }
    // Publication is itself the durable commit. Recover it even if the process
    // died before assimilation or response journaling; never repeat discovery.
    let known_brief = record.response.as_ref().and_then(|response| {
        response
            .evidence_refs
            .iter()
            .find_map(|reference| reference.strip_prefix("arda://research/briefs/"))
    });
    let publication = if let Some(id) = known_brief {
        // A known target must validate even if unrelated publications are corrupt.
        Some(super::super::research::question::read_bound_answer(
            root,
            owner,
            id,
            Some(event_id),
        )?)
    } else {
        super::super::research::question::recover_answer(root, owner, event_id)?
    };
    if let Some((summary, mut refs)) = publication {
        refs.insert(0, format!("arda://operator-events/{event_id}"));
        let response = GatewayOperatorResponse {
            schema_version: "arda.gateway-operator-response.v1".into(),
            summary,
            evidence_refs: refs,
            session_id: session.into(),
            run_id: None,
        };
        finish(&operation, &response)?;
        return Ok(Claim::Complete(response));
    }
    if let Some(response) = record.response {
        return Ok(Claim::Complete(response));
    }
    if record.phase == Phase::Claimed {
        return Ok(Claim::Started(operation));
    }
    let response = GatewayOperatorResponse {
        schema_version: "arda.gateway-operator-response.v1".into(),
        summary: "Research was interrupted without a published result. External capture work may have occurred; it has not been rerun. Send a new research request to retry; no commitment was created.".into(),
        evidence_refs: vec![format!("arda://operator-events/{event_id}")], session_id: session.into(), run_id: None,
    };
    finish(&operation, &response)?;
    Ok(Claim::Complete(response))
}

pub(super) fn executing(operation: &Operation) -> Result<(), ApiError> {
    let mut record = read_record(&operation.path)?;
    record.phase = Phase::Executing;
    write_record(&operation.path, &record)
}

pub(super) fn finish(
    operation: &Operation,
    response: &GatewayOperatorResponse,
) -> Result<(), ApiError> {
    let mut record = read_record(&operation.path)?;
    record.phase = Phase::Terminal;
    record.response = Some(response.clone());
    write_record(&operation.path, &record)
}

fn read_record(path: &Path) -> Result<Record, ApiError> {
    serde_json::from_slice(&fs::read(path).map_err(io_error)?)
        .map_err(|_| ApiError::internal("Research operation record is unreadable"))
}

fn write_record(path: &Path, record: &Record) -> Result<(), ApiError> {
    let directory = path
        .parent()
        .ok_or_else(|| ApiError::internal("Invalid research operation path"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(io_error)?;
    serde_json::to_writer(&mut temporary, record).map_err(|e| ApiError::internal(e.to_string()))?;
    temporary.flush().map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary.persist(path).map_err(|e| io_error(e.error))?;
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(io_error)?;
    Ok(())
}
fn io_error(error: std::io::Error) -> ApiError {
    ApiError::internal(format!("Research delivery storage failed: {error}"))
}
