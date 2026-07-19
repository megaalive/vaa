use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub struct FixtureModelAdapter {
    pub name: String,
    pub responses: HashMap<String, String>,
    pub error_responses: HashMap<String, String>,
    pub generation_counter: u32,
}

impl FixtureModelAdapter {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            responses: HashMap::new(),
            error_responses: HashMap::new(),
            generation_counter: 0,
        }
    }

    pub fn add_response(&mut self, key: &str, source: &str) {
        self.responses.insert(key.to_owned(), source.to_owned());
    }

    pub fn add_error(&mut self, key: &str, error: &str) {
        self.error_responses
            .insert(key.to_owned(), error.to_owned());
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
        let source = self
            .responses
            .get(&key)
            .or_else(|| self.responses.get(prompt));

        match source {
            Some(src) => {
                let gen_id = format!("{}-{:04}", self.name, self.generation_counter + 1);
                Ok(ModelResponse {
                    source: src.clone(),
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
            responses: self.responses.clone(),
            error_responses: self.error_responses.clone(),
            generation_counter: 0,
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
