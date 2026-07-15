//! OpenAI-compatible vision-LLM client (Qwen2.5-VL via llama-server).
//!
//! Sends a target reference image plus 1+ candidate renders to a vision-capable
//! chat-completions endpoint, asks for a structured comparison, parses the
//! response into a `ComparisonReport`.
//!
//! Default endpoint: `http://arda-server:8081` — override via
//! `FORGE_VISION_ADDR`. Default model alias: `qwen2.5-vl-7b-instruct` (override
//! `FORGE_VISION_MODEL`). Per-call timeout: `FORGE_VISION_TIMEOUT_SECS` (def 240).

use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_VISION_ADDR: &str = "http://arda-server:8081";
pub const DEFAULT_VISION_MODEL: &str = "qwen2.5-vl-7b-instruct";
pub const DEFAULT_VISION_TIMEOUT_SECS: u64 = 240;

#[derive(Debug, Clone)]
pub struct VisionClient {
    base_url: String,
    model: String,
    pub timeout: Duration,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// 0.0–1.0; 1.0 = perceptually identical to the reference.
    pub match_score: f64,
    /// Short list of elements present in the reference but absent from candidate.
    #[serde(default)]
    pub missing: Vec<String>,
    /// Short list of elements in candidate that diverge from reference.
    #[serde(default)]
    pub wrong: Vec<String>,
    /// Things the candidate gets right.
    #[serde(default)]
    pub strengths: Vec<String>,
    /// A complete improved prompt the next iteration should use.
    #[serde(default)]
    pub suggested_prompt_edit: String,
    /// Raw model response, useful for debugging.
    #[serde(default)]
    pub raw_response: String,
}

impl Default for VisionClient {
    fn default() -> Self {
        Self::new(DEFAULT_VISION_ADDR, DEFAULT_VISION_MODEL)
    }
}

impl VisionClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            timeout: Duration::from_secs(DEFAULT_VISION_TIMEOUT_SECS),
            http: reqwest::Client::builder()
                .pool_idle_timeout(Some(Duration::from_secs(30)))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn from_env() -> Self {
        let addr =
            std::env::var("FORGE_VISION_ADDR").unwrap_or_else(|_| DEFAULT_VISION_ADDR.to_string());
        let model = std::env::var("FORGE_VISION_MODEL")
            .unwrap_or_else(|_| DEFAULT_VISION_MODEL.to_string());
        let timeout = std::env::var("FORGE_VISION_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_VISION_TIMEOUT_SECS);
        let mut client = Self::new(addr, model);
        client.timeout = Duration::from_secs(timeout);
        client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Compare a target reference image against one or more candidate renders.
    /// `iteration` is informational (included in the prompt).
    pub async fn compare(
        &self,
        target: &Path,
        candidates: &[&Path],
        current_prompt: &str,
        iteration: u32,
    ) -> anyhow::Result<ComparisonReport> {
        if candidates.is_empty() {
            anyhow::bail!("vision.compare needs at least one candidate image");
        }

        let mut content: Vec<Value> = Vec::with_capacity(2 + candidates.len() * 2 + 1);
        content.push(serde_json::json!({"type": "text", "text": "TARGET reference image (this is what the generated asset should resemble):"}));
        content.push(image_part(target)?);
        for (i, cand) in candidates.iter().enumerate() {
            content.push(serde_json::json!({
                "type": "text",
                "text": format!("CANDIDATE view {} of {} (iteration {}, generated from current prompt):", i + 1, candidates.len(), iteration)
            }));
            content.push(image_part(cand)?);
        }
        content.push(serde_json::json!({
            "type": "text",
            "text": instruction_text(current_prompt)
        }));

        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0.2,
            "max_tokens": 900,
            "messages": [{"role": "user", "content": content}],
        });

        let url = format!("{}/v1/chat/completions", self.base_url);
        let send_fut = self.http.post(&url).json(&body).send();
        let resp = tokio::time::timeout(self.timeout, send_fut)
            .await
            .map_err(|_| {
                anyhow::anyhow!("vision LLM request timed out after {:?}", self.timeout)
            })??;
        let resp = resp.error_for_status()?;
        let value: Value = resp.json().await?;
        let text = value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("vision response missing choices[0].message.content"))?
            .to_string();
        parse_report(&text)
    }
}

fn image_part(path: &Path) -> anyhow::Result<Value> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read image {}: {e}", path.display()))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mime = match path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "image/png",
    };
    Ok(serde_json::json!({
        "type": "image_url",
        "image_url": {"url": format!("data:{mime};base64,{b64}")}
    }))
}

fn instruction_text(current_prompt: &str) -> String {
    format!(
        "You are a 3D asset reviewer. Compare the CANDIDATE views (one or more angles of a single 3D asset) against the TARGET reference. The candidate was generated from this current text prompt:\n\n\"{current_prompt}\"\n\n\
         Respond with STRICT JSON ONLY (no prose outside the object, no code fences):\n\
         {{\n\
           \"match_score\": <float 0..1, where 1.0 = visually indistinguishable from target, 0.0 = nothing in common>,\n\
           \"missing\": [<short strings: things in the target that are absent from the candidate>],\n\
           \"wrong\": [<short strings: things in the candidate that diverge from the target>],\n\
           \"strengths\": [<short strings: aspects of the candidate that match the target well>],\n\
           \"suggested_prompt_edit\": \"<a single complete improved prompt for the next iteration, designed to fix missing/wrong items; must include 'isolated on plain white background, centered product render, 3D model, single object' and a negative-style 'no humans, no environment'>\"\n\
         }}"
    )
}

fn parse_report(raw: &str) -> anyhow::Result<ComparisonReport> {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = trimmed
        .find('{')
        .ok_or_else(|| anyhow::anyhow!("no JSON object in vision response: {raw}"))?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| anyhow::anyhow!("no JSON object end in vision response: {raw}"))?;
    let json_str = &trimmed[start..=end];
    let v: Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("vision JSON parse failed: {e}; body: {json_str}"))?;
    let report = ComparisonReport {
        match_score: v
            .get("match_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0),
        missing: extract_strings(&v, "missing"),
        wrong: extract_strings(&v, "wrong"),
        strengths: extract_strings(&v, "strengths"),
        suggested_prompt_edit: v
            .get("suggested_prompt_edit")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        raw_response: raw.to_string(),
    };
    Ok(report)
}

fn extract_strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
