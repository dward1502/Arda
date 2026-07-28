use crate::{EvidenceRef, OracleQuery, OracleQueryError, OracleService, QueryType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::path::{Component, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateRequest {
    pub id: Option<String>,
    pub task: String,
    pub requester: Option<String>,
    pub context: Option<Vec<String>>,
    pub evidence: Option<Vec<EvidenceRef>>,
    pub query_type: Option<QueryType>,
    pub timestamp: Option<DateTime<Utc>>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

impl EvaluateRequest {
    pub fn from_payload(payload: Value) -> Result<Self, DispatchError> {
        serde_json::from_value(payload).map_err(|err| {
            DispatchError::invalid_request(format!("invalid evaluate payload: {err}"))
        })
    }

    fn into_query(self, id_prefix: &str) -> OracleQuery {
        let mut query = OracleQuery::new(
            self.id
                .unwrap_or_else(|| format!("{id_prefix}::{}", uuid::Uuid::new_v4())),
            self.task,
            self.requester.unwrap_or_else(|| "operator".to_string()),
        );
        query.context = self.context.unwrap_or_default();
        query.evidence = self
            .evidence
            .unwrap_or_default()
            .into_iter()
            .map(|evidence| evidence.with_sensitive_excerpt(true))
            .collect();
        query.query_type = self.query_type.unwrap_or_default();
        query.timestamp = self.timestamp.unwrap_or_else(Utc::now);
        query.correlation_id = self.correlation_id;
        query.causation_id = self.causation_id;
        query
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLedgerRequest {
    pub destination: PathBuf,
}

impl ExportLedgerRequest {
    pub fn from_payload(payload: Value) -> Result<Self, DispatchError> {
        serde_json::from_value(payload).map_err(|err| {
            DispatchError::invalid_request(format!("invalid export_ledger payload: {err}"))
        })
    }
}

#[derive(Debug, Clone)]
pub enum DispatchRequest {
    Status,
    Evaluate {
        request: EvaluateRequest,
        id_prefix: &'static str,
    },
    Verdicts {
        limit: usize,
    },
    Paths,
    VerifyLedger,
    ExportLedger {
        destination: PathBuf,
    },
}

fn transport_export_destination(
    service: &OracleService,
    destination: PathBuf,
) -> Result<PathBuf, DispatchError> {
    if destination.as_os_str().is_empty()
        || destination
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(DispatchError::invalid_request(
            "export destination must be a relative path beneath the Mandos export directory",
        ));
    }

    Ok(PathBuf::from(service.runtime_paths().home)
        .join("exports")
        .join(destination))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchError {
    pub code: &'static str,
    pub message: String,
    pub http_status: u16,
}

impl DispatchError {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: "INVALID_REQUEST",
            message: message.into(),
            http_status: 400,
        }
    }

    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self {
            code: "INVALID_QUERY",
            message: message.into(),
            http_status: 422,
        }
    }

    pub fn payload_too_large(limit: usize) -> Self {
        Self {
            code: "PAYLOAD_TOO_LARGE",
            message: format!("request payload exceeds {limit} bytes"),
            http_status: 413,
        }
    }

    pub fn unknown_command(command: &str) -> Self {
        Self {
            code: "UNKNOWN_COMMAND",
            message: format!("unknown ORACLE command: {command}"),
            http_status: 404,
        }
    }

    pub fn body(&self) -> Value {
        json!({
            "ok": false,
            "error": {
                "code": self.code,
                "message": self.message,
            }
        })
    }

    fn from_service(err: anyhow::Error) -> Self {
        if err.downcast_ref::<OracleQueryError>().is_some() {
            Self::invalid_query(err.to_string())
        } else if err
            .to_string()
            .contains("already exists with a different request identity")
        {
            Self {
                code: "QUERY_ID_CONFLICT",
                message: err.to_string(),
                http_status: 409,
            }
        } else {
            Self {
                code: "INTERNAL_ERROR",
                message: err.to_string(),
                http_status: 500,
            }
        }
    }
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DispatchError {}

pub async fn dispatch(
    service: &OracleService,
    request: DispatchRequest,
) -> Result<Value, DispatchError> {
    match request {
        DispatchRequest::Status => service.status().await.map_err(DispatchError::from_service),
        DispatchRequest::Evaluate { request, id_prefix } => service
            .evaluate(request.into_query(id_prefix))
            .await
            .map(|verdict| {
                serde_json::to_value(verdict.redacted_for_export())
                    .expect("Verdict serialization is infallible")
            })
            .map_err(DispatchError::from_service),
        DispatchRequest::Verdicts { limit } => service
            .recent_verdicts(limit.min(100))
            .and_then(|verdicts| serde_json::to_value(verdicts).map_err(Into::into))
            .map_err(DispatchError::from_service),
        DispatchRequest::Paths => {
            serde_json::to_value(service.runtime_paths()).map_err(|err| DispatchError {
                code: "INTERNAL_ERROR",
                message: err.to_string(),
                http_status: 500,
            })
        }
        DispatchRequest::VerifyLedger => service
            .verify_ledger()
            .await
            .and_then(|report| serde_json::to_value(report).map_err(Into::into))
            .map_err(DispatchError::from_service),
        DispatchRequest::ExportLedger { destination } => {
            let destination = transport_export_destination(service, destination)?;
            service
                .export_verified_ledger(destination)
                .await
                .and_then(|report| serde_json::to_value(report).map_err(Into::into))
                .map_err(DispatchError::from_service)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalid_query_has_transport_independent_error_code() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let error = dispatch(
            &service,
            DispatchRequest::Evaluate {
                request: EvaluateRequest {
                    id: Some("invalid".to_string()),
                    task: "   ".to_string(),
                    requester: None,
                    context: None,
                    evidence: None,
                    query_type: None,
                    timestamp: None,
                    correlation_id: None,
                    causation_id: None,
                },
                id_prefix: "test",
            },
        )
        .await
        .expect_err("blank task must fail");

        assert_eq!(error.code, "INVALID_QUERY");
        assert_eq!(error.http_status, 422);
    }

    #[tokio::test]
    async fn export_dispatch_rejects_paths_outside_the_service_export_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");

        for destination in [
            PathBuf::from("/tmp/ledger.jsonl"),
            PathBuf::from("../ledger.jsonl"),
        ] {
            let error = dispatch(&service, DispatchRequest::ExportLedger { destination })
                .await
                .expect_err("transport exports must remain under the service export root");
            assert_eq!(error.code, "INVALID_REQUEST");
            assert_eq!(error.http_status, 400);
        }
    }
}
