//! Fixture and trait model adapters.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub source: String,
    pub target: String,
    pub model_name: String,
    pub generation_id: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("model adapter not configured")]
    NotConfigured,
    #[error("generation failed: {0}")]
    GenerationFailed(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

pub trait ModelAdapter {
    fn name(&self) -> &str;
    fn generate(
        &self,
        prompt: &str,
        task_id: &str,
        target: &str,
    ) -> Result<ModelResponse, ModelError>;
}

/// Deterministic fixture adapter with a per-key response queue (wrong → repair).
pub struct FixtureModelAdapter {
    name: String,
    responses: Mutex<HashMap<String, VecDeque<String>>>,
    error_responses: HashMap<String, String>,
    generation_counter: AtomicU32,
}

impl FixtureModelAdapter {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            responses: Mutex::new(HashMap::new()),
            error_responses: HashMap::new(),
            generation_counter: AtomicU32::new(0),
        }
    }

    /// Enqueue one source response for `{task_id}::{target}` (or a free-form key).
    pub fn add_response(&mut self, key: &str, source: &str) {
        self.responses
            .lock()
            .expect("fixture responses lock")
            .entry(key.to_owned())
            .or_default()
            .push_back(source.to_owned());
    }

    pub fn add_error(&mut self, key: &str, error: &str) {
        self.error_responses
            .insert(key.to_owned(), error.to_owned());
    }

    /// Remaining queued responses for a key.
    #[must_use]
    pub fn pending_count(&self, key: &str) -> usize {
        self.responses
            .lock()
            .expect("fixture responses lock")
            .get(key)
            .map_or(0, VecDeque::len)
    }
}

impl ModelAdapter for FixtureModelAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        prompt: &str,
        task_id: &str,
        target: &str,
    ) -> Result<ModelResponse, ModelError> {
        let key = format!("{task_id}::{target}");
        let mut guard = self.responses.lock().expect("fixture responses lock");

        let source = guard
            .get_mut(&key)
            .and_then(VecDeque::pop_front)
            .or_else(|| guard.get_mut(prompt).and_then(VecDeque::pop_front));

        match source {
            Some(src) => {
                let n = self.generation_counter.fetch_add(1, Ordering::Relaxed) + 1;
                let gen_id = format!("{}-{n:04}", self.name);
                Ok(ModelResponse {
                    source: src,
                    target: target.to_owned(),
                    model_name: self.name.clone(),
                    generation_id: gen_id,
                    diagnostics: Vec::new(),
                })
            }
            None => {
                if let Some(err_msg) = self
                    .error_responses
                    .get(prompt)
                    .or_else(|| self.error_responses.get(&key))
                {
                    Err(ModelError::GenerationFailed(err_msg.clone()))
                } else {
                    Err(ModelError::NotConfigured)
                }
            }
        }
    }
}

impl Clone for FixtureModelAdapter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            responses: Mutex::new(
                self.responses
                    .lock()
                    .expect("fixture responses lock")
                    .clone(),
            ),
            error_responses: self.error_responses.clone(),
            generation_counter: AtomicU32::new(self.generation_counter.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_adapter_returns_configured_response() {
        let mut adapter = FixtureModelAdapter::new("test-model");
        adapter.add_response("sum_i64::x86_64-unknown-linux-gnu", "section .text");
        let resp = adapter.generate("prompt", "sum_i64", "x86_64-unknown-linux-gnu");
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap().source, "section .text");
    }

    #[test]
    fn fixture_queue_advances_and_bumps_generation_id() {
        let mut adapter = FixtureModelAdapter::new("fixture");
        let key = "t::target";
        adapter.add_response(key, "wrong");
        adapter.add_response(key, "repaired");
        let first = adapter.generate("", "t", "target").expect("first");
        let second = adapter.generate("", "t", "target").expect("second");
        assert_eq!(first.source, "wrong");
        assert_eq!(second.source, "repaired");
        assert_eq!(first.generation_id, "fixture-0001");
        assert_eq!(second.generation_id, "fixture-0002");
        assert!(adapter.generate("", "t", "target").is_err());
    }

    #[test]
    fn fixture_adapter_returns_error_for_configured_error() {
        let mut adapter = FixtureModelAdapter::new("test-model");
        adapter.add_error("fail", "assembler error");
        let resp = adapter.generate("fail", "any", "x86_64");
        assert!(matches!(resp, Err(ModelError::GenerationFailed(_))));
    }

    #[test]
    fn fixture_adapter_unconfigured_returns_not_configured() {
        let adapter = FixtureModelAdapter::new("test-model");
        let resp = adapter.generate("anything", "unknown", "x86_64");
        assert!(matches!(resp, Err(ModelError::NotConfigured)));
    }
}
