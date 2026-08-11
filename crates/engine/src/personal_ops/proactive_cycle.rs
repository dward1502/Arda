//! Durable, exactly-once coordination for bounded proactive communication.
//!
//! This ledger stores policy decisions, delivery receipts, and operator responses.
//! Message bodies remain transient and are deliberately absent from the schema.

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
};

use arda_core::proactive_communication::{
    evaluate_proactive_communication, PriorOperatorResponse, ProactiveChannel,
    ProactiveCommunicationDisposition, ProactiveCommunicationInput, ProactiveCommunicationPolicy,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct ProactiveCycleStore {
    ledger_path: PathBuf,
}

impl ProactiveCycleStore {
    pub fn new(root: &Path) -> Self {
        Self {
            ledger_path: root.join("data/personal/proactive_cycle.jsonl"),
        }
    }

    pub fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    pub fn evaluate_once(
        &self,
        policy: &ProactiveCommunicationPolicy,
        input: &ProactiveCommunicationInput,
    ) -> Result<ProactiveEvaluation, ProactiveCycleError> {
        let input_digest = digest(input)?;
        let disposition = evaluate_proactive_communication(policy, input);
        let mut file = self.open_locked()?;
        let projection = load_projection(&mut file)?;

        if let Some(existing) = projection.evaluations.get(&input.event_id) {
            if existing.input_digest != input_digest || existing.disposition != disposition {
                return Err(ProactiveCycleError::EventConflict {
                    event_id: input.event_id.clone(),
                });
            }
            return Ok(ProactiveEvaluation {
                status: ProactiveEvaluationStatus::AlreadyRecorded,
                disposition: existing.disposition.clone(),
                input_digest,
            });
        }

        let event = LedgerEvent::Evaluation {
            input_digest: input_digest.clone(),
            disposition: disposition.clone(),
        };
        append_event(
            &mut file,
            projection.next_sequence,
            &input.event_id,
            input.evaluated_at,
            event,
        )?;
        Ok(ProactiveEvaluation {
            status: ProactiveEvaluationStatus::Recorded,
            disposition,
            input_digest,
        })
    }

    pub fn delivery_permit(&self, event_id: &str) -> Result<DeliveryPermit, ProactiveCycleError> {
        let projection = self.load_all()?;
        let evaluation = projection.evaluations.get(event_id).ok_or_else(|| {
            ProactiveCycleError::UnknownEvent {
                event_id: event_id.to_string(),
            }
        })?;

        if let Some(response) = projection.operator_responses.get(event_id) {
            return Ok(DeliveryPermit::SuppressedByOperatorResponse(*response));
        }
        if let Some(receipt) = projection.deliveries.get(event_id) {
            return Ok(DeliveryPermit::AlreadyDelivered {
                idempotency_key: receipt.idempotency_key.clone(),
                provider_message_id: receipt.provider_message_id.clone(),
            });
        }
        let Some(channel) = evaluation.disposition.channel else {
            return Ok(DeliveryPermit::NotAuthorized);
        };
        if !evaluation.disposition.delivery_authorized {
            return Ok(DeliveryPermit::NotAuthorized);
        }
        Ok(DeliveryPermit::Ready {
            idempotency_key: delivery_key(event_id, &evaluation.input_digest),
            channel,
        })
    }

    pub fn record_delivery(
        &self,
        event_id: &str,
        idempotency_key: &str,
        provider_message_id: &str,
        delivered_at: DateTime<Utc>,
    ) -> Result<bool, ProactiveCycleError> {
        if provider_message_id.trim().is_empty() {
            return Err(ProactiveCycleError::EmptyProviderMessageId);
        }
        let mut file = self.open_locked()?;
        let projection = load_projection(&mut file)?;
        let evaluation = projection.evaluations.get(event_id).ok_or_else(|| {
            ProactiveCycleError::UnknownEvent {
                event_id: event_id.to_string(),
            }
        })?;
        if !evaluation.disposition.delivery_authorized {
            return Err(ProactiveCycleError::DeliveryNotAuthorized {
                event_id: event_id.to_string(),
            });
        }
        if projection.operator_responses.contains_key(event_id) {
            return Err(ProactiveCycleError::OperatorResponseTerminal {
                event_id: event_id.to_string(),
            });
        }
        let expected_key = delivery_key(event_id, &evaluation.input_digest);
        if idempotency_key != expected_key {
            return Err(ProactiveCycleError::InvalidDeliveryKey {
                event_id: event_id.to_string(),
            });
        }
        if let Some(existing) = projection.deliveries.get(event_id) {
            if existing.idempotency_key == idempotency_key
                && existing.provider_message_id == provider_message_id
            {
                return Ok(false);
            }
            return Err(ProactiveCycleError::DeliveryConflict {
                event_id: event_id.to_string(),
            });
        }

        append_event(
            &mut file,
            projection.next_sequence,
            event_id,
            delivered_at,
            LedgerEvent::Delivery {
                idempotency_key: idempotency_key.to_string(),
                provider_message_id: provider_message_id.to_string(),
            },
        )?;
        Ok(true)
    }

    pub fn record_operator_response(
        &self,
        event_id: &str,
        response: PriorOperatorResponse,
        responded_at: DateTime<Utc>,
    ) -> Result<bool, ProactiveCycleError> {
        if !matches!(
            response,
            PriorOperatorResponse::Acknowledged | PriorOperatorResponse::Dismissed
        ) {
            return Err(ProactiveCycleError::NonTerminalOperatorResponse);
        }
        let mut file = self.open_locked()?;
        let projection = load_projection(&mut file)?;
        if !projection.evaluations.contains_key(event_id) {
            return Err(ProactiveCycleError::UnknownEvent {
                event_id: event_id.to_string(),
            });
        }
        if let Some(existing) = projection.operator_responses.get(event_id) {
            if *existing == response {
                return Ok(false);
            }
            return Err(ProactiveCycleError::OperatorResponseConflict {
                event_id: event_id.to_string(),
            });
        }
        append_event(
            &mut file,
            projection.next_sequence,
            event_id,
            responded_at,
            LedgerEvent::OperatorResponse { response },
        )?;
        Ok(true)
    }

    pub fn load_all(&self) -> Result<ProactiveCycleProjection, ProactiveCycleError> {
        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .open(&self.ledger_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProactiveCycleProjection::default())
            }
            Err(error) => return Err(ProactiveCycleError::Io(error)),
        };
        let _lock = FileLock::shared(&file)?;
        load_projection(&mut file)
    }

    fn open_locked(&self) -> Result<LockedFile, ProactiveCycleError> {
        let parent = self
            .ledger_path
            .parent()
            .expect("proactive-cycle path has a parent");
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.ledger_path)?;
        std::fs::set_permissions(&self.ledger_path, std::fs::Permissions::from_mode(0o600))?;
        LockedFile::exclusive(file).map_err(ProactiveCycleError::Io)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactiveEvaluationStatus {
    Recorded,
    AlreadyRecorded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProactiveEvaluation {
    pub status: ProactiveEvaluationStatus,
    pub disposition: ProactiveCommunicationDisposition,
    pub input_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryPermit {
    Ready {
        idempotency_key: String,
        channel: ProactiveChannel,
    },
    AlreadyDelivered {
        idempotency_key: String,
        provider_message_id: String,
    },
    SuppressedByOperatorResponse(PriorOperatorResponse),
    NotAuthorized,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedEvaluation {
    pub input_digest: String,
    pub disposition: ProactiveCommunicationDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryReceipt {
    pub idempotency_key: String,
    pub provider_message_id: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProactiveCycleProjection {
    pub evaluations: BTreeMap<String, PersistedEvaluation>,
    pub deliveries: BTreeMap<String, DeliveryReceipt>,
    pub operator_responses: BTreeMap<String, PriorOperatorResponse>,
    next_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerEnvelope {
    schema_version: String,
    sequence: u64,
    event_id: String,
    recorded_at: DateTime<Utc>,
    event: LedgerEvent,
}

impl LedgerEnvelope {
    const SCHEMA_VERSION: &'static str = "arda.proactive-cycle-ledger.v1";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum LedgerEvent {
    Evaluation {
        input_digest: String,
        disposition: ProactiveCommunicationDisposition,
    },
    Delivery {
        idempotency_key: String,
        provider_message_id: String,
    },
    OperatorResponse {
        response: PriorOperatorResponse,
    },
}

fn load_projection(
    file: &mut std::fs::File,
) -> Result<ProactiveCycleProjection, ProactiveCycleError> {
    file.seek(SeekFrom::Start(0))?;
    let mut projection = ProactiveCycleProjection::default();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let envelope: LedgerEnvelope =
            serde_json::from_str(&line).map_err(|error| ProactiveCycleError::CorruptEntry {
                line: index + 1,
                message: error.to_string(),
            })?;
        let expected = projection.next_sequence + 1;
        if envelope.schema_version != LedgerEnvelope::SCHEMA_VERSION {
            return Err(ProactiveCycleError::UnsupportedVersion(
                envelope.schema_version,
            ));
        }
        if envelope.sequence != expected {
            return Err(ProactiveCycleError::SequenceGap {
                expected,
                actual: envelope.sequence,
            });
        }
        projection.next_sequence = envelope.sequence;
        match envelope.event {
            LedgerEvent::Evaluation {
                input_digest,
                disposition,
            } => {
                if projection
                    .evaluations
                    .insert(
                        envelope.event_id.clone(),
                        PersistedEvaluation {
                            input_digest,
                            disposition,
                        },
                    )
                    .is_some()
                {
                    return Err(ProactiveCycleError::DuplicateEvaluation {
                        event_id: envelope.event_id,
                    });
                }
            }
            LedgerEvent::Delivery {
                idempotency_key,
                provider_message_id,
            } => {
                if !projection.evaluations.contains_key(&envelope.event_id) {
                    return Err(ProactiveCycleError::DeliveryBeforeEvaluation {
                        event_id: envelope.event_id,
                    });
                }
                if projection
                    .deliveries
                    .insert(
                        envelope.event_id.clone(),
                        DeliveryReceipt {
                            idempotency_key,
                            provider_message_id,
                        },
                    )
                    .is_some()
                {
                    return Err(ProactiveCycleError::DeliveryConflict {
                        event_id: envelope.event_id,
                    });
                }
            }
            LedgerEvent::OperatorResponse { response } => {
                if !projection.evaluations.contains_key(&envelope.event_id) {
                    return Err(ProactiveCycleError::ResponseBeforeEvaluation {
                        event_id: envelope.event_id,
                    });
                }
                if projection
                    .operator_responses
                    .insert(envelope.event_id.clone(), response)
                    .is_some()
                {
                    return Err(ProactiveCycleError::OperatorResponseConflict {
                        event_id: envelope.event_id,
                    });
                }
            }
        }
    }
    Ok(projection)
}

fn append_event(
    file: &mut LockedFile,
    current_sequence: u64,
    event_id: &str,
    recorded_at: DateTime<Utc>,
    event: LedgerEvent,
) -> Result<(), ProactiveCycleError> {
    let envelope = LedgerEnvelope {
        schema_version: LedgerEnvelope::SCHEMA_VERSION.to_string(),
        sequence: current_sequence + 1,
        event_id: event_id.to_string(),
        recorded_at,
        event,
    };
    let line = serde_json::to_string(&envelope)?;
    writeln!(file, "{line}")?;
    file.sync_all()?;
    Ok(())
}

fn digest(value: &impl Serialize) -> Result<String, ProactiveCycleError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn delivery_key(event_id: &str, input_digest: &str) -> String {
    let material = format!("arda.proactive-delivery.v1\0{event_id}\0{input_digest}");
    format!("sha256:{:x}", Sha256::digest(material.as_bytes()))
}

struct FileLock {
    fd: std::os::fd::RawFd,
}

impl FileLock {
    fn shared(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_SH)
    }

    fn acquire(file: &std::fs::File, operation: libc::c_int) -> std::io::Result<Self> {
        let fd = file.as_raw_fd();
        // SAFETY: `fd` belongs to the live file held by the caller.
        if unsafe { libc::flock(fd, operation) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: the locked file outlives this guard.
        let _ = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
    }
}

struct LockedFile {
    file: std::fs::File,
    _lock: FileLock,
}

impl LockedFile {
    fn exclusive(file: std::fs::File) -> std::io::Result<Self> {
        let lock = FileLock::acquire(&file, libc::LOCK_EX)?;
        Ok(Self { file, _lock: lock })
    }
}

impl std::ops::Deref for LockedFile {
    type Target = std::fs::File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl std::ops::DerefMut for LockedFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

impl Write for LockedFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProactiveCycleError {
    #[error("proactive-cycle I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("proactive-cycle serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("corrupt proactive-cycle entry at line {line}: {message}")]
    CorruptEntry { line: usize, message: String },
    #[error("unsupported proactive-cycle ledger version: {0}")]
    UnsupportedVersion(String),
    #[error("proactive-cycle sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    #[error("proactive event {event_id:?} conflicts with its durable evaluation")]
    EventConflict { event_id: String },
    #[error("unknown proactive event {event_id:?}")]
    UnknownEvent { event_id: String },
    #[error("duplicate proactive evaluation for {event_id:?}")]
    DuplicateEvaluation { event_id: String },
    #[error("delivery for {event_id:?} was recorded before evaluation")]
    DeliveryBeforeEvaluation { event_id: String },
    #[error("operator response for {event_id:?} was recorded before evaluation")]
    ResponseBeforeEvaluation { event_id: String },
    #[error("delivery is not authorized for proactive event {event_id:?}")]
    DeliveryNotAuthorized { event_id: String },
    #[error("proactive delivery key is invalid for event {event_id:?}")]
    InvalidDeliveryKey { event_id: String },
    #[error("proactive delivery conflicts for event {event_id:?}")]
    DeliveryConflict { event_id: String },
    #[error("provider message id cannot be empty")]
    EmptyProviderMessageId,
    #[error("operator response is already terminal for event {event_id:?}")]
    OperatorResponseTerminal { event_id: String },
    #[error("only acknowledgement or dismissal can terminally suppress delivery")]
    NonTerminalOperatorResponse,
    #[error("operator response conflicts for event {event_id:?}")]
    OperatorResponseConflict { event_id: String },
}
