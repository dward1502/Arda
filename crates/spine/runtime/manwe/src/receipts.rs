use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct RouteReceipt {
    pub ts_utc: String,
    pub provider_id: String,
    pub model_id: String,
    pub resource_group: String,
    pub task_type: String,
    pub routing_mode: String,
    pub streaming: bool,
    pub status_code: u16,
    pub latency_ms: u64,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub generation_tokens_per_second: Option<f64>,
    pub finish_reason: Option<String>,
    pub has_answer_content: Option<bool>,
    pub reasoning_only: Option<bool>,
    pub expected_exact: Option<String>,
    pub exact_match: Option<bool>,
    pub quality_score: Option<f64>,
    pub benchmark: Option<TaskClassBenchmarkReceipt>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QualityExpectation {
    pub exact: String,
    pub benchmark_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskClassBenchmarkReceipt {
    pub schema_version: &'static str,
    pub benchmark_id: String,
    pub task_class: String,
    pub evaluator: &'static str,
    pub passed: bool,
    pub score: f64,
}

pub fn task_class_benchmark_receipt(
    task_type: &str,
    benchmark_id: &str,
    expected_exact: &str,
    body: Option<&serde_json::Value>,
) -> Option<TaskClassBenchmarkReceipt> {
    let valid_id = !benchmark_id.is_empty()
        && benchmark_id.len() <= 64
        && benchmark_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid_id || expected_exact.len() > 4096 {
        return None;
    }

    let task_class = match task_type {
        "chat" | "code" | "vision" | "reasoning" | "summary" | "tool_use" | "structured_output" => {
            task_type
        }
        _ => "other",
    };
    let actual = body
        .and_then(|value| value.pointer("/choices/0/message/content"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let passed = actual.is_some_and(|value| value == expected_exact.trim());

    Some(TaskClassBenchmarkReceipt {
        schema_version: "arda.manwe.task_benchmark.v1",
        benchmark_id: benchmark_id.to_string(),
        task_class: task_class.to_string(),
        evaluator: "exact_match",
        passed,
        score: if passed { 1.0 } else { 0.0 },
    })
}

#[derive(Debug, Clone)]
pub struct ReceiptWriter {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl ReceiptWriter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn append(&self, receipt: &RouteReceipt) -> std::io::Result<()> {
        let _guard = self.lock.lock().await;
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        let mut encoded = serde_json::to_vec(receipt).map_err(std::io::Error::other)?;
        encoded.push(b'\n');
        file.write_all(&encoded).await?;
        file.flush().await
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "route receipts keep each externally observed response field explicit"
)]
pub fn receipt_from_response(
    provider_id: String,
    model_id: String,
    resource_group: String,
    task_type: String,
    routing_mode: String,
    streaming: bool,
    status_code: u16,
    latency_ms: u64,
    quality_expectation: Option<QualityExpectation>,
    body: Option<&serde_json::Value>,
    error: Option<String>,
) -> RouteReceipt {
    let expected_exact = quality_expectation
        .as_ref()
        .map(|expectation| expectation.exact.clone());
    let prompt_tokens = body
        .and_then(|value| value.pointer("/usage/prompt_tokens"))
        .and_then(serde_json::Value::as_u64);
    let completion_tokens = body
        .and_then(|value| value.pointer("/usage/completion_tokens"))
        .and_then(serde_json::Value::as_u64);
    let generation_tokens_per_second = body
        .and_then(|value| value.pointer("/timings/predicted_per_second"))
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            completion_tokens
                .filter(|_| latency_ms > 0)
                .map(|tokens| tokens as f64 / (latency_ms as f64 / 1000.0))
        });
    let finish_reason = body
        .and_then(|value| value.pointer("/choices/0/finish_reason"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let answer = body
        .and_then(|value| value.pointer("/choices/0/message/content"))
        .and_then(serde_json::Value::as_str);
    let reasoning = body
        .and_then(|value| value.pointer("/choices/0/message/reasoning_content"))
        .and_then(serde_json::Value::as_str);
    let has_answer_content = body.map(|_| answer.is_some_and(|value| !value.trim().is_empty()));
    let reasoning_only = body.map(|_| {
        answer.is_none_or(|value| value.trim().is_empty())
            && reasoning.is_some_and(|value| !value.trim().is_empty())
    });
    let exact_match = expected_exact.as_deref().map(|expected| {
        answer
            .map(str::trim)
            .is_some_and(|actual| actual == expected.trim())
    });
    let quality_score = body.map(|_| {
        if exact_match == Some(false) {
            0.0
        } else if finish_reason.as_deref() == Some("length") {
            0.25
        } else if has_answer_content == Some(true) {
            1.0
        } else if reasoning_only == Some(true) {
            0.25
        } else {
            0.0
        }
    });
    let benchmark = quality_expectation
        .as_ref()
        .and_then(|expectation| {
            expectation
                .benchmark_id
                .as_deref()
                .map(|benchmark_id| (expectation, benchmark_id))
        })
        .and_then(|(expectation, benchmark_id)| {
            task_class_benchmark_receipt(&task_type, benchmark_id, &expectation.exact, body)
        });

    RouteReceipt {
        ts_utc: chrono::Utc::now().to_rfc3339(),
        provider_id,
        model_id,
        resource_group,
        task_type,
        routing_mode,
        streaming,
        status_code,
        latency_ms,
        prompt_tokens,
        completion_tokens,
        generation_tokens_per_second,
        finish_reason,
        has_answer_content,
        reasoning_only,
        expected_exact,
        exact_match,
        quality_score,
        benchmark,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_throughput_and_failed_exact_quality() {
        let body = serde_json::json!({
            "choices": [{
                "finish_reason": "length",
                "message": {"content": "", "reasoning_content": "thinking"}
            }],
            "usage": {"prompt_tokens": 21, "completion_tokens": 64},
            "timings": {"predicted_per_second": 23.2}
        });
        let receipt = receipt_from_response(
            "provider".into(),
            "model".into(),
            "gpu".into(),
            "reasoning".into(),
            "adaptive".into(),
            false,
            200,
            3000,
            Some(QualityExpectation {
                exact: "MANWE_OK".into(),
                benchmark_id: None,
            }),
            Some(&body),
            None,
        );
        assert_eq!(receipt.generation_tokens_per_second, Some(23.2));
        assert_eq!(receipt.reasoning_only, Some(true));
        assert_eq!(receipt.exact_match, Some(false));
        assert_eq!(receipt.quality_score, Some(0.0));
    }

    #[test]
    fn emits_deterministic_task_class_benchmark_receipt() {
        let body = serde_json::json!({
            "choices": [{"message": {"content": "fn answer() {}"}}]
        });

        let receipt =
            task_class_benchmark_receipt("code", "rust-smoke-01", "fn answer() {}", Some(&body))
                .expect("bounded benchmark receipt");

        assert_eq!(receipt.schema_version, "arda.manwe.task_benchmark.v1");
        assert_eq!(receipt.benchmark_id, "rust-smoke-01");
        assert_eq!(receipt.task_class, "code");
        assert_eq!(receipt.evaluator, "exact_match");
        assert!(receipt.passed);
        assert_eq!(receipt.score, 1.0);
    }

    #[test]
    fn rejects_unbounded_benchmark_identifiers() {
        let body = serde_json::json!({
            "choices": [{"message": {"content": "ok"}}]
        });

        assert!(
            task_class_benchmark_receipt("chat", &"x".repeat(65), "ok", Some(&body),).is_none()
        );
        assert!(
            task_class_benchmark_receipt("chat", "contains spaces", "ok", Some(&body)).is_none()
        );
    }
}
