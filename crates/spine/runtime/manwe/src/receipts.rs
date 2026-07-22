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
    pub error: Option<String>,
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

pub fn receipt_from_response(
    provider_id: String,
    model_id: String,
    resource_group: String,
    task_type: String,
    routing_mode: String,
    streaming: bool,
    status_code: u16,
    latency_ms: u64,
    expected_exact: Option<String>,
    body: Option<&serde_json::Value>,
    error: Option<String>,
) -> RouteReceipt {
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
        !answer.is_some_and(|value| !value.trim().is_empty())
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
            Some("MANWE_OK".into()),
            Some(&body),
            None,
        );
        assert_eq!(receipt.generation_tokens_per_second, Some(23.2));
        assert_eq!(receipt.reasoning_only, Some(true));
        assert_eq!(receipt.exact_match, Some(false));
        assert_eq!(receipt.quality_score, Some(0.0));
    }
}
