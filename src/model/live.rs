//! OpenAI-compatible chat completions adapter (feature `live-model`).
//!
//! Provider HTTP shapes stay in this module. Core [`ModelAdapter`] / evidence types
//! remain provider-neutral. Secrets are read from the environment and never written
//! into run evidence.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde_json::{json, Value};

use super::adapter::{ModelAdapter, ModelError, ModelResponse};

/// Default OpenAI-compatible chat completions path (relative to base URL).
pub const DEFAULT_CHAT_PATH: &str = "/v1/chat/completions";

/// Configuration for [`OpenAiCompatibleAdapter`].
#[derive(Debug, Clone)]
pub struct LiveModelConfig {
    /// Base URL without trailing slash (e.g. `https://api.openai.com`).
    pub base_url: String,
    /// Model id sent to the provider.
    pub model: String,
    /// Bearer API key (never logged or sealed).
    pub api_key: String,
    /// Sampling temperature.
    pub temperature: f64,
    /// Max completion tokens.
    pub max_tokens: u32,
    /// HTTP timeout.
    pub timeout: Duration,
}

impl LiveModelConfig {
    /// Load from environment (fail-closed if key missing).
    ///
    /// - `VAA_MODEL_API_KEY` (required)
    /// - `VAA_MODEL_BASE_URL` (default `https://api.openai.com`)
    /// - `VAA_MODEL_NAME` (default `gpt-4o-mini`)
    pub fn from_env() -> Result<Self, ModelError> {
        let api_key = std::env::var("VAA_MODEL_API_KEY")
            .map_err(|_| ModelError::NotConfigured)?
            .trim()
            .to_owned();
        if api_key.is_empty() {
            return Err(ModelError::NotConfigured);
        }
        let base_url = std::env::var("VAA_MODEL_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".to_owned())
            .trim_end_matches('/')
            .to_owned();
        let model = std::env::var("VAA_MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_owned());
        Ok(Self {
            base_url,
            model,
            api_key,
            temperature: 0.1,
            max_tokens: 8000,
            timeout: Duration::from_secs(120),
        })
    }
}

/// One OpenAI-compatible chat endpoint behind [`ModelAdapter`].
pub struct OpenAiCompatibleAdapter {
    config: LiveModelConfig,
    generation_counter: AtomicU32,
}

impl OpenAiCompatibleAdapter {
    #[must_use]
    pub fn new(config: LiveModelConfig) -> Self {
        Self {
            config,
            generation_counter: AtomicU32::new(0),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}{DEFAULT_CHAT_PATH}", self.config.base_url)
    }
}

impl ModelAdapter for OpenAiCompatibleAdapter {
    fn name(&self) -> &str {
        &self.config.model
    }

    fn generate(
        &self,
        prompt: &str,
        task_id: &str,
        target: &str,
    ) -> Result<ModelResponse, ModelError> {
        let body = json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are an assembly code generator. Reply with ONLY NASM-syntax assembly for the requested callable function. No markdown fences unless required; no prose."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let agent = ureq::AgentBuilder::new()
            .timeout_read(self.config.timeout)
            .timeout_write(self.config.timeout)
            .build();

        let response = agent
            .post(&self.endpoint())
            .set("Authorization", &format!("Bearer {}", self.config.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(classify_ureq_error)?;

        let status = response.status();
        let text = response
            .into_string()
            .map_err(|e| ModelError::GenerationFailed(format!("read body: {e}")))?;

        if !(200..300).contains(&status) {
            return Err(ModelError::GenerationFailed(format!(
                "HTTP {status}: {}",
                truncate_for_diag(&text, 400)
            )));
        }

        let content = extract_chat_content(&text)?;
        let source = strip_code_fences(&content);
        if source.trim().is_empty() {
            return Err(ModelError::InvalidResponse(
                "empty assembly content from model".to_owned(),
            ));
        }

        let n = self.generation_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let gen_id = format!("live-{}-{n:04}", self.config.model);
        Ok(ModelResponse {
            source,
            target: target.to_owned(),
            model_name: self.config.model.clone(),
            generation_id: gen_id,
            diagnostics: vec![
                format!("task_id={task_id}"),
                "provider=openai-compatible".into(),
            ],
        })
    }
}

fn classify_ureq_error(err: ureq::Error) -> ModelError {
    match err {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            ModelError::GenerationFailed(format!("HTTP {code}: {}", truncate_for_diag(&body, 400)))
        }
        ureq::Error::Transport(t) => ModelError::GenerationFailed(format!("transport: {t}")),
    }
}

/// Parse OpenAI-compatible chat completion JSON → assistant message content.
pub fn extract_chat_content(body: &str) -> Result<String, ModelError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| ModelError::InvalidResponse(format!("json: {e}")))?;
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            // Some gateways return a content array of text parts.
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
        });
    match content {
        Some(c) if !c.trim().is_empty() => Ok(c),
        Some(_) => Err(ModelError::InvalidResponse(
            "empty message.content".to_owned(),
        )),
        None => Err(ModelError::InvalidResponse(
            "missing choices[0].message.content".to_owned(),
        )),
    }
}

/// Strip optional markdown ``` / ```asm fences from model output.
#[must_use]
pub fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_owned();
    }
    let mut lines = trimmed.lines();
    let _ = lines.next(); // opening fence
    let mut body: Vec<&str> = lines.collect();
    if body.last().is_some_and(|l| l.trim().starts_with("```")) {
        body.pop();
    }
    body.join("\n").trim().to_owned()
}

fn truncate_for_diag(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        return t.to_owned();
    }
    let cut: String = t.chars().take(max).collect();
    format!("{cut}…")
}

/// Build a deterministic user prompt from locked-task fields (no secrets).
#[must_use]
pub fn build_generation_prompt(
    task_id: &str,
    target: &str,
    symbol: &str,
    abi: &str,
    behavior_summary: &str,
) -> String {
    format!(
        "Generate NASM BITS 64 assembly for callable function `{symbol}`.\n\
         Task id: {task_id}\n\
         Target: {target}\n\
         ABI: {abi}\n\
         Behavior: {behavior_summary}\n\
         Requirements: export global `{symbol}`; no syscalls; no libc; return via rax; keep stack usage small.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_chat_content() {
        let body =
            r#"{"choices":[{"message":{"role":"assistant","content":"xor eax, eax\nret\n"}}]}"#;
        assert_eq!(
            extract_chat_content(body).unwrap().trim(),
            "xor eax, eax\nret"
        );
    }

    #[test]
    fn strips_asm_fences() {
        let raw = "```asm\nBITS 64\nret\n```\n";
        assert_eq!(strip_code_fences(raw), "BITS 64\nret");
    }

    #[test]
    fn from_env_requires_key() {
        // Ensure we do not inherit a key from the developer shell for this assert.
        let prev = std::env::var_os("VAA_MODEL_API_KEY");
        std::env::remove_var("VAA_MODEL_API_KEY");
        let err = LiveModelConfig::from_env().unwrap_err();
        assert!(matches!(err, ModelError::NotConfigured));
        if let Some(v) = prev {
            std::env::set_var("VAA_MODEL_API_KEY", v);
        }
    }

    #[test]
    fn rejects_missing_choices() {
        let err = extract_chat_content(r#"{"choices":[]}"#).unwrap_err();
        assert!(matches!(err, ModelError::InvalidResponse(_)));
    }

    #[test]
    fn prompt_contains_task_fields_not_secrets() {
        let p = build_generation_prompt(
            "count-byte-linux-v1",
            "x86_64-unknown-linux-gnu",
            "count_byte",
            "sysv64",
            "count needle",
        );
        assert!(p.contains("count_byte"));
        assert!(!p.to_lowercase().contains("api_key"));
    }
}
